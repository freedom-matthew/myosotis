pub mod claude_code;
mod clock;
pub mod features;
pub mod storage;

use std::path::PathBuf;

use chrono::Utc;
use tauri::Manager;

use crate::features::{
    add_card, backup, concept_questions, import_source, inbox, library, list_cards, review,
    settings, tidy, today,
};
use crate::storage::SqliteCardRepo;

/// Composition root: wires storage into the slices and starts the app.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir: PathBuf = backup::data_dir(app.handle())?;
            std::fs::create_dir_all(&data_dir)?;
            let database: PathBuf = data_dir.join("myosotis.db");
            let backups: PathBuf = backup::folder(app.handle())?.join("backups");
            backup::apply_staged_restore(&data_dir, &database)?;
            let repo: SqliteCardRepo = tauri::async_runtime::block_on(backup::open_collection(
                &database,
                &backups,
                Utc::now(),
            ))?;
            if let Err(error) =
                tauri::async_runtime::block_on(backup::daily_backup(&repo, &backups, Utc::now()))
            {
                eprintln!("daily backup failed: {error}");
            }
            match tauri::async_runtime::block_on(settings::load(&repo)) {
                Ok(saved) => settings::apply_size(app.handle(), saved.size),
                Err(error) => eprintln!("could not load settings: {error}"),
            }
            tauri::async_runtime::spawn(concept_questions::reword_after_startup(
                repo.clone(),
                backups,
            ));
            app.manage(repo);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_cards::list_cards,
            add_card::add_card,
            today::today_summary,
            review::review_session,
            review::grade_card,
            review::undo_grade,
            review::edit_card,
            review::explain_card,
            review::mark_description,
            settings::get_settings,
            settings::update_settings,
            settings::claude_status,
            backup::back_up_now,
            backup::export_cards,
            backup::data_folder,
            backup::list_backups,
            backup::restore_backup,
            library::library_cards,
            library::set_suspended,
            library::delete_card,
            import_source::start_import,
            import_source::retry_import,
            import_source::stop_import,
            inbox::rewrite_draft,
            inbox::keep_both_cards,
            tidy::tidy_groups,
            tidy::merge_group,
            tidy::mark_not_duplicates,
            import_source::read_source_file,
            inbox::inbox,
            inbox::accept_draft,
            inbox::accept_all_drafts,
            inbox::edit_draft,
            inbox::delete_draft,
            import_source::discard_import,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Myosotis");
}
