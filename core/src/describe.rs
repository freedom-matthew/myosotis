use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::import::GenerateError;
use crate::review::Grade;

/// What the AI receives to mark a Describe card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkingRequest {
    pub question: String,
    pub model_answer: String,
    pub excerpt: Option<String>,
    pub description: String,
}

/// How well a description matched the model answer. The grade is only a suggestion;
/// the reviewer still decides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marking {
    pub covered: Vec<String>,
    pub missing: Vec<String>,
    pub suggested_grade: Grade,
}

/// Port for whatever marks a description during review.
pub trait CardMarker {
    fn mark(
        &self,
        request: &MarkingRequest,
    ) -> impl Future<Output = Result<Marking, GenerateError>> + Send;
}
