use chrono::{DateTime, Utc};
use myosotis_core::{Card, CardId, ImportId, ImportRepo, InboxImport, RepoError, Schedule};
use tauri::State;

use crate::storage::SqliteCardRepo;

pub async fn list(repo: &impl ImportRepo) -> Result<Vec<InboxImport>, RepoError> {
    repo.inbox().await
}

pub async fn accept(
    repo: &impl ImportRepo,
    card: CardId,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let import: ImportId = repo.accept_draft(card, &Schedule::new_card(now)).await?;
    repo.finish_import_if_empty(import).await
}

pub async fn accept_all(
    repo: &impl ImportRepo,
    import: ImportId,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let drafts: Vec<CardId> = repo.drafts_of(import).await?;
    for card in drafts {
        accept(repo, card, now).await?;
    }
    Ok(())
}

pub async fn edit(
    repo: &impl ImportRepo,
    card: CardId,
    question: &str,
    answer: &str,
) -> Result<(), RepoError> {
    repo.update_draft(card, &Card::new(question, answer)).await
}

pub async fn delete(repo: &impl ImportRepo, card: CardId) -> Result<(), RepoError> {
    let import: ImportId = repo.delete_draft(card).await?;
    repo.finish_import_if_empty(import).await
}

#[tauri::command]
pub async fn inbox(repo: State<'_, SqliteCardRepo>) -> Result<Vec<InboxImport>, String> {
    list(repo.inner()).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn accept_draft(repo: State<'_, SqliteCardRepo>, card: CardId) -> Result<(), String> {
    accept(repo.inner(), card, Utc::now())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn accept_all_drafts(
    repo: State<'_, SqliteCardRepo>,
    import: ImportId,
) -> Result<(), String> {
    accept_all(repo.inner(), import, Utc::now())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn edit_draft(
    repo: State<'_, SqliteCardRepo>,
    card: CardId,
    question: String,
    answer: String,
) -> Result<(), String> {
    edit(repo.inner(), card, &question, &answer)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_draft(repo: State<'_, SqliteCardRepo>, card: CardId) -> Result<(), String> {
    delete(repo.inner(), card)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn discard_import(
    repo: State<'_, SqliteCardRepo>,
    import: ImportId,
) -> Result<(), String> {
    repo.discard_failed_import(import)
        .await
        .map_err(|error| error.to_string())
}
