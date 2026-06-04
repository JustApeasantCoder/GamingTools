mod commands;
mod input;
mod macros;
mod profiles;
mod runtime;
mod screen;

use runtime::RuntimeState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RuntimeState::default())
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
            commands::start_runtime,
            commands::stop_runtime,
            commands::sample_pixel,
            commands::pick_pixel,
            commands::validate_key_sequence,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
