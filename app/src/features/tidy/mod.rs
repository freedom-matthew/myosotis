use std::collections::HashMap;

use myosotis_core::{
    Card, CardId, LibraryRepo, RepoError, StoredCard, TidyGroup, TidyRepo, similar_groups,
    tidy_group,
};
use tauri::State;

use crate::storage::SqliteCardRepo;

/// Finds groups of similar cards. The two directions of a concept card always use the
/// same words, so they are never proposed.
pub async fn groups(repo: &(impl LibraryRepo + TidyRepo)) -> Result<Vec<TidyGroup>, RepoError> {
    let cards: Vec<StoredCard> = repo.all_cards().await?;
    let mut apart: Vec<(CardId, CardId)> = repo.not_duplicate_pairs().await?;
    for card in &cards {
        if let Some(sibling) = card.due.sibling {
            apart.push((card.due.id, sibling));
        }
    }
    let texts: Vec<(CardId, String)> = cards
        .iter()
        .map(|card| {
            let text: String = format!("{} {}", card.due.card.question, card.due.card.answer);
            (card.due.id, text)
        })
        .collect();

    let mut by_id: HashMap<CardId, StoredCard> =
        cards.into_iter().map(|card| (card.due.id, card)).collect();
    Ok(similar_groups(&texts, &apart)
        .into_iter()
        .map(|ids| {
            let members: Vec<StoredCard> = ids.iter().filter_map(|id| by_id.remove(id)).collect();
            tidy_group(members)
        })
        .collect())
}

pub async fn merge(
    repo: &impl TidyRepo,
    keeper: CardId,
    others: &[CardId],
    question: &str,
    answer: &str,
) -> Result<(), RepoError> {
    repo.merge_cards(keeper, others, &Card::new(question, answer))
        .await
}

pub async fn not_duplicates(repo: &impl TidyRepo, cards: &[CardId]) -> Result<(), RepoError> {
    repo.remember_not_duplicates(cards).await
}

#[tauri::command]
pub async fn tidy_groups(repo: State<'_, SqliteCardRepo>) -> Result<Vec<TidyGroup>, String> {
    groups(repo.inner())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn merge_group(
    repo: State<'_, SqliteCardRepo>,
    keeper: CardId,
    others: Vec<CardId>,
    question: String,
    answer: String,
) -> Result<(), String> {
    merge(repo.inner(), keeper, &others, &question, &answer)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn mark_not_duplicates(
    repo: State<'_, SqliteCardRepo>,
    cards: Vec<CardId>,
) -> Result<(), String> {
    not_duplicates(repo.inner(), &cards)
        .await
        .map_err(|error| error.to_string())
}
