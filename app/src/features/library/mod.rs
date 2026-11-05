use chrono::{DateTime, Utc};
use myosotis_core::{CardId, LibraryCard, LibraryRepo, RepoError};
use tauri::State;

use crate::storage::SqliteCardRepo;

pub async fn list(
    repo: &impl LibraryRepo,
    now: DateTime<Utc>,
) -> Result<Vec<LibraryCard>, RepoError> {
    let cards: Vec<myosotis_core::StoredCard> = repo.all_cards().await?;
    Ok(cards
        .into_iter()
        .map(|card| card.into_library_card(now))
        .collect())
}

pub async fn suspend(
    repo: &impl LibraryRepo,
    card: CardId,
    suspended: bool,
) -> Result<(), RepoError> {
    repo.set_suspended(card, suspended).await
}

pub async fn delete(repo: &impl LibraryRepo, card: CardId) -> Result<(), RepoError> {
    repo.delete_card(card).await
}

#[tauri::command]
pub async fn library_cards(repo: State<'_, SqliteCardRepo>) -> Result<Vec<LibraryCard>, String> {
    list(repo.inner(), Utc::now())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn set_suspended(
    repo: State<'_, SqliteCardRepo>,
    card: CardId,
    suspended: bool,
) -> Result<(), String> {
    suspend(repo.inner(), card, suspended)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_card(repo: State<'_, SqliteCardRepo>, card: CardId) -> Result<(), String> {
    delete(repo.inner(), card)
        .await
        .map_err(|error| error.to_string())
}
