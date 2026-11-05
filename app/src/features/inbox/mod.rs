use chrono::{DateTime, Utc};
use myosotis_core::{
    Card, CardId, CardKind, CardRewriter, DraftCard, GenerateError, ImportId, ImportRepo,
    InboxImport, RepoError, RewriteRequest, Schedule,
};
use tauri::State;

use crate::claude_code::ClaudeCode;
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

pub async fn keep_both(repo: &impl ImportRepo, card: CardId) -> Result<(), RepoError> {
    repo.clear_duplicate(card).await
}

pub async fn add_source_to_existing(repo: &impl ImportRepo, card: CardId) -> Result<(), RepoError> {
    let import: ImportId = repo.add_source_to_existing(card).await?;
    repo.finish_import_if_empty(import).await
}

/// Asks the AI for a better version of a draft, from its original passage.
pub async fn rewrite(
    repo: &impl ImportRepo,
    rewriter: &impl CardRewriter,
    card: CardId,
) -> Result<(), GenerateError> {
    let draft: DraftCard = repo
        .draft(card)
        .await
        .map_err(|error| GenerateError(error.to_string()))?;
    let request: RewriteRequest = RewriteRequest {
        kind: draft.kind,
        question: draft.question,
        answer: draft.answer,
        excerpt: draft.excerpt,
    };
    let better: Card = rewriter.rewrite(&request).await?;
    let question: String = if request.kind == CardKind::Cloze && !better.question.contains("{{") {
        request.question
    } else {
        better.question
    };
    repo.update_draft(card, &Card::new(question, better.answer))
        .await
        .map_err(|error| GenerateError(error.to_string()))
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
pub async fn rewrite_draft(repo: State<'_, SqliteCardRepo>, card: CardId) -> Result<(), String> {
    rewrite(repo.inner(), &ClaudeCode, card)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn keep_both_cards(repo: State<'_, SqliteCardRepo>, card: CardId) -> Result<(), String> {
    keep_both(repo.inner(), card)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn merge_source_into_existing(
    repo: State<'_, SqliteCardRepo>,
    card: CardId,
) -> Result<(), String> {
    add_source_to_existing(repo.inner(), card)
        .await
        .map_err(|error| error.to_string())
}
