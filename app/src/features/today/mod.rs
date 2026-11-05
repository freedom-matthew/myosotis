use chrono::{DateTime, Duration, Utc};
use myosotis_core::{
    ImportRepo, RecentImport, RepoError, ReviewRepo, SessionCard, SettingsRepo, TodaySummary,
    summarise_day,
};
use tauri::State;

use crate::clock;
use crate::features::review;
use crate::storage::SqliteCardRepo;

const RECENT_IMPORTS: i64 = 5;
const RETENTION_DAYS: i64 = 30;

pub async fn summary(
    repo: &(impl ReviewRepo + ImportRepo + SettingsRepo),
    now: DateTime<Utc>,
    start_of_day: DateTime<Utc>,
) -> Result<TodaySummary, RepoError> {
    let session: Vec<SessionCard> = review::session(repo, now, start_of_day).await?;
    let due_dates: Vec<DateTime<Utc>> = repo.all_due_dates().await?;
    let inbox_count: i64 = repo.inbox_count().await?;
    let recent_imports: Vec<RecentImport> = repo.recent_imports(RECENT_IMPORTS).await?;
    let month_ago: DateTime<Utc> = now - Duration::days(RETENTION_DAYS);
    Ok(TodaySummary {
        retention_percent: repo.retention_since(month_ago).await?,
        slipping_count: repo.slipping_count().await?,
        ..summarise_day(
            &session,
            &due_dates,
            inbox_count,
            recent_imports,
            now,
            start_of_day,
        )
    })
}

#[tauri::command]
pub async fn today_summary(repo: State<'_, SqliteCardRepo>) -> Result<TodaySummary, String> {
    summary(repo.inner(), Utc::now(), clock::start_of_local_day())
        .await
        .map_err(|error| error.to_string())
}
