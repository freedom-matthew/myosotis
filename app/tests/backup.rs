mod common;

use std::fs;
use std::path::PathBuf;
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
