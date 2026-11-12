mod common;

use common::*;
use myosotis_app::features::{import_source, inbox, review};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{
    Card, Depth, DraftCard, ExplanationRequest, Grade, ImportId, ImportStatus, InboxImport,
    NewImport, SessionCard,
};

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
async fn an_accepted_card_is_ready_to_review() {
    // given a generated card in the Inbox
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Cue?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;

    // when I accept it
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();

    // then I can review it
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    assert_eq!(questions(&session), vec!["Cue?"]);
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
async fn explain_still_knows_the_passage_after_the_source_text_is_deleted() {
    // given a card accepted from a source whose Inbox is finished, so its text is gone
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Cue?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();

    // when I ask for an explanation
    let explainer: FakeExplainer = FakeExplainer::replying("A cue starts the habit.");
    review::explain(&repo, &explainer, drafts[0].id)
        .await
        .unwrap();

    // then the AI still gets the passage the card came from, though I never see it
    let asked: ExplanationRequest = explainer.last_request();
    assert_eq!(asked.excerpt.as_deref(), Some("Passage about: Cue?"));
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

#[tokio::test]
async fn accepting_a_card_twice_does_not_reset_what_i_learned() {
    // given a card I accepted and then learned
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Cue?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[0], Grade::Easy, monday()).await;

    // when the same card is accepted again, say by a double key press
    inbox::accept(&repo, drafts[0].id, monday()).await.unwrap();

    // then it stays scheduled for later
    assert!(session_at(&repo, monday()).await.is_empty());
}

#[tokio::test]
async fn a_long_source_is_worked_through_in_parts() {
    // given a source long enough for three parts
    let repo: SqliteCardRepo = empty_collection().await;
    let generator: FakeGenerator = FakeGenerator::producing(&["Cue?"]);

    // when it is imported
    import_text_with(&repo, &generator, &three_part_source()).await;

    // then the AI was called once per part, and every part's cards are in the Inbox
    assert_eq!(generator.calls(), 3);
    assert_eq!(drafts_in_inbox(&repo).await.len(), 3);
    assert_eq!(the_inbox(&repo).await[0].parts_done, 3);
}

#[tokio::test]
async fn cards_from_finished_parts_survive_a_failure_and_retry_resumes() {
    // given the AI drops out on the second of three parts
    let repo: SqliteCardRepo = empty_collection().await;
    let flaky: FakeGenerator = FakeGenerator::producing(&["Cue?"]).failing_on_call(2);
    let source: String = three_part_source();
    let import: ImportId = import_text_with(&repo, &flaky, &source).await;

    // then the first part's card is kept, and the failure is shown
    assert_eq!(drafts_in_inbox(&repo).await.len(), 1);
    assert_eq!(the_inbox(&repo).await[0].status, ImportStatus::Failed);

    // and when I retry, only the two missing parts are generated
    let working: FakeGenerator = FakeGenerator::producing(&["Cue?"]);
    import_source::generate(&repo, &working, import)
        .await
        .unwrap();
    assert_eq!(working.calls(), 2);
    assert_eq!(drafts_in_inbox(&repo).await.len(), 3);
}

#[tokio::test]
async fn rewrite_replaces_a_card_in_the_inbox() {
    // given a weak generated card
    let repo: SqliteCardRepo = empty_collection().await;
    import_source_with(&repo, &FakeGenerator::producing(&["Weak?"])).await;
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;
    let rewriter: FakeRewriter = FakeRewriter {
        better: Card::new("Why do cues trigger habits?", "They start the routine"),
    };

    // when I ask for a rewrite
    inbox::rewrite(&repo, &rewriter, drafts[0].id)
        .await
        .unwrap();

    // then the Inbox shows the better version
    let drafts: Vec<DraftCard> = drafts_in_inbox(&repo).await;
    assert_eq!(drafts[0].question, "Why do cues trigger habits?");
    assert_eq!(drafts[0].answer, "They start the routine");
}

#[tokio::test]
async fn discarding_a_part_failed_import_keeps_the_cards_already_made() {
    // given the AI dropped out after making one card
    let repo: SqliteCardRepo = empty_collection().await;
    let flaky: FakeGenerator = FakeGenerator::producing(&["Cue?"]).failing_on_call(2);
    let source: String = three_part_source();
    let import: ImportId = import_text_with(&repo, &flaky, &source).await;

    // when I discard the failed import instead of retrying
    import_source::discard(&repo, import).await.unwrap();

    // then the failure is gone, and the card is still there to check
    let inbox_now: Vec<InboxImport> = the_inbox(&repo).await;
    assert_eq!(inbox_now[0].status, ImportStatus::InInbox);
    assert_eq!(inbox_now[0].drafts.len(), 1);
}

#[tokio::test]
async fn discarding_an_import_with_no_cards_removes_it() {
    // given an import that failed before making any cards
    let repo: SqliteCardRepo = empty_collection().await;
    let failing: FakeGenerator = FakeGenerator::failing_with("Claude Code is not signed in");
    let import: ImportId = import_source_with(&repo, &failing).await;

    // when I discard it
    import_source::discard(&repo, import).await.unwrap();

    // then it is gone
    assert!(the_inbox(&repo).await.is_empty());
}

/// Starts an import the way the New source screen does, then looks at the Inbox.
async fn inbox_label_after_importing(name: &str, text: &str) -> String {
    let repo: SqliteCardRepo = empty_collection().await;
    let source: NewImport = NewImport {
        title: name.to_string(),
        text: text.to_string(),
        depth: Depth::Balanced,
        focus: None,
        describe: true,
    };
    import_source::create(&repo, &source, monday())
        .await
        .unwrap();
    the_inbox(&repo).await[0].title.clone()
}

#[tokio::test]
async fn pasted_text_needs_no_name_and_is_labelled_by_its_first_words() {
    // when I paste text without naming it
    let label: String = inbox_label_after_importing(
        "",
        "Software engineering is hard. To be successful at it, we have to learn.",
    )
    .await;

    // then the Inbox shows its first words
    assert_eq!(label, "Software engineering is hard. To be…");
}

#[tokio::test]
async fn an_opened_file_is_labelled_by_its_file_name() {
    // when I open a file
    let label: String = inbox_label_after_importing(
        "01 Analyzing Business Domains",
        "If you are anything like me",
    )
    .await;

    // then the Inbox shows the file's name
    assert_eq!(label, "01 Analyzing Business Domains");
}
