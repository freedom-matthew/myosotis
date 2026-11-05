mod common;

use common::*;
use myosotis_app::features::settings;
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{SessionCard, Settings, Theme};

#[tokio::test]
async fn a_fresh_app_uses_twenty_new_cards_and_ninety_percent_retention() {
    // given a new installation
    let repo: SqliteCardRepo = empty_collection().await;

    // when I open Settings
    let current: Settings = settings::load(&repo).await.unwrap();

    // then the defaults apply
    assert_eq!(current.new_cards_per_day, 20);
    assert_eq!(current.desired_retention_percent, 90);
    assert_eq!(current.theme, Theme::System);
}

#[tokio::test]
async fn the_new_card_limit_can_be_changed() {
    // given 10 new cards and a limit of 3
    let repo: SqliteCardRepo = collection_with_cards(10).await;
    let changed: Settings = Settings {
        new_cards_per_day: 3,
        ..Settings::default()
    };
    settings::update(&repo, changed).await.unwrap();

    // when I start a session
    let session: Vec<SessionCard> = session_at(&repo, monday()).await;

    // then only 3 are offered
    assert_eq!(session.len(), 3);
}

#[tokio::test]
async fn higher_desired_retention_brings_cards_back_sooner() {
    // given the Easy interval at the default retention
    let repo: SqliteCardRepo = collection_with_cards(1).await;
    let default: Vec<SessionCard> = session_at(&repo, monday()).await;

    // when I raise retention to 97%
    let stricter: Settings = Settings {
        desired_retention_percent: 97,
        ..Settings::default()
    };
    settings::update(&repo, stricter).await.unwrap();
    let raised: Vec<SessionCard> = session_at(&repo, monday()).await;

    // then Easy returns sooner than before
    assert_eq!(default[0].grades[3].interval, "15d");
    assert_eq!(raised[0].grades[3].interval, "4d");
}

#[tokio::test]
async fn retention_stays_within_the_sensible_range() {
    // given an empty collection
    let repo: SqliteCardRepo = empty_collection().await;

    // when retention is set far too high
    let wild: Settings = Settings {
        desired_retention_percent: 150,
        ..Settings::default()
    };
    let saved: Settings = settings::update(&repo, wild).await.unwrap();

    // then it is capped at 97%
    assert_eq!(saved.desired_retention_percent, 97);
}

#[tokio::test]
async fn settings_are_remembered() {
    // given I chose the dark theme
    let repo: SqliteCardRepo = empty_collection().await;
    let dark: Settings = Settings {
        theme: Theme::Dark,
        ..Settings::default()
    };
    settings::update(&repo, dark).await.unwrap();

    // when Settings is opened again
    let current: Settings = settings::load(&repo).await.unwrap();

    // then the theme is still dark
    assert_eq!(current.theme, Theme::Dark);
}
