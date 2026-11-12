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
    /// A term and its meaning, reviewed both ways round: each direction is its own question.
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

/// The other direction of a concept, written as its own question: from the meaning back to
/// the term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reverse {
    pub question: String,
    pub answer: String,
}

impl Reverse {
    pub fn new(question: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            answer: answer.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Card {
    pub question: String,
    pub answer: String,
    /// Only a concept has one.
    pub reverse: Option<Reverse>,
}

impl Card {
    pub fn new(question: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            answer: answer.into(),
            reverse: None,
        }
    }

    pub fn concept(
        question: impl Into<String>,
        answer: impl Into<String>,
        reverse: Reverse,
    ) -> Self {
        Self {
            reverse: Some(reverse),
            ..Self::new(question, answer)
        }
    }
}

/// A card I write myself rather than import.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewCard {
    pub kind: CardKind,
    pub question: String,
    pub answer: String,
    /// A concept's other direction; ignored for other kinds.
    #[serde(default)]
    pub reverse: Option<Reverse>,
}

impl NewCard {
    /// Without surrounding whitespace; refused when a question or answer is blank.
    pub fn tidied(&self) -> Result<NewCard, AddCardError> {
        let (question, answer): (String, String) = tidy_pair(&self.question, &self.answer)?;
        let reverse: Option<Reverse> = match (self.kind, &self.reverse) {
            (CardKind::Concept, Some(reverse)) => {
                let (question, answer): (String, String) =
                    tidy_pair(&reverse.question, &reverse.answer)?;
                Some(Reverse { question, answer })
            }
            (CardKind::Concept, None) => return Err(AddCardError::Blank),
            _ => None,
        };
        Ok(NewCard {
            kind: self.kind,
            question,
            answer,
            reverse,
        })
    }
}

fn tidy_pair(question: &str, answer: &str) -> Result<(String, String), AddCardError> {
    let question: &str = question.trim();
    let answer: &str = answer.trim();
    if question.is_empty() || answer.is_empty() {
        return Err(AddCardError::Blank);
    }
    Ok((question.to_string(), answer.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum AddCardError {
    #[error("Every question needs an answer, and a concept needs both directions.")]
    Blank,
    #[error(transparent)]
    Repo(#[from] RepoError),
}

/// A concept pair in the old shape: `term` on the front of one card, `meaning` on the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OldConcept {
    pub forward: CardId,
    pub reverse: CardId,
    pub term: String,
    pub meaning: String,
    pub excerpt: Option<String>,
}

/// Port for card storage. Adapters (e.g. SQLite) implement it outside core.
pub trait CardRepo {
    fn list(&self) -> impl Future<Output = Result<Vec<Card>, RepoError>> + Send;

    /// Concepts from before both directions were written as questions: one side is simply
    /// the other swapped round, so the front is a bare term.
    fn old_concepts(&self) -> impl Future<Output = Result<Vec<OldConcept>, RepoError>> + Send;

    /// Rewords both directions of an old concept, keeping their review history. Leaves it
    /// alone if either side was edited in the meantime.
    fn reword_concept(
        &self,
        old: &OldConcept,
        card: &Card,
        reverse: &Reverse,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

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
