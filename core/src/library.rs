use std::future::Future;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::card::{CardId, RepoError};
use crate::review::DueCard;

/// A card as storage hands it to the Library.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredCard {
    pub due: DueCard,
    pub suspended: bool,
    pub review_count: i64,
}

/// A card as the Library shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LibraryCard {
    pub id: CardId,
    pub question: String,
    pub answer: String,
    pub source_title: Option<String>,
    pub excerpt: Option<String>,
    pub is_new: bool,
    pub suspended: bool,
    pub review_count: i64,
    pub recall_percent: Option<u8>,
    pub next_due: DateTime<Utc>,
}

impl StoredCard {
    pub fn into_library_card(self, now: DateTime<Utc>) -> LibraryCard {
        LibraryCard {
            is_new: self.due.schedule.is_new(),
            recall_percent: self.due.schedule.recall_percent(now),
            next_due: self.due.schedule.due,
            id: self.due.id,
            question: self.due.card.question,
            answer: self.due.card.answer,
            source_title: self.due.source_title,
            excerpt: self.due.excerpt,
            suspended: self.suspended,
            review_count: self.review_count,
        }
    }
}

/// Port for everything the Library needs from storage.
pub trait LibraryRepo {
    fn all_cards(&self) -> impl Future<Output = Result<Vec<StoredCard>, RepoError>> + Send;

    fn set_suspended(
        &self,
        card: CardId,
        suspended: bool,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    /// Deletes a card and its review history. The UI must confirm first.
    fn delete_card(&self, card: CardId) -> impl Future<Output = Result<(), RepoError>> + Send;
}
