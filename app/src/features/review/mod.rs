use chrono::{DateTime, Utc};
use myosotis_core::{
    Card, CardExplainer, CardId, CardMarker, DueCard, ExplanationRequest, GenerateError, Grade,
    Marking, MarkingRequest, RepoError, ReviewRepo, Schedule, SessionCard, plan_session,
    session_horizon,
};
use myosotis_core::{Settings, SettingsRepo};
use tauri::State;

use crate::claude_code::ClaudeCode;
use crate::clock;
use crate::storage::SqliteCardRepo;

pub async fn session(
    repo: &(impl ReviewRepo + SettingsRepo),
    now: DateTime<Utc>,
    start_of_day: DateTime<Utc>,
) -> Result<Vec<SessionCard>, RepoError> {
    let cards: Vec<DueCard> = repo.due_cards(session_horizon(now)).await?;
    let new_started: usize = repo.new_cards_started_since(start_of_day).await?;
    let reviewed_today: Vec<CardId> = repo.cards_reviewed_since(start_of_day).await?;
    let settings: Settings = repo.load_settings().await?;
    Ok(plan_session(
        cards,
        new_started,
        &reviewed_today,
        &settings,
        now,
    ))
}

pub async fn grade(
    repo: &(impl ReviewRepo + SettingsRepo),
    card: CardId,
    grade: Grade,
    now: DateTime<Utc>,
) -> Result<(), RepoError> {
    let before: Schedule = repo.schedule_of(card).await?;
    let settings: Settings = repo.load_settings().await?;
    let after: Schedule = before.after(grade, now, settings.desired_retention());
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

/// Has the AI mark what I wrote for a Describe card against its model answer.
pub async fn describe(
    repo: &impl ReviewRepo,
    marker: &impl CardMarker,
    card: CardId,
    description: &str,
) -> Result<Marking, GenerateError> {
    let card: ExplanationRequest = repo
        .explanation_request(card)
        .await
        .map_err(|error| GenerateError(error.to_string()))?;
    let request: MarkingRequest = MarkingRequest {
        question: card.question,
        model_answer: card.answer,
        excerpt: card.excerpt,
        description: description.to_string(),
    };
    marker.mark(&request).await
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

#[tauri::command]
pub async fn mark_description(
    repo: State<'_, SqliteCardRepo>,
    card: CardId,
    description: String,
) -> Result<Marking, String> {
    describe(repo.inner(), &ClaudeCode, card, &description)
        .await
        .map_err(|error| error.to_string())
}
