use std::error::Error;
use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::review::Schedule;

pub type CardId = i64;

/// How a card is reviewed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardKind {
    /// A question with a short answer.
    #[default]
    Question,
    /// A term and its meaning, reviewed both ways round.
    Concept,
    /// An open question answered in your own words, marked by the AI.
    Describe,
}

impl CardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CardKind::Question => "question",
            CardKind::Concept => "concept",
            CardKind::Describe => "describe",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "concept" => CardKind::Concept,
            "describe" => CardKind::Describe,
            _ => CardKind::Question,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Card {
    pub question: String,
    pub answer: String,
}

impl Card {
    pub fn new(question: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            answer: answer.into(),
        }
    }
}

/// A card I write myself rather than import.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewCard {
    pub kind: CardKind,
    pub question: String,
    pub answer: String,
}

impl NewCard {
    /// Without surrounding whitespace; refused when the question or answer is blank.
    pub fn tidied(&self) -> Result<NewCard, AddCardError> {
        let question: &str = self.question.trim();
        let answer: &str = self.answer.trim();
        if question.is_empty() || answer.is_empty() {
            return Err(AddCardError::Blank);
        }
        Ok(NewCard {
            kind: self.kind,
            question: question.to_string(),
            answer: answer.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AddCardError {
    #[error("A card needs both a question and an answer.")]
    Blank,
    #[error(transparent)]
    Repo(#[from] RepoError),
}

/// Port for card storage. Adapters (e.g. SQLite) implement it outside core.
pub trait CardRepo {
    fn list(&self) -> impl Future<Output = Result<Vec<Card>, RepoError>> + Send;

    /// Stores an active card, ready for review; a concept also gets its other direction.
    fn add(
        &self,
        card: &NewCard,
        schedule: &Schedule,
    ) -> impl Future<Output = Result<CardId, RepoError>> + Send;
}

/// Storage failed. Wraps the adapter's own error so core stays I/O-agnostic.
#[derive(Debug, thiserror::Error)]
#[error("card storage failed")]
pub struct RepoError(#[source] pub Box<dyn Error + Send + Sync>);
