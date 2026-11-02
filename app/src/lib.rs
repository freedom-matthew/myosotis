pub mod features;
pub mod storage;

use tauri::Manager;

use crate::storage::SqliteCardRepo;

/// Composition root: wires storage into the slices and starts the app.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let repo = tauri::async_runtime::block_on(SqliteCardRepo::open(
                &data_dir.join("myosotis.db"),
            ))?;
            app.manage(repo);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![features::list_cards::list_cards])
        .run(tauri::generate_context!())
        .expect("error while running Myosotis");
}
