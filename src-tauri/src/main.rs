//! Tauri entry point: builds the window and registers commands.
//!
//! On macOS the app shows in the Dock as a normal desktop application; no
//! menu-bar-only behaviour is needed, because every connection lives in a tab.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cube_studio_lib::commands::{self, AppState};
use cube_studio_lib::store::ProfileStore;
use tauri::Manager;

fn main() {
    commands::init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            // Profiles live in the platform data directory so they survive
            // upgrades and are backed up with the rest of the user's data.
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("cannot resolve the app data directory: {e}"))?;
            let store = ProfileStore::open(data_dir)
                .map_err(|e| format!("cannot open the profile store: {}", e.message))?;
            app.manage(AppState::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_metadata,
            commands::discover_cloud_resources,
            commands::cloud_resource_profile,
            commands::connect_cloud_resource,
            commands::list_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::connect,
            commands::disconnect,
            commands::list_sessions,
            commands::session_info,
            commands::test_connection,
            commands::list_containers,
            commands::list_entries,
            commands::read_entry,
            commands::write_entry,
            commands::delete_entries,
            commands::set_entry_ttl,
            commands::publish_message,
            commands::flush_container,
            commands::server_stats,
            commands::run_console,
            commands::start_tail,
            commands::stop_tail,
            commands::list_tails,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Let the window close immediately; sessions are torn down by
                // the process exiting. A blocking shutdown here would freeze the
                // UI for as long as a broker takes to answer.
                let _ = window.app_handle().state::<AppState>();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running cube studio");
}
