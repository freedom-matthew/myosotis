mod common;

use common::*;
use myosotis_app::features::{concept_questions, inbox, library};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{
    Card, CardKind, CardRepo, DraftCard, GenerateError, GeneratedCard, Grade, LibraryCard, NewCard,
    Reverse, Schedule, SessionCard,
};

fn haus() -> GeneratedCard {
    GeneratedCard {
        kind: CardKind::Concept,
        question: "What does the German word Haus mean?".to_string(),
        answer: "A house.".to_string(),
        excerpt: "Das Haus means the house.".to_string(),
        reverse: Some(Reverse::new(
            "What is the German word for a house?",
            "Haus.",
        )),
    }
}

/// A concept as cards were made before: the front is a bare term, the other card its swap.
async fn old_concept(repo: &SqliteCardRepo, term: &str, meaning: &str) {
    let card: NewCard = NewCard {
        kind: CardKind::Concept,
        question: term.to_string(),
        answer: meaning.to_string(),
        reverse: None,
    };
    repo.add(&card, &Schedule::new_card(monday()))
        .await
        .unwrap();
}

fn business_domain() -> Card {
    Card::concept(
        "What is the business domain in DDD?",
        "A company's main area of activity.",
        Reverse::new(
            "In DDD, what do we call a company's main area of activity?",
            "The business domain.",
        ),
    )
}

async fn library_of(repo: &SqliteCardRepo) -> Vec<LibraryCard> {
    library::list(repo, monday()).await.unwrap()
}

fn has(cards: &[LibraryCard], question: &str, answer: &str) -> bool {
    cards
        .iter()
        .any(|card| card.question == question && card.answer == answer)
}

#[tokio::test]
async fn the_inbox_shows_both_directions_of_a_concept() {
    // given the AI made a concept
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::with_cards(vec![haus()])).await;

    // when I look at the Inbox
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // then I can check the other direction before accepting it
    assert_eq!(
        drafts[0].reverse,
        Some(Reverse::new(
            "What is the German word for a house?",
            "Haus."
        ))
    );
}

#[tokio::test]
async fn editing_a_concept_in_the_inbox_changes_both_directions() {
    // given a concept waiting in the Inbox
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::with_cards(vec![haus()])).await;
    let draft: DraftCard = drafts_in_inbox(&repo).await.remove(0);

    // when I reword both directions, then accept it
    let reverse: Reverse = Reverse::new("Which German word names a home?", "Haus.");
    inbox::edit_concept(&repo, draft.id, "What is a Haus?", "A house.", &reverse)
        .await
        .unwrap();
    inbox::accept(&repo, draft.id, monday()).await.unwrap();

    // then both cards use my wording
    let cards: Vec<LibraryCard> = library_of(&repo).await;
    assert!(has(&cards, "What is a Haus?", "A house."));
    assert!(has(&cards, "Which German word names a home?", "Haus."));
}

#[tokio::test]
async fn rewriting_a_concept_in_the_inbox_rewrites_both_directions() {
    // given a concept waiting in the Inbox
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::with_cards(vec![haus()])).await;
    let draft: DraftCard = drafts_in_inbox(&repo).await.remove(0);

    // when I ask the AI for a better version
    let rewriter: FakeRewriter = FakeRewriter {
        better: business_domain(),
    };
    inbox::rewrite(&repo, &rewriter, draft.id).await.unwrap();

    // then the Inbox shows both new directions
    let draft: DraftCard = drafts_in_inbox(&repo).await.remove(0);
    assert_eq!(draft.question, "What is the business domain in DDD?");
    assert_eq!(draft.reverse, business_domain().reverse);
}

#[tokio::test]
async fn an_old_concept_is_reworded_as_two_questions() {
    // given a concept from before, with a bare term on its front
    let repo: SqliteCardRepo = empty_collection().await;
    old_concept(
        &repo,
        "Business domain (DDD)",
        "A company's main area of activity.",
    )
    .await;

    // when the app rewords old concepts
    let rewriter: FakeRewriter = FakeRewriter {
        better: business_domain(),
    };
    concept_questions::reword_old(&repo, &rewriter)
        .await
        .unwrap();

    // then both directions are full questions, and the bare term is gone
    let cards: Vec<LibraryCard> = library_of(&repo).await;
    assert_eq!(cards.len(), 2);
    assert!(has(
        &cards,
        "What is the business domain in DDD?",
        "A company's main area of activity."
    ));
    assert!(has(
        &cards,
        "In DDD, what do we call a company's main area of activity?",
        "The business domain."
    ));
}

#[tokio::test]
async fn rewording_an_old_concept_keeps_what_i_learned() {
    // given an old concept I have already reviewed
    let repo: SqliteCardRepo = empty_collection().await;
    old_concept(
        &repo,
        "Business domain (DDD)",
        "A company's main area of activity.",
    )
    .await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Good, monday()).await;

    // when it is reworded
    let rewriter: FakeRewriter = FakeRewriter {
        better: business_domain(),
    };
    concept_questions::reword_old(&repo, &rewriter)
        .await
        .unwrap();

    // then its review history is still there
    let cards: Vec<LibraryCard> = library_of(&repo).await;
    assert_eq!(cards.iter().map(|card| card.review_count).sum::<i64>(), 1);
}

#[tokio::test]
async fn concepts_already_written_as_questions_are_left_alone() {
    // given a concept written as two questions
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::with_cards(vec![haus()])).await;
    for draft in drafts_in_inbox(&repo).await {
        inbox::accept(&repo, draft.id, monday()).await.unwrap();
    }

    // when the app looks for old concepts
    let rewriter: FakeRewriter = FakeRewriter {
        better: business_domain(),
    };
    let reworded: usize = concept_questions::reword_old(&repo, &rewriter)
        .await
        .unwrap();

    // then there is nothing to reword
    assert_eq!(reworded, 0);
    assert!(has(
        &library_of(&repo).await,
        "What is the German word for a house?",
        "Haus."
    ));
}

#[tokio::test]
async fn an_old_concept_stays_as_it_was_when_the_ai_is_unavailable() {
    // given an old concept
    let repo: SqliteCardRepo = empty_collection().await;
    old_concept(
        &repo,
        "Business domain (DDD)",
        "A company's main area of activity.",
    )
    .await;

    // when rewording fails because Claude Code can't be reached
    let result: Result<usize, GenerateError> =
        concept_questions::reword_old(&repo, &UnavailableRewriter).await;

    // then it is reported, the cards are unchanged, and the next start tries again
    assert!(result.is_err());
    let cards: Vec<LibraryCard> = library_of(&repo).await;
    assert!(has(
        &cards,
        "Business domain (DDD)",
        "A company's main area of activity."
    ));
    assert!(concept_questions::has_old(&repo).await.unwrap());
}
