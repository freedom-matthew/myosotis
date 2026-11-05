use std::future::Future;

use crate::import::GenerateError;

/// What the AI receives to explain a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationRequest {
    pub question: String,
    pub answer: String,
    pub excerpt: Option<String>,
}

/// Port for whatever explains a card during review.
pub trait CardExplainer {
    fn explain(
        &self,
        request: &ExplanationRequest,
    ) -> impl Future<Output = Result<String, GenerateError>> + Send;
}
