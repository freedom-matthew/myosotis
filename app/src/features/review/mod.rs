use chrono::{DateTime, Utc};
use myosotis_core::{
    Card, CardExplainer, CardId, DueCard, ExplanationRequest, GenerateError, Grade, RepoError,
    ReviewRepo, Schedule, SessionCard, plan_session, session_horizon,
};
use tauri::State;

use crate::claude_code::ClaudeCode;
use crate::clock;
use crate::storage::SqliteCardRepo;

pub async fn session(
    repo: &impl ReviewRepo,
    now: DateTime<Utc>,
    start_of_day: DateTime<Utc>,
) -> Result<Vec<SessionCard>, RepoError> {
    let cards: Vec<DueCard> = repo.due_cards(session_horizon(now)).await?;
    let new_started: usize = repo.new_cards_started_since(start_of_day).await?;
    Ok(plan_session(cards, new_started, now))
}

pub async fn grade(
    repo: &impl ReviewRepo,
    card: CardId,
    grade: Grade,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let before: Schedule = repo.schedule_of(card).await?;
    let after: Schedule = before.after(grade, now);
    repo.record_review(card, grade, &before, &after, now).await
}

pub async fn undo(repo: &impl ReviewRepo, card: CardId) -> Result<(), RepoError> {
    repo.undo_review(card).await
}

pub async fn edit(
    repo: &impl ReviewRepo,
    card: CardId,
    question: &str,
    answer: &str,
) -> Result<(), RepoError> {
    repo.update_card(card, &Card::new(question, answer)).await
}

pub async fn explain(
    repo: &impl ReviewRepo,
    explainer: &impl CardExplainer,
    card: CardId,
) -> Result<String, GenerateError> {
    let request: ExplanationRequest = repo
        .explanation_request(card)
        .await
        .map_err(|error| GenerateError(error.to_string()))?;
    explainer.explain(&request).await
}

#[tauri::command]
pub async fn review_session(repo: State<'_, SqliteCardRepo>) -> Result<Vec<SessionCard>, String> {
    session(repo.inner(), Utc::now(), clock::start_of_local_day())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn grade_card(
    repo: State<'_, SqliteCardRepo>,
    card: CardId,
    grade: Grade,
) -> Result<(), String> {
    self::grade(repo.inner(), card, grade, Utc::now())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn undo_grade(repo: State<'_, SqliteCardRepo>, card: CardId) -> Result<(), String> {
    undo(repo.inner(), card)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn edit_card(
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
pub async fn explain_card(repo: State<'_, SqliteCardRepo>, card: CardId) -> Result<String, String> {
    explain(repo.inner(), &ClaudeCode, card)
        .await
        .map_err(|error| error.to_string())
}
