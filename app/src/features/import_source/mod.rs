use chrono::{DateTime, Utc};
use myosotis_core::{
    CardGenerator, CardId, DraftToStore, GeneratedCard, GenerationRequest, ImportId, ImportRepo,
    NewImport, RepoError, find_duplicate, split_source,
};
use tauri::State;
use tauri::ipc::{InvokeBody, Request};

use crate::claude_code::ClaudeCode;
use crate::storage::SqliteCardRepo;

/// About a chapter's worth of text: enough context for good cards, and quick enough to
/// finish within one AI call.
const MAX_PART_CHARS: usize = 30_000;

pub async fn create(
    repo: &impl ImportRepo,
    source: &NewImport,
    now: DateTime<Utc>,
) -> Result<ImportId, RepoError> {
    let labelled: NewImport = NewImport {
        title: source.label(),
        ..source.clone()
    };
    repo.create_import(&labelled, now).await
}

/// Runs the AI over a stored source, part by part. Each part's cards are saved as soon
/// as they arrive, so the Inbox fills up during generation and a retry resumes where it
/// stopped. AI failures are recorded on the import; only storage failures come back as errors.
pub async fn generate(
    repo: &impl ImportRepo,
    generator: &impl CardGenerator,
    import: ImportId,
) -> Result<(), RepoError> {
    let request: GenerationRequest = repo.generation_request(import).await?;
    let parts: Vec<String> = split_source(&request.text, MAX_PART_CHARS);
    repo.mark_generating(import, parts.len()).await?;
    let done: usize = repo.parts_done(import).await?;

    for part in parts.iter().skip(done) {
        if !repo.is_generating(import).await? {
            return Ok(());
        }
        let part_request: GenerationRequest = GenerationRequest {
            text: part.clone(),
            ..request.clone()
        };
        match generator.generate(&part_request).await {
            Ok(cards) => {
                let known: Vec<(CardId, String)> = repo.known_card_texts().await?;
                let drafts: Vec<DraftToStore> = cards
                    .into_iter()
                    .map(|card| {
                        let card: GeneratedCard = card.settled();
                        let text: String = format!("{} {}", card.question, card.answer);
                        DraftToStore {
                            duplicate_of: find_duplicate(&text, &known),
                            card,
                        }
                    })
                    .collect();
                repo.store_drafts(import, &drafts).await?;
            }
            Err(error) => return repo.mark_failed(import, &error.to_string()).await,
        }
    }

    if repo.draft_count(import).await? == 0 {
        return repo
            .mark_failed(import, "No cards could be created from this source.")
            .await;
    }
    repo.mark_ready(import).await?;
    repo.finish_import_if_empty(import).await
}

/// Gives up on a failed import. Any cards it already made stay in the Inbox.
pub async fn discard(repo: &impl ImportRepo, import: ImportId) -> Result<(), RepoError> {
    if repo.draft_count(import).await? == 0 {
        return repo.discard_failed_import(import).await;
    }
    repo.mark_generating_finished(import).await?;
    repo.finish_import_if_empty(import).await
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

/// Reads an opened file. It arrives as raw bytes, with its name in a header, because large
/// PDFs would be slow as JSON.
#[tauri::command]
pub async fn read_source_file(request: Request<'_>) -> Result<String, String> {
    let name: String = request
        .headers()
        .get("x-file-name")
        .and_then(|name| name.to_str().ok())
        .unwrap_or_default()
        .to_string();
    // Only the extension matters, and it survives the header's URL encoding.
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("The file couldn't be received.".to_string());
    };
    let bytes: Vec<u8> = bytes.clone();
    // Parsing a big PDF is slow, so it stays off the async threads.
    tauri::async_runtime::spawn_blocking(move || myosotis_core::read_source(&name, &bytes))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn stop_import(repo: State<'_, SqliteCardRepo>, import: ImportId) -> Result<(), String> {
    repo.mark_ready(import)
        .await
        .map_err(|error| error.to_string())?;
    repo.finish_import_if_empty(import)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn discard_import(
    repo: State<'_, SqliteCardRepo>,
    import: ImportId,
) -> Result<(), String> {
    discard(repo.inner(), import)
        .await
        .map_err(|error| error.to_string())
}
