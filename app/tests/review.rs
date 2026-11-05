mod common;

use common::*;
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{Grade, SessionCard};

#[tokio::test]
async fn a_new_card_is_ready_to_review() {
    // given a collection with one card
    let repo: SqliteCardRepo = collection_with_cards(1).await;

    // when I start a review session
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // then that card is shown
    assert_eq!(questions(&session), vec!["Question 1"]);
}

#[tokio::test]
async fn grading_a_card_easy_removes_it_from_todays_session() {
    // given a card in today's session
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // when I grade it Easy
    grade(&repo, &session[0], Grade::Easy, monday()).await;

    // then there is nothing left to review
    assert!(session_at(&repo, monday()).await.is_empty());
}

#[tokio::test]
async fn a_forgotten_card_comes_back_in_the_same_session() {
    // given a card in today's session
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // when I grade it Again
    grade(&repo, &session[0], Grade::Again, monday()).await;

    // then I see it again a few minutes later
    let session: Vec<SessionCard> = session_at(&repo, minutes_later(monday(), 5)).await;
    assert_eq!(questions(&session), vec!["Question 1"]);
}

#[tokio::test]
async fn a_learned_card_returns_when_its_review_date_arrives() {
    // given a card I graded Easy
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;

    // when a month has passed
    let session: Vec<SessionCard> = session_at(&repo, days_later(monday(), 30)).await;

    // then it is due again
    assert_eq!(questions(&session), vec!["Question 1"]);
}

#[tokio::test]
async fn grade_buttons_preview_the_next_interval_for_all_four_grades() {
    // given a card in today's session
    let repo: SqliteCardRepo = collection_with_cards(1).await;

    // when I look at its grade buttons
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    let previews: Vec<(Grade, &str)> = session[0]
        .grades
        .iter()
        .map(|preview| (preview.grade, preview.interval.as_str()))
        .collect();

    // then each grade shows when the card would return, easier grades later
    assert_eq!(
        previews,
        vec![
            (Grade::Again, "1m"),
            (Grade::Hard, "5m"),
            (Grade::Good, "10m"),
            (Grade::Easy, "15d"),
        ]
    );
}

#[tokio::test]
async fn at_most_twenty_new_cards_are_started_per_day() {
    // given 25 new cards
    let repo: SqliteCardRepo = collection_with_cards(25).await;

    // when I start a session
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // then only 20 are offered
    assert_eq!(session.len(), 20);
}

#[tokio::test]
async fn new_cards_started_earlier_today_count_towards_the_daily_limit() {
    // given 25 new cards, 3 of them already reviewed today
    let repo: SqliteCardRepo = collection_with_cards(25).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    for card in &session[..3] {
        grade(&repo, card, Grade::Easy, monday()).await;
    }

    // when I come back later the same day
    let session: Vec<SessionCard> = session_at(&repo, minutes_later(monday(), 30)).await;

    // then only the remaining 17 new cards are offered
    assert_eq!(session.len(), 17);
}

#[tokio::test]
async fn reviews_come_before_new_cards() {
    // given a learned card that is now due, and a card added afterwards
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;
    repo.insert(&myosotis_core::Card::new("Brand new", "Answer"))
        .await
        .unwrap();

    // when I start a session a month later
    let session: Vec<SessionCard> = session_at(&repo, days_later(monday(), 30)).await;

    // then the learned card comes first
    assert_eq!(questions(&session), vec!["Question 1", "Brand new"]);
}

#[tokio::test]
async fn every_review_is_kept_in_the_cards_history() {
    // given a card
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // when I review it twice
    grade(&repo, &session[0], Grade::Again, monday()).await;
    grade(&repo, &session[0], Grade::Good, minutes_later(monday(), 5)).await;

    // then both reviews are recorded
    assert_eq!(repo.review_count(session[0].id).await.unwrap(), 2);
}
