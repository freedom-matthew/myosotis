use std::future::Future;

use serde::Serialize;

use crate::card::{Card, CardId, RepoError};
use crate::library::StoredCard;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TidyMember {
    pub id: CardId,
    pub question: String,
    pub answer: String,
    pub source_title: Option<String>,
    pub review_count: i64,
    pub keeps_history: bool,
}

/// Similar cards, with the merge being proposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TidyGroup {
    pub members: Vec<TidyMember>,
    pub suggested_question: String,
    pub suggested_answer: String,
}

/// The member with the most reviews keeps its history, and its wording is the suggestion.
pub fn tidy_group(members: Vec<StoredCard>) -> TidyGroup {
    let keeper: CardId = members
        .iter()
        .max_by_key(|member| member.review_count)
        .map_or(0, |member| member.due.id);
    let (suggested_question, suggested_answer): (String, String) = members
        .iter()
        .find(|member| member.due.id == keeper)
        .map(|member| {
            (
                member.due.card.question.clone(),
                member.due.card.answer.clone(),
            )
        })
        .unwrap_or_default();

    TidyGroup {
        suggested_question,
        suggested_answer,
        members: members
            .into_iter()
            .map(|member| TidyMember {
                keeps_history: member.due.id == keeper,
                id: member.due.id,
                question: member.due.card.question,
                answer: member.due.card.answer,
                source_title: member.due.source_title,
                review_count: member.review_count,
            })
            .collect(),
    }
}

/// Port for what Tidy up needs from storage, beyond the Library's cards.
pub trait TidyRepo {
    fn not_duplicate_pairs(
        &self,
    ) -> impl Future<Output = Result<Vec<(CardId, CardId)>, RepoError>> + Send;

    fn remember_not_duplicates(
        &self,
        cards: &[CardId],
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    /// Folds `others` into `keeper`: all sources move across, and `keeper` keeps its reviews.
    fn merge_cards(
        &self,
        keeper: CardId,
        others: &[CardId],
        wording: &Card,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;
}
