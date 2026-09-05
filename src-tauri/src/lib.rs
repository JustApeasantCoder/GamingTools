mod commands;
mod diagnostics;
mod foreground;
mod input;
mod inventory;
mod macros;
mod map_crafter;
mod profiles;
mod recorder;
mod runtime;
mod screen;
mod stash_inventory;
mod tablets;

use runtime::RuntimeState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let diagnostics_guard = std::sync::Arc::new(std::sync::Mutex::new(None));
    let setup_guard = diagnostics_guard.clone();
    let app = tauri::Builder::default()
        .manage(RuntimeState::default())
        .manage(recorder::RecorderState::default())
        .setup(move |app| {
            match app.path().app_log_dir() {
                Ok(directory) => match diagnostics::initialize(&directory) {
                    Ok(guard) => *setup_guard.lock().unwrap() = Some(guard),
                    Err(error) => eprintln!("Could not initialize diagnostics: {error}"),
                },
                Err(error) => eprintln!("Could not resolve diagnostics directory: {error}"),
            }
            tracing::info!(target: "gaming_toolkit.diagnostics", subsystem = "app",
                event = "app.started", "[App] Gaming Toolkit started");
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            input::start_wheel_monitor().map_err(std::io::Error::other)?;
            runtime::start_hotkey_monitor(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::set_active_profile,
            commands::export_profile,
            commands::import_profile,
            commands::get_foreground_app,
            commands::start_macro_recording,
            commands::stop_macro_recording,
            commands::start_runtime,
            commands::stop_runtime,
            commands::is_runtime_running,
            commands::sample_pixel,
            commands::pick_pixel,
            commands::test_pixel_rule,
            commands::test_pixel_actions,
            commands::test_inventory_stash_rule,
            commands::capture_inventory_stash_snapshot,
            commands::test_stash_inventory_rule,
            commands::scan_tablet_stash,
            commands::scan_and_craft_tablets,
            commands::capture_tablet_craft_location,
            commands::highlight_tablet_slot,
            commands::move_tablet_to_inventory,
            commands::scan_map_grid,
            commands::craft_maps,
            commands::capture_map_currency_location,
            commands::validate_key_sequence,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");
    app.run(move |app, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            let _ = app.state::<RuntimeState>().stop(app);
        }
        if matches!(event, tauri::RunEvent::Exit) {
            tracing::info!(target: "gaming_toolkit.diagnostics", subsystem = "app",
                event = "app.stopped", "[App] Gaming Toolkit stopped");
            // Drain queued events before Tauri completes process shutdown.
            diagnostics_guard.lock().unwrap().take();
        }
    });
}
