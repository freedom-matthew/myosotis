mod common;

use chrono::{DateTime, Duration, Utc};
use common::*;
use myosotis_app::features::today;
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{Grade, SessionCard, TodaySummary};

async fn summary_at(repo: &SqliteCardRepo, now: DateTime<Utc>) -> TodaySummary {
    let start_of_day: DateTime<Utc> = now - Duration::hours(1);
    today::summary(repo, now, start_of_day).await.unwrap()
}

#[tokio::test]
async fn today_counts_cards_due_and_estimates_the_time() {
    // given eight new cards
    let repo: SqliteCardRepo = collection_with_cards(8).await;

    // when I open Today
    let summary: TodaySummary = summary_at(&repo, monday()).await;

    // then it reports them, at about 15 seconds a card
    assert_eq!(summary.due_new, 8);
    assert_eq!(summary.due_reviews, 0);
    assert_eq!(summary.minutes, 2);
}

#[tokio::test]
async fn today_tells_me_when_the_next_card_is_due_if_nothing_is_due_now() {
    // given my only card was graded Easy
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;

    // when I open Today
    let summary: TodaySummary = summary_at(&repo, monday()).await;

    // then nothing is due, and I can see when something next will be
    assert_eq!(summary.due_new + summary.due_reviews, 0);
    assert!(summary.next_due.unwrap() > monday());
}

#[tokio::test]
async fn today_shows_how_many_cards_wait_in_the_inbox() {
    // given a source that produced two cards
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["One?", "Two?"])).await;

    // when I open Today
    let summary: TodaySummary = summary_at(&repo, monday()).await;

    // then the Inbox count is two
    assert_eq!(summary.inbox_count, 2);
}

#[tokio::test]
async fn the_seven_day_forecast_places_each_card_on_its_due_day() {
    // given a card due today and a card graded Easy, due in over a week
    let repo: SqliteCardRepo = collection_with_cards(2).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;

    // when I open Today
    let summary: TodaySummary = summary_at(&repo, monday()).await;

    // then today's bar has one card and the rest of the week is empty
    assert_eq!(summary.forecast, vec![1, 0, 0, 0, 0, 0, 0]);
}

#[tokio::test]
async fn retention_is_the_share_of_due_reviews_i_remembered() {
    // given two learned cards that come due, one remembered and one forgotten
    let repo: SqliteCardRepo = collection_with_cards(2).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;
    grade(&repo, &session[1], Grade::Easy, monday()).await;
    let later: DateTime<Utc> = days_later(monday(), 20);
    let due: Vec<SessionCard> = session_at(&repo, later).await;
    grade(&repo, &due[0], Grade::Good, later).await;
    grade(&repo, &due[1], Grade::Again, later).await;

    // when I open Today
    let summary: TodaySummary = summary_at(&repo, later).await;

    // then retention is 50%
    assert_eq!(summary.retention_percent, Some(50));
}

#[tokio::test]
async fn there_is_no_retention_figure_before_any_due_reviews() {
    // given a collection with only new cards
    let repo: SqliteCardRepo = collection_with_cards(1).await;

    // when I open Today
    let summary: TodaySummary = summary_at(&repo, monday()).await;

    // then there is no figure to show
    assert_eq!(summary.retention_percent, None);
}
