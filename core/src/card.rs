use std::error::Error;
use std::future::Future;

use serde::Serialize;

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
