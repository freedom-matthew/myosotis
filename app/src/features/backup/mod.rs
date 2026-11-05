use std::error::Error;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use myosotis_core::{LibraryCard, RepoError};
use tauri::{AppHandle, Manager, State};

use crate::features::library;
use crate::storage::SqliteCardRepo;

const BACKUPS_KEPT: usize = 14;

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
        now.with_timezone(&Local).format("%Y-%m-%d-%H%M%S")
    )
}

fn day_prefix(now: DateTime<Utc>) -> String {
    format!("myosotis-{}", now.with_timezone(&Local).format("%Y-%m-%d"))
}

fn backups_in(folder: &Path) -> Result<Vec<PathBuf>, RepoError> {
    let mut backups: Vec<PathBuf> = fs::read_dir(folder)
        .map_err(file_error)?
        .filter_map(Result::ok)
        .map(|entry: DirEntry| entry.path())
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
