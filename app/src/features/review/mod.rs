use chrono::{DateTime, Utc};
use myosotis_core::{
    CardId, DueCard, Grade, RepoError, ReviewRepo, Schedule, SessionCard, plan_session,
    session_horizon,
};
use tauri::State;

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
    repo.record_review(card, grade, &after, before.is_new(), now)
        .await
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
