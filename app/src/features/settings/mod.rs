use myosotis_core::{RepoError, Settings, SettingsRepo};
use tauri::State;

use crate::claude_code::{self, ClaudeStatus};
use crate::storage::SqliteCardRepo;

pub async fn load(repo: &impl SettingsRepo) -> Result<Settings, RepoError> {
    repo.load_settings().await
}

pub async fn update(repo: &impl SettingsRepo, settings: Settings) -> Result<Settings, RepoError> {
    let validated: Settings = settings.validated();
    repo.save_settings(&validated).await?;
    Ok(validated)
}

#[tauri::command]
pub async fn get_settings(repo: State<'_, SqliteCardRepo>) -> Result<Settings, String> {
    load(repo.inner()).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_settings(
    repo: State<'_, SqliteCardRepo>,
    settings: Settings,
) -> Result<Settings, String> {
    update(repo.inner(), settings)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn claude_status() -> ClaudeStatus {
    claude_code::status().await
}
