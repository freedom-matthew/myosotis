use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use myosotis_core::{LibraryCard, RepoError};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::features::library;
use crate::storage::SqliteCardRepo;

const BACKUPS_KEPT: usize = 14;
const NAME_TIME_FORMAT: &str = "%Y-%m-%d-%H%M%S";
/// A backup waiting to replace the collection on the next start.
const STAGED_RESTORE: &str = "restore-staged.db";

/// A backup I can go back to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Backup {
    pub name: String,
    pub taken_at: DateTime<Utc>,
    pub card_count: Option<i64>,
}

fn file_error(error: impl Error + Send + Sync + 'static) -> RepoError {
    RepoError(Box::new(error))
}

/// `~/Documents/Myosotis`: a folder the user can see and copy.
pub fn folder(app: &AppHandle) -> Result<PathBuf, tauri::Error> {
    Ok(app.path().document_dir()?.join("Myosotis"))
}

fn backup_name(now: DateTime<Utc>) -> String {
    format!(
        "myosotis-{}.db",
        now.with_timezone(&Local).format(NAME_TIME_FORMAT)
    )
}

/// When a backup was taken, read from its name, or from the file if renamed.
fn taken_at(path: &Path) -> DateTime<Utc> {
    let stem: String = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let from_name: Option<DateTime<Utc>> = stem
        .strip_prefix("myosotis-")
        .and_then(|time| NaiveDateTime::parse_from_str(time, NAME_TIME_FORMAT).ok())
        .and_then(|time| Local.from_local_datetime(&time).earliest())
        .map(|time| time.with_timezone(&Utc));
    from_name.unwrap_or_else(|| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .map(DateTime::<Utc>::from)
            .unwrap_or_default()
    })
}

fn day_prefix(now: DateTime<Utc>) -> String {
    format!("myosotis-{}", now.with_timezone(&Local).format("%Y-%m-%d"))
}

fn backups_in(folder: &Path) -> Result<Vec<PathBuf>, RepoError> {
    let mut backups: Vec<PathBuf> = fs::read_dir(folder)
        .map_err(file_error)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "db"))
        .collect();
    // The date in the name makes name order the same as age order.
    backups.sort();
    Ok(backups)
}

/// Writes a backup and keeps only the most recent ones.
pub async fn back_up(
    repo: &SqliteCardRepo,
    folder: &Path,
    now: DateTime<Utc>,
) -> Result<PathBuf, RepoError> {
    fs::create_dir_all(folder).map_err(file_error)?;
    let path: PathBuf = folder.join(backup_name(now));
    repo.back_up_to(&path).await?;

    let backups: Vec<PathBuf> = backups_in(folder)?;
    let surplus: usize = backups.len().saturating_sub(BACKUPS_KEPT);
    for old in &backups[..surplus] {
        fs::remove_file(old).map_err(file_error)?;
    }
    Ok(path)
}

/// Backs up once per day, the first time the app starts that day.
pub async fn daily_backup(
    repo: &SqliteCardRepo,
    folder: &Path,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    fs::create_dir_all(folder).map_err(file_error)?;
    let today: String = day_prefix(now);
    let done_today: bool = backups_in(folder)?.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(&today))
    });
    if !done_today {
        back_up(repo, folder, now).await?;
    }
    Ok(())
}

/// The backups in the folder, newest first, with how many cards each holds.
pub async fn list(folder: &Path) -> Result<Vec<Backup>, RepoError> {
    if !folder.exists() {
        return Ok(Vec::new());
    }
    let mut backups: Vec<Backup> = Vec::new();
    for path in backups_in(folder)?.into_iter().rev() {
        backups.push(Backup {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            taken_at: taken_at(&path),
            card_count: SqliteCardRepo::count_cards_in(&path).await,
        });
    }
    Ok(backups)
}

/// Prepares going back to a backup. The current collection is backed up first, so the
/// restore can itself be undone; the swap happens on the next start (`apply_staged_restore`),
/// while nothing has the collection open.
pub async fn stage_restore(
    repo: &SqliteCardRepo,
    backups: &Path,
    name: &str,
    data_dir: &Path,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let chosen: PathBuf = backups_in(backups)?
        .into_iter()
        .find(|path| path.file_name().is_some_and(|file| file == name))
        .ok_or_else(|| {
            file_error(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "That backup no longer exists.",
            ))
        })?;
    // Copied before backing up, because making room for the new backup may remove it.
    fs::copy(&chosen, data_dir.join(STAGED_RESTORE)).map_err(file_error)?;
    back_up(repo, backups, now).await?;
    Ok(())
}

/// Swaps a staged backup in for the collection. Runs at startup, before it is opened.
pub fn apply_staged_restore(data_dir: &Path, database: &Path) -> Result<bool, RepoError> {
    let staged: PathBuf = data_dir.join(STAGED_RESTORE);
    if !staged.exists() {
        return Ok(false);
    }
    // Leftover journal files belong to the old collection and would corrupt the new one.
    for suffix in ["-wal", "-shm"] {
        let journal: PathBuf = PathBuf::from(format!("{}{suffix}", database.display()));
        if journal.exists() {
            fs::remove_file(&journal).map_err(file_error)?;
        }
    }
    fs::rename(&staged, database).map_err(file_error)?;
    Ok(true)
}

/// Writes every card to a readable JSON file.
pub async fn export(
    repo: &SqliteCardRepo,
    folder: &Path,
    now: DateTime<Utc>,
) -> Result<PathBuf, RepoError> {
    fs::create_dir_all(folder).map_err(file_error)?;
    let cards: Vec<LibraryCard> = library::list(repo, now).await?;
    let json: String = serde_json::to_string_pretty(&cards).map_err(file_error)?;
    let name: String = format!(
        "myosotis-cards-{}.json",
        now.with_timezone(&Local).format("%Y-%m-%d-%H%M%S")
    );
    let path: PathBuf = folder.join(name);
    fs::write(&path, json).map_err(file_error)?;
    Ok(path)
}

#[tauri::command]
pub async fn back_up_now(
    app: AppHandle,
    repo: State<'_, SqliteCardRepo>,
) -> Result<String, String> {
    let backups: PathBuf = folder(&app)
        .map_err(|error| error.to_string())?
        .join("backups");
    back_up(repo.inner(), &backups, Utc::now())
        .await
        .map(|path| path.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn export_cards(
    app: AppHandle,
    repo: State<'_, SqliteCardRepo>,
) -> Result<String, String> {
    let target: PathBuf = folder(&app).map_err(|error| error.to_string())?;
    export(repo.inner(), &target, Utc::now())
        .await
        .map(|path| path.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn data_folder(app: AppHandle) -> Result<String, String> {
    folder(&app)
        .map(|path| path.display().to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn list_backups(app: AppHandle) -> Result<Vec<Backup>, String> {
    let backups: PathBuf = folder(&app)
        .map_err(|error| error.to_string())?
        .join("backups");
    list(&backups).await.map_err(|error| error.to_string())
}

/// Stages the restore and restarts, so the backup is swapped in on the way up.
#[tauri::command]
pub async fn restore_backup(
    app: AppHandle,
    repo: State<'_, SqliteCardRepo>,
    name: String,
) -> Result<(), String> {
    let backups: PathBuf = folder(&app)
        .map_err(|error| error.to_string())?
        .join("backups");
    let data_dir: PathBuf = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    stage_restore(repo.inner(), &backups, &name, &data_dir, Utc::now())
        .await
        .map_err(|error| error.to_string())?;
    app.restart()
}
