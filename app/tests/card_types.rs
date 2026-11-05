mod common;

use common::*;
use myosotis_app::features::{inbox, library};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{CardKind, DraftCard, GeneratedCard, Grade, LibraryCard, SessionCard};

fn generated(kind: CardKind, question: &str, answer: &str) -> GeneratedCard {
    GeneratedCard {
        kind,
        question: question.to_string(),
        answer: answer.to_string(),
        excerpt: "A passage.".to_string(),
    }
}

async fn accept_everything(repo: &SqliteCardRepo) {
    let drafts: Vec<DraftCard> = drafts_in_inbox(repo).await;
    for draft in drafts {
        inbox::accept(repo, draft.id, monday()).await.unwrap();
    }
}

#[tokio::test]
async fn a_fill_the_gap_card_reaches_review_as_that_kind() {
    // given the AI made a fill-the-gap card
    let repo: SqliteCardRepo = empty_collection().await;
    let card: GeneratedCard = generated(CardKind::Cloze, "Rust has {{ownership}}.", "ignored");
    import_source_with(&repo, &FakeGenerator::with_cards(vec![card])).await;

    // when I accept it and review
    accept_everything(&repo).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // then it is a fill-the-gap card, and its answer is the text of the gap
    assert_eq!(session[0].kind, CardKind::Cloze);
    assert_eq!(session[0].answer, "ownership");
}

#[tokio::test]
async fn a_fill_the_gap_card_without_a_gap_becomes_a_normal_card() {
    // given the AI forgot to mark the gap
    let repo: SqliteCardRepo = empty_collection().await;
    let card: GeneratedCard = generated(CardKind::Cloze, "What does Rust have?", "Ownership");
    import_source_with(&repo, &FakeGenerator::with_cards(vec![card])).await;

    // when I look at the Inbox
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // then it is a plain question and answer
    assert_eq!(drafts[0].kind, CardKind::Qa);
    assert_eq!(drafts[0].answer, "Ownership");
}

#[tokio::test]
async fn accepting_a_two_way_card_creates_both_directions() {
    // given the AI made a two-way term
    let repo: SqliteCardRepo = empty_collection().await;
    let card: GeneratedCard = generated(CardKind::Reverse, "Haus", "House");
    import_source_with(&repo, &FakeGenerator::with_cards(vec![card])).await;

    // when I accept it
    accept_everything(&repo).await;

    // then the Library has it both ways round
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
}

#[tokio::test]
async fn the_two_directions_of_a_card_are_not_reviewed_on_the_same_day() {
    // given a two-way card, accepted
    let repo: SqliteCardRepo = empty_collection().await;
    let card: GeneratedCard = generated(CardKind::Reverse, "Haus", "House");
    import_source_with(&repo, &FakeGenerator::with_cards(vec![card])).await;
    accept_everything(&repo).await;

    // when I start a session, only one direction is offered
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    assert_eq!(session.len(), 1);

    // and after reviewing it, the other waits for another day
    grade(&repo, &session[0], Grade::Easy, monday()).await;
    assert!(session_at(&repo, monday()).await.is_empty());
    let tomorrow: Vec<SessionCard> = session_at(&repo, days_later(monday(), 1)).await;
    assert_eq!(tomorrow.len(), 1);
    assert_ne!(tomorrow[0].id, session[0].id);
}
