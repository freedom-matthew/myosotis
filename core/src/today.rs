use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::import::RecentImport;
use crate::review::SessionCard;

const SECONDS_PER_CARD: usize = 15;
const FORECAST_DAYS: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TodaySummary {
    pub due_reviews: usize,
    pub due_new: usize,
    pub minutes: usize,
    pub next_due: Option<DateTime<Utc>>,
    pub inbox_count: i64,
    pub forecast: Vec<usize>,
    pub recent_imports: Vec<RecentImport>,
}

/// Works out Today's numbers. Bar 0 of the forecast is today and counts everything
/// already overdue; later bars are the following days.
pub fn summarise_day(
    session: &[SessionCard],
    due_dates: &[DateTime<Utc>],
    inbox_count: i64,
    recent_imports: Vec<RecentImport>,
    now: DateTime<Utc>,
    start_of_day: DateTime<Utc>,
) -> TodaySummary {
    let due_new: usize = session.iter().filter(|card| card.is_new).count();
    let due_reviews: usize = session.len() - due_new;

    let mut forecast: Vec<usize> = vec![0; FORECAST_DAYS];
    for due in due_dates {
        let day: i64 = (*due - start_of_day).num_days().max(0);
        if let Some(bar) = forecast.get_mut(day as usize) {
            *bar += 1;
        }
    }

    TodaySummary {
        due_reviews,
        due_new,
        minutes: (session.len() * SECONDS_PER_CARD).div_ceil(60),
        next_due: due_dates.iter().copied().filter(|due| *due > now).min(),
        inbox_count,
        forecast,
        recent_imports,
    }
}
