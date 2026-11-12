//! Small helpers so the acceptance tests read like requirements.
#![allow(dead_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Duration, TimeZone, Utc};
use myosotis_app::features::{import_source, inbox, review};
use myosotis_app::storage::SqliteCardRepo;
use myosotis_core::{
    Card, CardExplainer, CardGenerator, CardKind, CardMarker, CardRewriter, Depth, DraftCard,
    ExplanationRequest, GenerateError, GeneratedCard, GenerationRequest, Grade, ImportId,
    InboxImport, Marking, MarkingRequest, NewImport, Reverse, RewriteRequest, SessionCard,
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
    calls: AtomicUsize,
    fails_on_call: Option<usize>,
}

impl FakeGenerator {
    pub fn with_cards(cards: Vec<GeneratedCard>) -> Self {
        Self {
            result: Ok(cards),
            calls: AtomicUsize::new(0),
            fails_on_call: None,
        }
    }

    pub fn producing(questions: &[&str]) -> Self {
        let cards: Vec<GeneratedCard> = questions
            .iter()
            .map(|question| GeneratedCard {
                kind: CardKind::Question,
                question: question.to_string(),
                answer: format!("Answer to: {question}"),
                excerpt: format!("Passage about: {question}"),
                reverse: None,
            })
            .collect();
        Self::with_cards(cards)
    }

    pub fn failing_with(message: &str) -> Self {
        Self {
            result: Err(GenerateError(message.to_string())),
            calls: AtomicUsize::new(0),
            fails_on_call: None,
        }
    }

    /// Works, except on this call (counted from 1), as if the AI dropped out midway.
    pub fn failing_on_call(self, call: usize) -> Self {
        Self {
            fails_on_call: Some(call),
            ..self
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl CardGenerator for FakeGenerator {
    async fn generate(
        &self,
        _request: &GenerationRequest,
    ) -> Result<Vec<GeneratedCard>, GenerateError> {
        let call: usize = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if self.fails_on_call == Some(call) {
            return Err(GenerateError("The AI dropped out".to_string()));
        }
        self.result.clone()
    }
}

pub struct FakeRewriter {
    pub better: Card,
}

impl CardRewriter for FakeRewriter {
    async fn rewrite(&self, _request: &RewriteRequest) -> Result<Card, GenerateError> {
        Ok(self.better.clone())
    }
}

/// A rewriter that can't reach the AI.
pub struct UnavailableRewriter;

impl CardRewriter for UnavailableRewriter {
    async fn rewrite(&self, _request: &RewriteRequest) -> Result<Card, GenerateError> {
        Err(GenerateError("Claude Code is not signed in".to_string()))
    }
}

/// The AI made a concept with both directions as questions, and I accepted it.
pub async fn accepted_concept(
    repo: &SqliteCardRepo,
    question: &str,
    answer: &str,
    reverse: Reverse,
) -> Vec<SessionCard> {
    let card: GeneratedCard = GeneratedCard {
        kind: CardKind::Concept,
        question: question.to_string(),
        answer: answer.to_string(),
        excerpt: "A passage.".to_string(),
        reverse: Some(reverse),
    };
    import_source_with(repo, &FakeGenerator::with_cards(vec![card])).await;
    for draft in drafts_in_inbox(repo).await {
        inbox::accept(repo, draft.id, monday()).await.unwrap();
    }
    session_at(repo, monday()).await
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

/// Stands in for the AI when marking a description: remembers what it was asked.
pub struct FakeMarker {
    result: Result<Marking, GenerateError>,
    requests: Mutex<Vec<MarkingRequest>>,
}

impl FakeMarker {
    pub fn replying(marking: Marking) -> Self {
        Self {
            result: Ok(marking),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub fn failing_with(message: &str) -> Self {
        Self {
            result: Err(GenerateError(message.to_string())),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub fn last_request(&self) -> MarkingRequest {
        self.requests.lock().unwrap().last().unwrap().clone()
    }
}

impl CardMarker for FakeMarker {
    async fn mark(&self, request: &MarkingRequest) -> Result<Marking, GenerateError> {
        self.requests.lock().unwrap().push(request.clone());
        self.result.clone()
    }
}

/// An accepted card of this kind, from a source with this passage.
pub async fn accepted_card(
    repo: &SqliteCardRepo,
    kind: CardKind,
    question: &str,
    answer: &str,
) -> Vec<SessionCard> {
    let card: GeneratedCard = GeneratedCard {
        kind,
        question: question.to_string(),
        answer: answer.to_string(),
        excerpt: "A passage.".to_string(),
        reverse: None,
    };
    import_source_with(repo, &FakeGenerator::with_cards(vec![card])).await;
    for draft in drafts_in_inbox(repo).await {
        inbox::accept(repo, draft.id, monday()).await.unwrap();
    }
    session_at(repo, monday()).await
}

pub async fn import_source_with(repo: &SqliteCardRepo, generator: &FakeGenerator) -> ImportId {
    let text: &str = "Habits form through a loop of cue, routine and reward.";
    import_text_with(repo, generator, text).await
}

/// A source long enough to be worked through in three parts.
pub fn three_part_source() -> String {
    let paragraph: String = "A sentence that fills the page. ".repeat(800);
    [paragraph.as_str(); 3].join("\n\n")
}

pub async fn import_text_with(
    repo: &SqliteCardRepo,
    generator: &FakeGenerator,
    text: &str,
) -> ImportId {
    let source: NewImport = NewImport {
        title: "Habit notes".to_string(),
        text: text.to_string(),
        depth: Depth::Balanced,
        focus: None,
        describe: true,
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
