use std::future::Future;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::card::{Card, CardId, RepoError};
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
}

/// What the AI returns for one card.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GeneratedCard {
    pub question: String,
    pub answer: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct GenerateError(pub String);

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
    pub question: String,
    pub answer: String,
    pub excerpt: Option<String>,
}

/// A source with cards still to check, as the Inbox shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InboxImport {
    pub id: ImportId,
    pub title: String,
    pub status: ImportStatus,
    pub error: Option<String>,
    pub text: Option<String>,
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
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn store_drafts(
        &self,
        import: ImportId,
        cards: &[GeneratedCard],
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn mark_failed(
        &self,
        import: ImportId,
        message: &str,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

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

    /// Removes a failed import. It has no cards, so nothing learned can be lost.
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
