use myosotis_core::{RepoError, Settings, SettingsRepo, Size};
use tauri::{AppHandle, Manager, State, WebviewWindow};

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

/// Draws the whole window at the chosen size.
pub fn apply_size(app: &AppHandle, size: Size) {
    let window: Option<WebviewWindow> = app.get_webview_window("main");
    if let Some(window) = window
        && let Err(error) = window.set_zoom(size.zoom())
    {
        eprintln!("could not change the size: {error}");
    }
}

#[tauri::command]
pub async fn get_settings(repo: State<'_, SqliteCardRepo>) -> Result<Settings, String> {
    load(repo.inner()).await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    repo: State<'_, SqliteCardRepo>,
    settings: Settings,
) -> Result<Settings, String> {
    let saved: Settings = update(repo.inner(), settings)
        .await
        .map_err(|error| error.to_string())?;
    apply_size(&app, saved.size);
    Ok(saved)
}

#[tauri::command]
pub async fn claude_status() -> ClaudeStatus {
    claude_code::status().await
}
