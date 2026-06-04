mod commands;
mod foreground;
mod input;
mod macros;
mod profiles;
mod recorder;
mod runtime;
mod screen;

use runtime::RuntimeState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(RuntimeState::default())
        .manage(recorder::RecorderState::default())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
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
            commands::validate_key_sequence,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");
    app.run(|app, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            let _ = app.state::<RuntimeState>().stop(app);
        }
    });
}
