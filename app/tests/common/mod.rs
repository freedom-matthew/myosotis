//! Small helpers so the acceptance tests read like requirements.
#![allow(dead_code)]

use std::sync::Mutex;

use chrono::{DateTime, Duration, TimeZone, Utc};
use myosotis_app::features::{import_source, inbox, review};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{
    Card, CardExplainer, CardGenerator, Depth, DraftCard, ExplanationRequest, GenerateError,
    GeneratedCard, GenerationRequest, Grade, ImportId, InboxImport, NewImport, SessionCard,
};

/// Monday morning. Every test starts here, so nothing depends on the real clock.
pub fn monday() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 2, 9, 0, 0).unwrap()
}

pub fn minutes_later(time: DateTime<Utc>, minutes: i64) -> DateTime<Utc> {
    time + Duration::minutes(minutes)
}

pub fn days_later(time: DateTime<Utc>, days: i64) -> DateTime<Utc> {
    time + Duration::days(days)
}

pub async fn empty_collection() -> SqliteCardRepo {
    SqliteCardRepo::in_memory().await.unwrap()
}

pub async fn collection_with_cards(count: usize) -> SqliteCardRepo {
    let repo: SqliteCardRepo = empty_collection().await;
    for number in 1..=count {
        let card: Card = Card::new(format!("Question {number}"), format!("Answer {number}"));
        repo.insert(&card).await.unwrap();
    }
    repo
}

/// The cards a review session would show at this moment.
pub async fn session_at(repo: &SqliteCardRepo, now: DateTime<Utc>) -> Vec<SessionCard> {
    let start_of_day: DateTime<Utc> = now - Duration::hours(1);
    review::session(repo, now, start_of_day).await.unwrap()
}

pub async fn grade(repo: &SqliteCardRepo, card: &SessionCard, grade: Grade, now: DateTime<Utc>) {
    review::grade(repo, card.id, grade, now).await.unwrap();
}

pub fn questions(cards: &[SessionCard]) -> Vec<&str> {
    cards.iter().map(|card| card.question.as_str()).collect()
}

/// Stands in for the AI: returns the cards it was given, or fails.
pub struct FakeGenerator {
    pub result: Result<Vec<GeneratedCard>, GenerateError>,
}

impl FakeGenerator {
    pub fn producing(questions: &[&str]) -> Self {
        let cards: Vec<GeneratedCard> = questions
            .iter()
            .map(|question| GeneratedCard {
                question: question.to_string(),
                answer: format!("Answer to: {question}"),
                excerpt: format!("Passage about: {question}"),
            })
            .collect();
        Self { result: Ok(cards) }
    }

    pub fn failing_with(message: &str) -> Self {
        Self {
            result: Err(GenerateError(message.to_string())),
        }
    }
}

impl CardGenerator for FakeGenerator {
    async fn generate(
        &self,
        _request: &GenerationRequest,
    ) -> Result<Vec<GeneratedCard>, GenerateError> {
        self.result.clone()
    }
}

/// Stands in for the AI when explaining: remembers what it was asked.
pub struct FakeExplainer {
    result: Result<String, GenerateError>,
    requests: Mutex<Vec<ExplanationRequest>>,
}

impl FakeExplainer {
    pub fn replying(text: &str) -> Self {
        Self {
            result: Ok(text.to_string()),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub fn failing_with(message: &str) -> Self {
        Self {
            result: Err(GenerateError(message.to_string())),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub fn last_request(&self) -> ExplanationRequest {
        self.requests.lock().unwrap().last().unwrap().clone()
    }
}

impl CardExplainer for FakeExplainer {
    async fn explain(&self, request: &ExplanationRequest) -> Result<String, GenerateError> {
        self.requests.lock().unwrap().push(request.clone());
        self.result.clone()
    }
}

pub async fn import_source_with(repo: &SqliteCardRepo, generator: &FakeGenerator) -> ImportId {
    let source: NewImport = NewImport {
        title: "Habit notes".to_string(),
        text: "Habits form through a loop of cue, routine and reward.".to_string(),
        depth: Depth::Balanced,
        focus: None,
    };
    let import: ImportId = import_source::create(repo, &source, monday())
        .await
        .unwrap();
    import_source::generate(repo, generator, import)
        .await
        .unwrap();
    import
}

pub async fn the_inbox(repo: &SqliteCardRepo) -> Vec<InboxImport> {
    inbox::list(repo).await.unwrap()
}

pub async fn drafts_in_inbox(repo: &SqliteCardRepo) -> Vec<DraftCard> {
    the_inbox(repo)
        .await
        .into_iter()
        .flat_map(|import| import.drafts)
        .collect()
}
