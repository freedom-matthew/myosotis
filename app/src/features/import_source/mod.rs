use chrono::{DateTime, Utc};
use myosotis_core::{
    CardGenerator, GenerateError, GeneratedCard, GenerationRequest, ImportId, ImportRepo,
    NewImport, RepoError,
};
use tauri::State;

use crate::claude_code::ClaudeCode;
use crate::storage::SqliteCardRepo;

pub async fn create(
    repo: &impl ImportRepo,
    source: &NewImport,
    now: DateTime<Utc>,
) -> Result<ImportId, RepoError> {
    repo.create_import(source, now).await
}

/// Runs the AI over a stored source. Its failures are recorded on the import for the
/// Inbox to show; only storage failures come back as errors.
pub async fn generate(
    repo: &impl ImportRepo,
    generator: &impl CardGenerator,
    import: ImportId,
) -> Result<(), RepoError> {
    repo.mark_generating(import).await?;
    let request: GenerationRequest = repo.generation_request(import).await?;
    let result: Result<Vec<GeneratedCard>, GenerateError> = generator.generate(&request).await;
    match result {
        Ok(cards) if cards.is_empty() => {
            repo.mark_failed(import, "No cards could be created from this source.")
                .await
        }
        Ok(cards) => repo.store_drafts(import, &cards).await,
        Err(error) => repo.mark_failed(import, &error.to_string()).await,
    }
}

/// Generation takes a while, so it runs in the background and the UI watches the Inbox.
fn generate_in_background(repo: SqliteCardRepo, import: ImportId) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = generate(&repo, &ClaudeCode, import).await {
            eprintln!("import {import} could not be saved: {error}");
        }
    });
}

#[tauri::command]
pub async fn start_import(
    repo: State<'_, SqliteCardRepo>,
    source: NewImport,
) -> Result<ImportId, String> {
    let import: ImportId = create(repo.inner(), &source, Utc::now())
        .await
        .map_err(|error| error.to_string())?;
    generate_in_background(repo.inner().clone(), import);
    Ok(import)
}

#[tauri::command]
pub async fn retry_import(repo: State<'_, SqliteCardRepo>, import: ImportId) -> Result<(), String> {
    generate_in_background(repo.inner().clone(), import);
    Ok(())
}
