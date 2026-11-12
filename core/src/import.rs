use std::future::Future;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::card::{Card, CardId, CardKind, RepoError};
use crate::review::Schedule;

pub type ImportId = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Depth {
    Essentials,
    Balanced,
    Thorough,
}

impl Depth {
    pub fn target_cards(self) -> u32 {
        match self {
            Depth::Essentials => 6,
            Depth::Balanced => 14,
            Depth::Thorough => 30,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Depth::Essentials => "essentials",
            Depth::Balanced => "balanced",
            Depth::Thorough => "thorough",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "essentials" => Depth::Essentials,
            "thorough" => Depth::Thorough,
            _ => Depth::Balanced,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewImport {
    pub title: String,
    pub text: String,
    pub depth: Depth,
    pub focus: Option<String>,
    /// Whether the AI may also make Describe cards.
    pub describe: bool,
}

/// How many words of the text label an import that has no name.
const LABEL_WORDS: usize = 6;

impl NewImport {
    /// What the Inbox calls this import while its cards are being checked: the given name
    /// (a file's name), or else the text's first few words. Nobody has to name a source.
    pub fn label(&self) -> String {
        let name: &str = self.title.trim();
        if !name.is_empty() {
            return name.to_string();
        }
        let words: Vec<&str> = self.text.split_whitespace().collect();
        let start: String = words
            .iter()
            .take(LABEL_WORDS)
            .copied()
            .collect::<Vec<&str>>()
            .join(" ");
        if words.len() > LABEL_WORDS {
            format!("{start}…")
        } else {
            start
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Generating,
    InInbox,
    Failed,
    Done,
}

impl ImportStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ImportStatus::Generating => "generating",
            ImportStatus::InInbox => "in_inbox",
            ImportStatus::Failed => "failed",
            ImportStatus::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "generating" => ImportStatus::Generating,
            "failed" => ImportStatus::Failed,
            "done" => ImportStatus::Done,
            _ => ImportStatus::InInbox,
        }
    }
}

/// What the AI receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationRequest {
    pub title: String,
    pub text: String,
    pub target_cards: u32,
    pub focus: Option<String>,
    pub describe: bool,
}

/// What the AI returns for one card.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GeneratedCard {
    #[serde(default)]
    pub kind: CardKind,
    pub question: String,
    pub answer: String,
    pub excerpt: String,
}

/// Splits a long source at paragraph breaks, so each part fits one AI call.
pub fn split_source(text: &str, max_chars: usize) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current: String = String::new();
    for paragraph in text.split("\n\n") {
        if !current.is_empty() && current.chars().count() + paragraph.chars().count() > max_chars {
            parts.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(paragraph);
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct GenerateError(pub String);

/// What the AI receives to improve one card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteRequest {
    pub kind: CardKind,
    pub question: String,
    pub answer: String,
    pub excerpt: Option<String>,
}

/// Port for whatever rewrites a card from its original passage.
pub trait CardRewriter {
    fn rewrite(
        &self,
        request: &RewriteRequest,
    ) -> impl Future<Output = Result<Card, GenerateError>> + Send;
}

/// Port for whatever creates cards from a source.
pub trait CardGenerator {
    fn generate(
        &self,
        request: &GenerationRequest,
    ) -> impl Future<Output = Result<Vec<GeneratedCard>, GenerateError>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftCard {
    pub id: CardId,
    pub kind: CardKind,
    pub question: String,
    pub answer: String,
    pub excerpt: Option<String>,
    pub duplicate: Option<ExistingCard>,
}

/// The card a draft resembles, shown side by side with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExistingCard {
    pub id: CardId,
    pub question: String,
    pub answer: String,
    pub source_title: Option<String>,
    pub review_count: i64,
}

/// A generated card on its way into the Inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftToStore {
    pub card: GeneratedCard,
    pub duplicate_of: Option<CardId>,
}

/// A source with cards still to check, as the Inbox shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InboxImport {
    pub id: ImportId,
    pub title: String,
    pub status: ImportStatus,
    pub error: Option<String>,
    pub text: Option<String>,
    pub parts_total: i64,
    pub parts_done: i64,
    pub drafts: Vec<DraftCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecentImport {
    pub id: ImportId,
    pub title: String,
    pub status: ImportStatus,
    pub created_at: DateTime<Utc>,
    pub card_count: i64,
    pub draft_count: i64,
}

/// Port for everything importing and the Inbox need from storage.
pub trait ImportRepo {
    fn create_import(
        &self,
        import: &NewImport,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<ImportId, RepoError>> + Send;

    fn generation_request(
        &self,
        import: ImportId,
    ) -> impl Future<Output = Result<GenerationRequest, RepoError>> + Send;

    fn mark_generating(
        &self,
        import: ImportId,
        parts_total: usize,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn parts_done(&self, import: ImportId)
    -> impl Future<Output = Result<usize, RepoError>> + Send;

    fn is_generating(
        &self,
        import: ImportId,
    ) -> impl Future<Output = Result<bool, RepoError>> + Send;

    /// Saves the cards of one finished part, so they appear in the Inbox straight away.
    fn store_drafts(
        &self,
        import: ImportId,
        cards: &[DraftToStore],
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn draft_count(&self, import: ImportId) -> impl Future<Output = Result<i64, RepoError>> + Send;

    /// Marks generation as finished, keeping the cards made so far.
    fn mark_ready(&self, import: ImportId) -> impl Future<Output = Result<(), RepoError>> + Send;

    /// Moves an import that failed partway on to the Inbox with the cards it has.
    fn mark_generating_finished(
        &self,
        import: ImportId,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn mark_failed(
        &self,
        import: ImportId,
        message: &str,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn draft(&self, card: CardId) -> impl Future<Output = Result<DraftCard, RepoError>> + Send;

    /// The text of every card and draft, for spotting duplicates.
    fn known_card_texts(
        &self,
    ) -> impl Future<Output = Result<Vec<(CardId, String)>, RepoError>> + Send;

    /// Treats a suspected duplicate as a normal card.
    fn clear_duplicate(&self, card: CardId) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn inbox(&self) -> impl Future<Output = Result<Vec<InboxImport>, RepoError>> + Send;

    fn drafts_of(
        &self,
        import: ImportId,
    ) -> impl Future<Output = Result<Vec<CardId>, RepoError>> + Send;

    /// Turns a draft into a normal card, due from `schedule`. Returns its import.
    fn accept_draft(
        &self,
        card: CardId,
        schedule: &Schedule,
    ) -> impl Future<Output = Result<ImportId, RepoError>> + Send;

    fn update_draft(
        &self,
        card: CardId,
        content: &Card,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    /// Deletes a draft, never a card that was accepted. Returns its import.
    fn delete_draft(
        &self,
        card: CardId,
    ) -> impl Future<Output = Result<ImportId, RepoError>> + Send;

    /// Marks the import done and deletes its full text if no drafts remain.
    fn finish_import_if_empty(
        &self,
        import: ImportId,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    /// Removes a failed import that produced no cards, so nothing can be lost.
    fn discard_failed_import(
        &self,
        import: ImportId,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn recent_imports(
        &self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<RecentImport>, RepoError>> + Send;

    fn inbox_count(&self) -> impl Future<Output = Result<i64, RepoError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_sources_split_at_paragraphs_and_keep_all_the_text() {
        let text: String = ["aaaa", "bbbb", "cccc"].join("\n\n");
        let parts: Vec<String> = split_source(&text, 10);
        assert_eq!(parts, vec!["aaaa\n\nbbbb", "cccc"]);
    }

    #[test]
    fn a_short_source_stays_in_one_part() {
        assert_eq!(split_source("short", 100), vec!["short"]);
    }

    #[test]
    fn one_huge_paragraph_is_kept_rather_than_lost() {
        let text: String = "x".repeat(50);
        assert_eq!(split_source(&text, 10), vec![text.clone()]);
    }
}
