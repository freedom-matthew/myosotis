use myosotis_core::{Card, CardRepo, RepoError};
use tauri::State;

use crate::storage::SqliteCardRepo;

pub async fn handle(repo: &impl CardRepo) -> Result<Vec<Card>, RepoError> {
    repo.list().await
}

#[tauri::command]
pub async fn list_cards(repo: State<'_, SqliteCardRepo>) -> Result<Vec<Card>, String> {
    handle(repo.inner())
        .await
        .map_err(|error| error.to_string())
}
