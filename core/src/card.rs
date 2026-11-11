use std::error::Error;
use std::future::Future;

use serde::{Deserialize, Serialize};

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

/// Port for card storage. Adapters (e.g. SQLite) implement it outside core.
pub trait CardRepo {
    fn list(&self) -> impl Future<Output = Result<Vec<Card>, RepoError>> + Send;
}

/// Storage failed. Wraps the adapter's own error so core stays I/O-agnostic.
#[derive(Debug, thiserror::Error)]
#[error("card storage failed")]
pub struct RepoError(#[source] pub Box<dyn Error + Send + Sync>);
