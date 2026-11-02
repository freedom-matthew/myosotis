use myosotis_app::features::list_cards;
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::Card;

#[tokio::test]
async fn lists_the_card_stored_in_the_database() {
    // given a database containing one card
    let repo: SqliteCardRepo = SqliteCardRepo::in_memory().await.unwrap();
    let card: Card = Card::new("What is Rust?", "A systems programming language");
    repo.insert(&card).await.unwrap();

    // when I list cards
    let cards: Vec<Card> = list_cards::handle(&repo).await.unwrap();

    // then I get exactly that card back
    assert_eq!(cards, vec![card]);
}
