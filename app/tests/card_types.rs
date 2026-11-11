mod common;

use common::*;
use myosotis_app::features::library;
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{CardKind, Grade, LibraryCard, SessionCard};

#[tokio::test]
async fn a_question_card_reaches_review_as_that_kind() {
    // given the AI made a question card, and I accepted it
    let repo: SqliteCardRepo = empty_collection().await;
    let session: Vec<SessionCard> = accepted_card(
        &repo,
        CardKind::Question,
        "What does Rust check?",
        "Ownership",
    )
    .await;

    // then I review it as a question
    assert_eq!(session[0].kind, CardKind::Question);
}

#[tokio::test]
async fn a_describe_card_reaches_review_with_its_model_answer() {
    // given the AI made a describe card, and I accepted it
    let repo: SqliteCardRepo = empty_collection().await;
    let session: Vec<SessionCard> = accepted_card(
        &repo,
        CardKind::Describe,
        "Why does spacing beat cramming?",
        "Effortful retrieval strengthens memory.",
    )
    .await;

    // then I review it as a describe card, with the model answer to compare against
    assert_eq!(session[0].kind, CardKind::Describe);
    assert_eq!(session[0].answer, "Effortful retrieval strengthens memory.");
}

#[tokio::test]
async fn accepting_a_concept_card_creates_both_directions() {
    // given the AI made a concept card, and I accepted it
    let repo: SqliteCardRepo = empty_collection().await;
    accepted_card(&repo, CardKind::Concept, "Haus", "House").await;

    // then the Library has it both ways round, both as concepts
    let cards: Vec<LibraryCard> = library::list(&repo, monday()).await.unwrap();
    assert_eq!(cards.len(), 2);
    assert!(
        cards
            .iter()
            .any(|card| card.question == "Haus" && card.answer == "House")
    );
    assert!(
        cards
            .iter()
            .any(|card| card.question == "House" && card.answer == "Haus")
    );
    assert!(cards.iter().all(|card| card.kind == CardKind::Concept));
}

#[tokio::test]
async fn the_two_directions_of_a_concept_are_not_reviewed_on_the_same_day() {
    // given a concept card, accepted
    let repo: SqliteCardRepo = empty_collection().await;

    // when I start a session, only one direction is offered
    let session: Vec<SessionCard> = accepted_card(&repo, CardKind::Concept, "Haus", "House").await;
    assert_eq!(session.len(), 1);

    // and after reviewing it, the other waits for another day
    grade(&repo, &session[0], Grade::Easy, monday()).await;
    assert!(session_at(&repo, monday()).await.is_empty());
    let tomorrow: Vec<SessionCard> = session_at(&repo, days_later(monday(), 1)).await;
    assert_eq!(tomorrow.len(), 1);
    assert_ne!(tomorrow[0].id, session[0].id);
}
