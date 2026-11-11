mod common;

use common::*;
use myosotis_app::features::review;
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{CardKind, GenerateError, Grade, Marking, MarkingRequest, SessionCard};

fn mostly_there() -> Marking {
    Marking {
        covered: vec!["Effort strengthens memory".to_string()],
        missing: vec!["Why cramming feels effective".to_string()],
        suggested_grade: Grade::Hard,
    }
}

async fn a_describe_card(repo: &SqliteCardRepo) -> SessionCard {
    let session: Vec<SessionCard> = accepted_card(
        repo,
        CardKind::Describe,
        "Why does spacing beat cramming?",
        "Effortful retrieval strengthens memory; fluency is mistaken for learning.",
    )
    .await;
    session[0].clone()
}

#[tokio::test]
async fn the_ai_marks_my_description_against_the_model_answer() {
    // given a describe card in my session
    let repo: SqliteCardRepo = empty_collection().await;
    let card: SessionCard = a_describe_card(&repo).await;
    let marker: FakeMarker = FakeMarker::replying(mostly_there());

    // when I describe it in my own words
    let my_words: &str = "Remembering after a wait takes effort, and effort makes it stick.";
    review::describe(&repo, &marker, card.id, my_words)
        .await
        .unwrap();

    // then the AI compares my words with the card's question, model answer and passage
    let asked: MarkingRequest = marker.last_request();
    assert_eq!(asked.question, "Why does spacing beat cramming?");
    assert_eq!(
        asked.model_answer,
        "Effortful retrieval strengthens memory; fluency is mistaken for learning."
    );
    assert_eq!(asked.excerpt.as_deref(), Some("A passage."));
    assert_eq!(asked.description, my_words);
}

#[tokio::test]
async fn i_see_what_i_covered_what_i_missed_and_a_suggested_grade() {
    // given a describe card in my session
    let repo: SqliteCardRepo = empty_collection().await;
    let card: SessionCard = a_describe_card(&repo).await;

    // when the AI has marked my description
    let marker: FakeMarker = FakeMarker::replying(mostly_there());
    let marking: Marking = review::describe(&repo, &marker, card.id, "Effort makes it stick.")
        .await
        .unwrap();

    // then I get its marking back as it is
    assert_eq!(marking, mostly_there());
}

#[tokio::test]
async fn marking_does_not_grade_the_card_for_me() {
    // given a describe card in my session
    let repo: SqliteCardRepo = empty_collection().await;
    let card: SessionCard = a_describe_card(&repo).await;

    // when the AI has marked my description
    let marker: FakeMarker = FakeMarker::replying(mostly_there());
    review::describe(&repo, &marker, card.id, "Effort makes it stick.")
        .await
        .unwrap();

    // then the card waits for my grade: the suggestion is not recorded
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    assert_eq!(questions(&session), vec!["Why does spacing beat cramming?"]);
}

#[tokio::test]
async fn a_failed_marking_is_reported() {
    // given the AI is unavailable
    let repo: SqliteCardRepo = empty_collection().await;
    let card: SessionCard = a_describe_card(&repo).await;
    let marker: FakeMarker = FakeMarker::failing_with("Not logged in");

    // when I describe the card
    let error: GenerateError = review::describe(&repo, &marker, card.id, "Effort.")
        .await
        .unwrap_err();

    // then I am told why
    assert_eq!(error.0, "Not logged in");
}
