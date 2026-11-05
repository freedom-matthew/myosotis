mod common;

use common::*;
use myosotis_app::features::{library, review};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{Grade, LibraryCard, SessionCard};

async fn the_library(repo: &SqliteCardRepo) -> Vec<LibraryCard> {
    library::list(repo, monday()).await.unwrap()
}

#[tokio::test]
async fn the_library_shows_accepted_cards_but_not_cards_still_in_the_inbox() {
    // given one accepted card and one card waiting in the Inbox
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    import_source_with(&repo, &FakeGenerator::producing(&["Still in the Inbox?"])).await;

    // when I open the Library
    let cards: Vec<LibraryCard> = the_library(&repo).await;

    // then only the accepted card is listed
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].question, "Question 1");
}

#[tokio::test]
async fn a_new_card_has_no_recall_yet() {
    // given a card I have never reviewed
    let repo: SqliteCardRepo = collection_with_cards(1).await;

    // when I open the Library
    let cards: Vec<LibraryCard> = the_library(&repo).await;

    // then it has no recall figure and no reviews
    assert_eq!(cards[0].recall_percent, None);
    assert_eq!(cards[0].review_count, 0);
}

#[tokio::test]
async fn a_learned_card_shows_its_recall_reviews_and_next_date() {
    // given a card I graded Easy
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;

    // when I open the Library
    let cards: Vec<LibraryCard> = the_library(&repo).await;

    // then it shows full recall, one review, and a date in the future
    assert_eq!(cards[0].recall_percent, Some(100));
    assert_eq!(cards[0].review_count, 1);
    assert!(cards[0].next_due > monday());
}

#[tokio::test]
async fn a_suspended_card_leaves_review_until_it_is_resumed() {
    // given a card in today's session
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // when I suspend it, it is no longer offered, but stays in the Library
    library::suspend(&repo, session[0].id, true).await.unwrap();
    assert!(session_at(&repo, monday()).await.is_empty());
    assert!(the_library(&repo).await[0].suspended);

    // and when I resume it, it is offered again
    library::suspend(&repo, session[0].id, false).await.unwrap();
    assert_eq!(session_at(&repo, monday()).await.len(), 1);
}

#[tokio::test]
async fn deleting_a_card_removes_it_and_its_history() {
    // given a card with a review
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Again, monday()).await;

    // when I delete it
    library::delete(&repo, session[0].id).await.unwrap();

    // then it is gone from the Library, from review and from the history
    assert!(the_library(&repo).await.is_empty());
    assert!(session_at(&repo, monday()).await.is_empty());
    assert_eq!(repo.review_count(session[0].id).await.unwrap(), 0);
}

#[tokio::test]
async fn editing_a_card_in_the_library_changes_its_wording() {
    // given a card
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let id: i64 = the_library(&repo).await[0].id;

    // when I reword it
    review::edit(&repo, id, "Better question", "Better answer")
        .await
        .unwrap();

    // then the Library shows the new wording
    assert_eq!(the_library(&repo).await[0].question, "Better question");
    assert_eq!(the_library(&repo).await[0].answer, "Better answer");
}
