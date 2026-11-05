mod claude_code;
mod clock;
pub mod features;
pub mod storage;

use std::path::PathBuf;

use tauri::Manager;

use crate::features::{import_source, inbox, library, list_cards, review, today};
use crate::storage::SqliteCardRepo;

/// Composition root: wires storage into the slices and starts the app.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let repo: SqliteCardRepo = tauri::async_runtime::block_on(SqliteCardRepo::open(
                &data_dir.join("myosotis.db"),
            ))?;
            app.manage(repo);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_cards::list_cards,
            today::today_summary,
            review::review_session,
            review::grade_card,
            review::undo_grade,
            review::edit_card,
            review::explain_card,
            library::library_cards,
            library::set_suspended,
            library::delete_card,
            import_source::start_import,
            import_source::retry_import,
            inbox::inbox,
            inbox::accept_draft,
            inbox::accept_all_drafts,
            inbox::edit_draft,
            inbox::delete_draft,
            inbox::discard_import,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Myosotis");
}
