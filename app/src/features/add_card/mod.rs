use chrono::{DateTime, Utc};
use myosotis_core::{AddCardError, CardId, CardRepo, NewCard, Schedule};
use tauri::State;

use crate::storage::SqliteCardRepo;

/// Adds a card written by hand. It skips the Inbox and is ready for review as new.
pub async fn handle(
    repo: &impl CardRepo,
    card: &NewCard,
    now: DateTime<Utc>,
) -> Result<CardId, AddCardError> {
    let card: NewCard = card.tidied()?;
    Ok(repo.add(&card, &Schedule::new_card(now)).await?)
}

#[tauri::command]
pub async fn add_card(repo: State<'_, SqliteCardRepo>, card: NewCard) -> Result<CardId, String> {
    handle(repo.inner(), &card, Utc::now())
        .await
        .map_err(|error| error.to_string())
}
