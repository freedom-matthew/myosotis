mod common;

use common::*;
use myosotis_app::features::{add_card, library};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{AddCardError, CardKind, LibraryCard, NewCard, SessionCard};

fn written(kind: CardKind, question: &str, answer: &str) -> NewCard {
    NewCard {
        kind,
        question: question.to_string(),
        answer: answer.to_string(),
    }
}

async fn library_of(repo: &SqliteCardRepo) -> Vec<LibraryCard> {
    library::list(repo, monday()).await.unwrap()
}

#[tokio::test]
async fn a_card_i_add_is_in_my_library_straight_away() {
    // given an empty collection
    let repo: SqliteCardRepo = empty_collection().await;

    // when I add a question by hand
    let card: NewCard = written(CardKind::Question, "What does FSRS schedule?", "Reviews");
    add_card::handle(&repo, &card, monday()).await.unwrap();

    // then it is in the Library without going through the Inbox, and has no source
    let cards: Vec<LibraryCard> = library_of(&repo).await;
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].question, "What does FSRS schedule?");
    assert_eq!(cards[0].answer, "Reviews");
    assert_eq!(cards[0].kind, CardKind::Question);
    assert_eq!(cards[0].source_title, None);
}

#[tokio::test]
async fn a_card_i_add_is_reviewed_as_a_new_card() {
    // given I added a card
    let repo: SqliteCardRepo = empty_collection().await;
    let card: NewCard = written(CardKind::Describe, "Why space practice?", "Effort helps.");
    add_card::handle(&repo, &card, monday()).await.unwrap();

    // when I start a session
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // then it is there as a new card of the kind I chose
    assert_eq!(session.len(), 1);
    assert!(session[0].is_new);
    assert_eq!(session[0].kind, CardKind::Describe);
}

#[tokio::test]
async fn a_concept_i_add_is_reviewed_both_ways() {
    // given an empty collection
    let repo: SqliteCardRepo = empty_collection().await;

    // when I add a concept
    let card: NewCard = written(CardKind::Concept, "Haus", "House");
    add_card::handle(&repo, &card, monday()).await.unwrap();

    // then the Library has it both ways round
    let cards: Vec<LibraryCard> = library_of(&repo).await;
    assert_eq!(cards.len(), 2);
    assert!(
        cards
            .iter()
            .any(|card| card.question == "House" && card.answer == "Haus")
    );
}

#[tokio::test]
async fn spaces_around_what_i_typed_are_dropped() {
    // given an empty collection
    let repo: SqliteCardRepo = empty_collection().await;

    // when I add a card with stray spaces and line breaks
    let card: NewCard = written(CardKind::Question, "  What is Rust?\n", " A language \n");
    add_card::handle(&repo, &card, monday()).await.unwrap();

    // then they are not stored
    let cards: Vec<LibraryCard> = library_of(&repo).await;
    assert_eq!(cards[0].question, "What is Rust?");
    assert_eq!(cards[0].answer, "A language");
}

#[tokio::test]
async fn a_card_without_an_answer_is_refused() {
    // given an empty collection
    let repo: SqliteCardRepo = empty_collection().await;

    // when I try to add a card whose answer is only spaces
    let card: NewCard = written(CardKind::Question, "What is Rust?", "   ");
    let result: Result<i64, AddCardError> = add_card::handle(&repo, &card, monday()).await;

    // then it is refused, and nothing is stored
    assert!(matches!(result, Err(AddCardError::Blank)));
    assert!(library_of(&repo).await.is_empty());
}
