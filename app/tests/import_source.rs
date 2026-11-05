mod common;

use common::*;
use myosotis_app::features::{import_source, inbox};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{DraftCard, ImportId, ImportStatus, InboxImport, SessionCard};

#[tokio::test]
async fn generated_cards_wait_in_the_inbox_and_are_not_reviewed_yet() {
    // given an empty collection
    let repo: SqliteCardRepo = empty_collection().await;

    // when the AI creates two cards from a source
    import_source_with(&repo, &FakeGenerator::producing(&["Cue?", "Reward?"])).await;

    // then both wait in the Inbox, and neither appears in review
    assert_eq!(drafts_in_inbox(&repo).await.len(), 2);
    assert!(session_at(&repo, monday()).await.is_empty());
}

#[tokio::test]
async fn an_accepted_card_is_ready_to_review_and_remembers_its_source() {
    // given a generated card in the Inbox
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Cue?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when I accept it
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();

    // then I can review it, and it shows where it came from
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    assert_eq!(questions(&session), vec!["Cue?"]);
    assert_eq!(session[0].source_title.as_deref(), Some("Habit notes"));
    assert_eq!(session[0].excerpt.as_deref(), Some("Passage about: Cue?"));
}

#[tokio::test]
async fn a_card_edited_in_the_inbox_keeps_my_wording() {
    // given a generated card in the Inbox
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Cue?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when I reword it and accept it
    inbox::edit(&repo, drafts[0].id, "What triggers a habit?", "A cue")
        .await
        .unwrap();
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();

    // then review shows my wording
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    assert_eq!(session[0].question, "What triggers a habit?");
    assert_eq!(session[0].answer, "A cue");
}

#[tokio::test]
async fn a_deleted_card_never_reaches_review() {
    // given two generated cards in the Inbox
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Keep?", "Bin?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when I accept one and delete the other
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();
    inbox::delete(&repo, drafts[1].id).await.unwrap();

    // then only the accepted one can be reviewed
    assert_eq!(questions(&session_at(&repo, monday()).await), vec!["Keep?"]);
}

#[tokio::test]
async fn accept_all_accepts_every_remaining_card_of_that_source() {
    // given three generated cards
    let repo: SqliteCardRepo = empty_collection().await;
    let generator: FakeGenerator = FakeGenerator::producing(&["One?", "Two?", "Three?"]);
    let import: ImportId = import_source_with(&repo, &generator).await;

    // when I accept all of them
    inbox::accept_all(&repo, import, monday()).await.unwrap();

    // then the Inbox is empty and all three can be reviewed
    assert!(the_inbox(&repo).await.is_empty());
    assert_eq!(session_at(&repo, monday()).await.len(), 3);
}

#[tokio::test]
async fn the_source_text_is_kept_until_the_inbox_is_finished_then_deleted() {
    // given a source with two generated cards
    let repo: SqliteCardRepo = empty_collection().await;
    let generator: FakeGenerator = FakeGenerator::producing(&["One?", "Two?"]);
    let import: ImportId = import_source_with(&repo, &generator).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when one card is still waiting, the text is still there
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();
    assert!(repo.source_text(import).await.unwrap().is_some());

    // and when the last card is dealt with, the text is deleted
    inbox::delete(&repo, drafts[1].id).await.unwrap();
    assert_eq!(repo.source_text(import).await.unwrap(), None);
}

#[tokio::test]
async fn cards_keep_their_source_excerpt_after_the_full_text_is_deleted() {
    // given a card accepted from a source whose Inbox is finished
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Cue?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();

    // when I review it
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // then its short excerpt is still available
    assert_eq!(session[0].excerpt.as_deref(), Some("Passage about: Cue?"));
}

#[tokio::test]
async fn a_failed_generation_is_reported_and_can_be_retried() {
    // given the AI failed on a source
    let repo: SqliteCardRepo = empty_collection().await;
    let failing: FakeGenerator = FakeGenerator::failing_with("Claude Code is not signed in");
    let import: ImportId = import_source_with(&repo, &failing).await;

    // then the Inbox shows why
    let inbox_now: Vec<InboxImport> = the_inbox(&repo).await;
    assert_eq!(inbox_now[0].status, ImportStatus::Failed);
    assert_eq!(
        inbox_now[0].error.as_deref(),
        Some("Claude Code is not signed in")
    );

    // and when I retry and it works, the cards arrive from the same text
    let working: FakeGenerator = FakeGenerator::producing(&["Cue?"]);
    import_source::generate(&repo, &working, import)
        .await
        .unwrap();
    assert_eq!(drafts_in_inbox(&repo).await.len(), 1);
}

#[tokio::test]
async fn generating_no_cards_counts_as_a_failure() {
    // given the AI found nothing worth keeping
    let repo: SqliteCardRepo = empty_collection().await;

    // when the import finishes
    import_source_with(&repo, &FakeGenerator::producing(&[])).await;

    // then I am told, rather than the source silently vanishing
    assert_eq!(the_inbox(&repo).await[0].status, ImportStatus::Failed);
}
