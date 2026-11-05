mod common;

use common::*;
use myosotis_app::features::{library, tidy};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{Card, CardId, Grade, LibraryCard, SessionCard, TidyGroup, TidyMember};

async fn collection_with_two_similar_cards() -> SqliteCardRepo {
    let repo: SqliteCardRepo = empty_collection().await;
    repo.insert(&Card::new(
        "What triggers a habit routine?",
        "A contextual cue",
    ))
    .await
    .unwrap();
    repo.insert(&Card::new(
        "Which contextual cue triggers habit routine?",
        "A cue",
    ))
    .await
    .unwrap();
    repo.insert(&Card::new("Who wrote Hamlet?", "Shakespeare"))
        .await
        .unwrap();
    repo
}

async fn the_groups(repo: &SqliteCardRepo) -> Vec<TidyGroup> {
    tidy::groups(repo).await.unwrap()
}

#[tokio::test]
async fn similar_cards_are_grouped_and_nothing_changes_yet() {
    // given two similar cards and an unrelated one
    let repo: SqliteCardRepo = collection_with_two_similar_cards().await;

    // when I open Tidy up
    let groups: Vec<TidyGroup> = the_groups(&repo).await;

    // then the similar pair is proposed, and all three cards still exist
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].members.len(), 2);
    assert_eq!(library::list(&repo, monday()).await.unwrap().len(), 3);
}

#[tokio::test]
async fn the_card_with_the_most_reviews_keeps_its_history() {
    // given the second similar card has been reviewed
    let repo: SqliteCardRepo = collection_with_two_similar_cards().await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[1], Grade::Easy, monday()).await;

    // when I open Tidy up
    let groups: Vec<TidyGroup> = the_groups(&repo).await;

    // then that card is the one that keeps its history, and its wording is suggested
    let keeper: &TidyMember = groups[0]
        .members
        .iter()
        .find(|member| member.keeps_history)
        .unwrap();
    assert_eq!(keeper.id, session[1].id);
    assert_eq!(groups[0].suggested_question, session[1].question);
}

#[tokio::test]
async fn merging_keeps_one_card_with_my_wording_and_its_reviews() {
    // given a group whose second card has a review
    let repo: SqliteCardRepo = collection_with_two_similar_cards().await;
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;
    grade(&repo, &session[1], Grade::Easy, monday()).await;
    let (keeper, other): (CardId, CardId) = (session[1].id, session[0].id);

    // when I merge them with new wording
    tidy::merge(&repo, keeper, &[other], "What starts a habit?", "A cue")
        .await
        .unwrap();

    // then one card replaces the two, with the wording and review kept
    let cards: Vec<LibraryCard> = library::list(&repo, monday()).await.unwrap();
    assert_eq!(cards.len(), 2);
    let merged: &LibraryCard = cards.iter().find(|card| card.id == keeper).unwrap();
    assert_eq!(merged.question, "What starts a habit?");
    assert_eq!(merged.review_count, 1);
    assert!(the_groups(&repo).await.is_empty());
}

#[tokio::test]
async fn not_duplicates_is_remembered() {
    // given a proposed group
    let repo: SqliteCardRepo = collection_with_two_similar_cards().await;
    let groups: Vec<TidyGroup> = the_groups(&repo).await;
    let ids: Vec<CardId> = groups[0].members.iter().map(|member| member.id).collect();

    // when I say they are different
    tidy::not_duplicates(&repo, &ids).await.unwrap();

    // then they aren't proposed again
    assert!(the_groups(&repo).await.is_empty());
}
