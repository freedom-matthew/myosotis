mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use common::*;
use myosotis_app::features::backup;
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{Card, CardRepo};

static NEXT_FOLDER: AtomicU32 = AtomicU32::new(0);

fn empty_folder() -> PathBuf {
    let number: u32 = NEXT_FOLDER.fetch_add(1, Ordering::Relaxed);
    let nanos: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("myosotis-test-{nanos}-{number}"))
}

/// Backups run on the real database file, so these tests use one too.
async fn collection_on_disk(cards: usize) -> (SqliteCardRepo, PathBuf) {
    let folder: PathBuf = empty_folder();
    fs::create_dir_all(&folder).unwrap();
    let repo: SqliteCardRepo = SqliteCardRepo::open(&folder.join("collection.sqlite"))
        .await
        .unwrap();
    for number in 1..=cards {
        let card: Card = Card::new(format!("Question {number}"), format!("Answer {number}"));
        repo.insert(&card).await.unwrap();
    }
    (repo, folder)
}

fn files_in(folder: &PathBuf) -> usize {
    fs::read_dir(folder).unwrap().count()
}

#[tokio::test]
async fn a_backup_can_be_opened_and_contains_my_cards() {
    // given a collection with two cards
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(2).await;
    let folder: PathBuf = home.join("backups");

    // when I back up
    let path: PathBuf = backup::back_up(&repo, &folder, monday()).await.unwrap();

    // then the backup is a working collection with the same cards
    let restored: SqliteCardRepo = SqliteCardRepo::open(&path).await.unwrap();
    let cards: Vec<Card> = restored.list().await.unwrap();
    assert_eq!(cards.len(), 2);
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn the_daily_backup_runs_once_a_day() {
    // given a collection
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(1).await;
    let folder: PathBuf = home.join("backups");

    // when the app starts twice on one day, then again the next day
    backup::daily_backup(&repo, &folder, monday())
        .await
        .unwrap();
    backup::daily_backup(&repo, &folder, minutes_later(monday(), 60))
        .await
        .unwrap();
    assert_eq!(files_in(&folder), 1);
    backup::daily_backup(&repo, &folder, days_later(monday(), 1))
        .await
        .unwrap();

    // then there is one backup per day
    assert_eq!(files_in(&folder), 2);
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn only_the_fourteen_most_recent_backups_are_kept() {
    // given twenty days of backups
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(1).await;
    let folder: PathBuf = home.join("backups");
    for day in 0..20 {
        backup::back_up(&repo, &folder, days_later(monday(), day))
            .await
            .unwrap();
    }

    // then only the last fourteen remain
    assert_eq!(files_in(&folder), 14);
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn cards_can_be_exported_to_a_readable_file() {
    // given a collection with one card
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let folder: PathBuf = empty_folder();

    // when I export
    let path: PathBuf = backup::export(&repo, &folder, monday()).await.unwrap();

    // then the file contains the card's text
    let content: String = fs::read_to_string(&path).unwrap();
    assert!(content.contains("Question 1"));
    assert!(content.contains("Answer 1"));
    fs::remove_dir_all(&folder).unwrap();
}

/// Restoring as the app does it: stage, restart, open the collection again.
async fn restore_and_restart(
    repo: &SqliteCardRepo,
    home: &Path,
    backup: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> SqliteCardRepo {
    let database: PathBuf = home.join("collection.sqlite");
    backup::stage_restore(repo, &home.join("backups"), backup, home, now)
        .await
        .unwrap();
    backup::apply_staged_restore(home, &database).unwrap();
    SqliteCardRepo::open(&database).await.unwrap()
}

async fn add_a_card(repo: &SqliteCardRepo) {
    repo.insert(&Card::new("Added later", "Answer"))
        .await
        .unwrap();
}

fn name_of(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

#[tokio::test]
async fn backups_are_listed_newest_first_with_how_many_cards_they_hold() {
    // given a backup with one card, and a later one with two
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(1).await;
    let folder: PathBuf = home.join("backups");
    backup::back_up(&repo, &folder, monday()).await.unwrap();
    add_a_card(&repo).await;
    backup::back_up(&repo, &folder, days_later(monday(), 1))
        .await
        .unwrap();

    // when I look at my backups
    let backups: Vec<backup::Backup> = backup::list(&folder).await.unwrap();

    // then the newest comes first, and each says how many cards it holds
    assert_eq!(backups.len(), 2);
    assert_eq!(backups[0].taken_at, days_later(monday(), 1));
    assert_eq!(backups[0].card_count, Some(2));
    assert_eq!(backups[1].card_count, Some(1));
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn restoring_a_backup_brings_back_the_collection_it_holds() {
    // given a backup with two cards, and a third card added since
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(2).await;
    let path: PathBuf = backup::back_up(&repo, &home.join("backups"), monday())
        .await
        .unwrap();
    add_a_card(&repo).await;

    // when I restore that backup
    let restored: SqliteCardRepo =
        restore_and_restart(&repo, &home, &name_of(&path), days_later(monday(), 1)).await;

    // then my collection is as it was in the backup
    let cards: Vec<Card> = restored.list().await.unwrap();
    assert_eq!(cards.len(), 2);
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn my_collection_is_backed_up_before_a_restore_replaces_it() {
    // given a backup with two cards, and a third card added since
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(2).await;
    let path: PathBuf = backup::back_up(&repo, &home.join("backups"), monday())
        .await
        .unwrap();
    add_a_card(&repo).await;

    // when I restore that backup
    restore_and_restart(&repo, &home, &name_of(&path), days_later(monday(), 1)).await;

    // then the newest backup holds the three cards I had, so I can go back
    let backups: Vec<backup::Backup> = backup::list(&home.join("backups")).await.unwrap();
    assert_eq!(backups[0].card_count, Some(3));
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn the_oldest_backup_can_be_restored_when_the_folder_is_full() {
    // given fourteen backups, the oldest from when I had one card
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(1).await;
    let folder: PathBuf = home.join("backups");
    let oldest: PathBuf = backup::back_up(&repo, &folder, monday()).await.unwrap();
    add_a_card(&repo).await;
    for day in 1..14 {
        backup::back_up(&repo, &folder, days_later(monday(), day))
            .await
            .unwrap();
    }

    // when I restore the oldest, though making room for the safety backup removes it
    let restored: SqliteCardRepo =
        restore_and_restart(&repo, &home, &name_of(&oldest), days_later(monday(), 20)).await;

    // then I get its one card back
    assert_eq!(restored.list().await.unwrap().len(), 1);
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn a_backup_that_is_not_in_the_folder_cannot_be_restored() {
    // given a collection with a backup
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(1).await;
    let folder: PathBuf = home.join("backups");
    backup::back_up(&repo, &folder, monday()).await.unwrap();

    // when I ask for a file that isn't one of my backups
    let result: Result<(), myosotis_core::RepoError> =
        backup::stage_restore(&repo, &folder, "../collection.sqlite", &home, monday()).await;

    // then it is refused, and the next start keeps my collection
    assert!(result.is_err());
    let database: PathBuf = home.join("collection.sqlite");
    assert!(!backup::apply_staged_restore(&home, &database).unwrap());
    fs::remove_dir_all(&home).unwrap();
}

/// A schema version from before an update, as an older app left the collection.
const AN_OLDER_SCHEMA: i64 = 20261111000000;

#[tokio::test]
async fn my_collection_is_backed_up_before_an_update_changes_it() {
    // given a collection left by an older version of the app
    let home: PathBuf = empty_folder();
    fs::create_dir_all(&home).unwrap();
    let database: PathBuf = home.join("collection.sqlite");
    let older: SqliteCardRepo = SqliteCardRepo::open_at_version(&database, AN_OLDER_SCHEMA)
        .await
        .unwrap();
    drop(older);

    // when the new version starts
    let backups: PathBuf = home.join("backups");
    let repo: SqliteCardRepo = backup::open_collection(&database, &backups, monday())
        .await
        .unwrap();

    // then a backup was made before the update, and the collection works
    assert_eq!(files_in(&backups), 1);
    assert!(repo.list().await.unwrap().is_empty());
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn starting_without_an_update_makes_no_extra_backup() {
    // given a collection that is already up to date
    let (repo, home): (SqliteCardRepo, PathBuf) = collection_on_disk(1).await;
    drop(repo);

    // when the app starts again
    let backups: PathBuf = home.join("backups");
    backup::open_collection(&home.join("collection.sqlite"), &backups, monday())
        .await
        .unwrap();

    // then nothing was backed up for an update
    assert!(!backups.exists());
    fs::remove_dir_all(&home).unwrap();
}

#[tokio::test]
async fn a_new_installation_makes_no_backup_of_its_empty_collection() {
    // given no collection yet
    let home: PathBuf = empty_folder();
    fs::create_dir_all(&home).unwrap();

    // when the app starts for the first time
    let backups: PathBuf = home.join("backups");
    backup::open_collection(&home.join("collection.sqlite"), &backups, monday())
        .await
        .unwrap();

    // then there is nothing to protect, so no backup
    assert!(!backups.exists());
    fs::remove_dir_all(&home).unwrap();
}
