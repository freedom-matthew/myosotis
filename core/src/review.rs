use std::future::Future;

use chrono::{DateTime, Duration, Utc};
use rs_fsrs::{FSRS, Parameters, Rating, SchedulingInfo, State};
use serde::{Deserialize, Serialize};

use crate::card::{Card, CardId, CardKind, RepoError};
use crate::explain::ExplanationRequest;
use crate::settings::Settings;

/// Cards that return within this window are shown now rather than ending the session.
const LEARN_AHEAD_MINUTES: i64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

impl Grade {
    pub const ALL: [Grade; 4] = [Grade::Again, Grade::Hard, Grade::Good, Grade::Easy];

    pub fn as_str(self) -> &'static str {
        match self {
            Grade::Again => "again",
            Grade::Hard => "hard",
            Grade::Good => "good",
            Grade::Easy => "easy",
        }
    }

    fn rating(self) -> Rating {
        match self {
            Grade::Again => Rating::Again,
            Grade::Hard => Rating::Hard,
            Grade::Good => Rating::Good,
            Grade::Easy => Rating::Easy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    New,
    Learning,
    Review,
    Relearning,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::New => "new",
            Phase::Learning => "learning",
            Phase::Review => "review",
            Phase::Relearning => "relearning",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "learning" => Phase::Learning,
            "review" => Phase::Review,
            "relearning" => Phase::Relearning,
            _ => Phase::New,
        }
    }
}

/// Everything FSRS remembers about one card. Owned here so stored data doesn't depend on
/// the FSRS crate's types.
#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {
    pub due: DateTime<Utc>,
    pub stability: f64,
    pub difficulty: f64,
    pub elapsed_days: i64,
    pub scheduled_days: i64,
    pub reps: i32,
    pub lapses: i32,
    pub phase: Phase,
    pub last_review: DateTime<Utc>,
}

impl Schedule {
    pub fn new_card(now: DateTime<Utc>) -> Self {
        Self {
            due: now,
            stability: 0.0,
            difficulty: 0.0,
            elapsed_days: 0,
            scheduled_days: 0,
            reps: 0,
            lapses: 0,
            phase: Phase::New,
            last_review: now,
        }
    }

    pub fn is_new(&self) -> bool {
        self.phase == Phase::New
    }

    /// The chance of remembering the card right now, once it has been reviewed.
    pub fn recall_percent(&self, now: DateTime<Utc>) -> Option<u8> {
        if self.is_new() {
            return None;
        }
        Some((self.to_fsrs().get_retrievability(now) * 100.0).round() as u8)
    }

    pub fn after(&self, grade: Grade, now: DateTime<Utc>, retention: f64) -> Schedule {
        let info: SchedulingInfo = scheduler(retention).next(self.to_fsrs(), now, grade.rating());
        Schedule::from_fsrs(&info.card)
    }

    pub fn previews(&self, now: DateTime<Utc>, retention: f64) -> Vec<GradePreview> {
        Grade::ALL
            .iter()
            .map(|grade| GradePreview {
                grade: *grade,
                interval: format_interval(self.after(*grade, now, retention).due - now),
            })
            .collect()
    }

    fn to_fsrs(&self) -> rs_fsrs::Card {
        rs_fsrs::Card {
            due: self.due,
            stability: self.stability,
            difficulty: self.difficulty,
            elapsed_days: self.elapsed_days,
            scheduled_days: self.scheduled_days,
            reps: self.reps,
            lapses: self.lapses,
            state: match self.phase {
                Phase::New => State::New,
                Phase::Learning => State::Learning,
                Phase::Review => State::Review,
                Phase::Relearning => State::Relearning,
            },
            last_review: self.last_review,
        }
    }

    fn from_fsrs(card: &rs_fsrs::Card) -> Self {
        Self {
            due: card.due,
            stability: card.stability,
            difficulty: card.difficulty,
            elapsed_days: card.elapsed_days,
            scheduled_days: card.scheduled_days,
            reps: card.reps,
            lapses: card.lapses,
            phase: match card.state {
                State::New => Phase::New,
                State::Learning => Phase::Learning,
                State::Review => Phase::Review,
                State::Relearning => Phase::Relearning,
            },
            last_review: card.last_review,
        }
    }
}

fn scheduler(retention: f64) -> FSRS {
    FSRS::new(Parameters {
        request_retention: retention,
        ..Parameters::default()
    })
}

fn format_interval(interval: Duration) -> String {
    let minutes: i64 = interval.num_minutes();
    let days: i64 = interval.num_days();
    match () {
        _ if minutes < 60 => format!("{}m", minutes.max(1)),
        _ if days < 1 => format!("{}h", interval.num_hours()),
        _ if days < 31 => format!("{days}d"),
        _ if days < 365 => format!("{}mo", days / 30),
        _ => format!("{:.1}y", days as f64 / 365.0),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GradePreview {
    pub grade: Grade,
    pub interval: String,
}

/// A card that could be reviewed, as storage hands it over.
#[derive(Debug, Clone, PartialEq)]
pub struct DueCard {
    pub id: CardId,
    pub kind: CardKind,
    pub sibling: Option<CardId>,
    pub card: Card,
    pub source_title: Option<String>,
    pub excerpt: Option<String>,
    pub schedule: Schedule,
}

/// A card as the review screen shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionCard {
    pub id: CardId,
    pub kind: CardKind,
    pub question: String,
    pub answer: String,
    pub source_title: Option<String>,
    pub excerpt: Option<String>,
    pub is_new: bool,
    pub grades: Vec<GradePreview>,
}

/// Decides what a session shows: reviews that are due, then new cards up to the daily
/// limit, then cards coming back within minutes.
pub fn plan_session(
    cards: Vec<DueCard>,
    new_started_today: usize,
    reviewed_today: &[CardId],
    settings: &Settings,
    now: DateTime<Utc>,
) -> Vec<SessionCard> {
    let new_allowed: usize =
        (settings.new_cards_per_day as usize).saturating_sub(new_started_today);
    let retention: f64 = settings.desired_retention();
    let cards: Vec<DueCard> = apart_from_siblings(cards, reviewed_today);
    let (new_cards, seen_cards): (Vec<DueCard>, Vec<DueCard>) =
        cards.into_iter().partition(|card| card.schedule.is_new());
    let (reviews, coming_back): (Vec<DueCard>, Vec<DueCard>) = seen_cards
        .into_iter()
        .partition(|card| card.schedule.due <= now);

    reviews
        .into_iter()
        .chain(new_cards.into_iter().take(new_allowed))
        .chain(coming_back)
        .map(|card| to_session_card(card, retention, now))
        .collect()
}

/// Keeps the two directions of a concept card out of the same day, because seeing one
/// gives away the other.
fn apart_from_siblings(cards: Vec<DueCard>, reviewed_today: &[CardId]) -> Vec<DueCard> {
    let mut taken: Vec<CardId> = reviewed_today.to_vec();
    let mut kept: Vec<DueCard> = Vec::with_capacity(cards.len());
    for card in cards {
        if card.sibling.is_some_and(|sibling| taken.contains(&sibling)) {
            continue;
        }
        taken.push(card.id);
        kept.push(card);
    }
    kept
}

/// How far ahead storage must look so `plan_session` can see cards coming back soon.
pub fn session_horizon(now: DateTime<Utc>) -> DateTime<Utc> {
    now + Duration::minutes(LEARN_AHEAD_MINUTES)
}

fn to_session_card(due: DueCard, retention: f64, now: DateTime<Utc>) -> SessionCard {
    SessionCard {
        id: due.id,
        kind: due.kind,
        is_new: due.schedule.is_new(),
        grades: due.schedule.previews(now, retention),
        question: due.card.question,
        answer: due.card.answer,
        source_title: due.source_title,
        excerpt: due.excerpt,
    }
}

/// Port for everything reviewing needs from storage.
pub trait ReviewRepo {
    /// Active cards due by `until`, oldest due first.
    fn due_cards(
        &self,
        until: DateTime<Utc>,
    ) -> impl Future<Output = Result<Vec<DueCard>, RepoError>> + Send;

    fn new_cards_started_since(
        &self,
        since: DateTime<Utc>,
    ) -> impl Future<Output = Result<usize, RepoError>> + Send;

    fn cards_reviewed_since(
        &self,
        since: DateTime<Utc>,
    ) -> impl Future<Output = Result<Vec<CardId>, RepoError>> + Send;

    /// Share of due reviews since then that were remembered, if there were any.
    fn retention_since(
        &self,
        since: DateTime<Utc>,
    ) -> impl Future<Output = Result<Option<u8>, RepoError>> + Send;

    /// Cards forgotten again and again.
    fn slipping_count(&self) -> impl Future<Output = Result<i64, RepoError>> + Send;

    fn schedule_of(&self, card: CardId)
    -> impl Future<Output = Result<Schedule, RepoError>> + Send;

    fn record_review(
        &self,
        card: CardId,
        grade: Grade,
        before: &Schedule,
        after: &Schedule,
        now: DateTime<Utc>,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    /// Takes back the card's latest review, restoring the schedule from before it.
    fn undo_review(&self, card: CardId) -> impl Future<Output = Result<(), RepoError>> + Send;

    fn explanation_request(
        &self,
        card: CardId,
    ) -> impl Future<Output = Result<ExplanationRequest, RepoError>> + Send;

    fn update_card(
        &self,
        card: CardId,
        content: &Card,
    ) -> impl Future<Output = Result<(), RepoError>> + Send;

    /// The due date of every active card, for forecasts.
    fn all_due_dates(&self) -> impl Future<Output = Result<Vec<DateTime<Utc>>, RepoError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_use_the_shortest_readable_unit() {
        assert_eq!(format_interval(Duration::seconds(30)), "1m");
        assert_eq!(format_interval(Duration::minutes(10)), "10m");
        assert_eq!(format_interval(Duration::hours(5)), "5h");
        assert_eq!(format_interval(Duration::days(8)), "8d");
        assert_eq!(format_interval(Duration::days(90)), "3mo");
        assert_eq!(format_interval(Duration::days(730)), "2.0y");
    }
}
