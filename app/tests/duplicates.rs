mod common;

use common::*;
use myosotis_app::features::{inbox, library};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{Card, DraftCard, ExistingCard, Grade, LibraryCard, SessionCard};

async fn collection_knowing_habits() -> SqliteCardRepo {
    let repo: SqliteCardRepo = empty_collection().await;
    repo.insert(&Card::new(
        "What triggers a habit routine?",
        "A contextual cue",
    ))
    .await
    .unwrap();
    repo
}

#[tokio::test]
async fn a_new_card_like_an_existing_one_is_flagged_in_the_inbox() {
    // given I already have a card about habit cues
    let repo: SqliteCardRepo = collection_knowing_habits().await;

    // when a source produces nearly the same card
    let generator: FakeGenerator =
        FakeGenerator::producing(&["Which cue triggers a habit routine?"]);
    import_source_with(&repo, &generator).await;

    // then the Inbox flags it and shows the existing card
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;
    let existing: ExistingCard = drafts[0].duplicate.clone().unwrap();
    assert_eq!(existing.question, "What triggers a habit routine?");
}

#[tokio::test]
async fn an_unrelated_card_is_not_flagged() {
    // given I have a card about habit cues
    let repo: SqliteCardRepo = collection_knowing_habits().await;

    // when a source produces something different
    let generator: FakeGenerator = FakeGenerator::producing(&["Who wrote Hamlet?"]);
    import_source_with(&repo, &generator).await;

    // then it is a normal new card
    assert_eq!(drafts_in_inbox(&repo).await[0].duplicate, None);
}

#[tokio::test]
async fn adding_the_source_keeps_the_existing_card_and_its_reviews() {
    // given a reviewed card and a duplicate of it in the Inbox
    let repo: SqliteCardRepo = collection_knowing_habits().await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;
    let generator: FakeGenerator =
        FakeGenerator::producing(&["Which cue triggers a habit routine?"]);
    import_source_with(&repo, &generator).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when I add the source to the existing card
    inbox::add_source_to_existing(&repo, drafts[0].id)
        .await
        .unwrap();

    // then there is still one card, with its review, and it now shows that source
    let cards: Vec<LibraryCard> = library::list(&repo, monday()).await.unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].review_count, 1);
    assert_eq!(cards[0].source_title.as_deref(), Some("Habit notes"));
    assert!(the_inbox(&repo).await.is_empty());
}

#[tokio::test]
async fn keep_both_treats_the_card_as_a_normal_new_one() {
    // given a suspected duplicate in the Inbox
    let repo: SqliteCardRepo = collection_knowing_habits().await;
    let generator: FakeGenerator =
        FakeGenerator::producing(&["Which cue triggers a habit routine?"]);
    import_source_with(&repo, &generator).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when I keep both and accept it
    inbox::keep_both(&repo, drafts[0].id).await.unwrap();
    assert_eq!(drafts_in_inbox(&repo).await[0].duplicate, None);
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();

    // then I have both cards
    assert_eq!(library::list(&repo, monday()).await.unwrap().len(), 2);
}

#[tokio::test]
async fn skipping_a_duplicate_leaves_the_collection_as_it_was() {
    // given a suspected duplicate in the Inbox
    let repo: SqliteCardRepo = collection_knowing_habits().await;
    let generator: FakeGenerator =
        FakeGenerator::producing(&["Which cue triggers a habit routine?"]);
    import_source_with(&repo, &generator).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when I skip it
    inbox::delete(&repo, drafts[0].id).await.unwrap();

    // then only the original remains
    assert_eq!(library::list(&repo, monday()).await.unwrap().len(), 1);
}
