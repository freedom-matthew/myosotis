use chrono::{DateTime, Utc};
use myosotis_core::{
    ImportRepo, RecentImport, RepoError, ReviewRepo, SessionCard, TodaySummary, summarise_day,
};
use tauri::State;

use crate::clock;
use crate::features::review;
use crate::storage::SqliteCardRepo;

const RECENT_IMPORTS: i64 = 5;

pub async fn summary(
    repo: &(impl ReviewRepo + ImportRepo),
    now: DateTime<Utc>,
    start_of_day: DateTime<Utc>,
) -> Result<TodaySummary, RepoError> {
    let session: Vec<SessionCard> = review::session(repo, now, start_of_day).await?;
    let due_dates: Vec<DateTime<Utc>> = repo.all_due_dates().await?;
    let inbox_count: i64 = repo.inbox_count().await?;
    let recent_imports: Vec<RecentImport> = repo.recent_imports(RECENT_IMPORTS).await?;
    Ok(summarise_day(
        &session,
        &due_dates,
        inbox_count,
        recent_imports,
        now,
        start_of_day,
    ))
}

#[tauri::command]
pub async fn today_summary(repo: State<'_, SqliteCardRepo>) -> Result<TodaySummary, String> {
    summary(repo.inner(), Utc::now(), clock::start_of_local_day())
        .await
        .map_err(|error| error.to_string())
}
