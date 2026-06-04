use tauri::{AppHandle, State};

use crate::{
    input,
    macros::ValidationResult,
    profiles::{self, PixelPoint, Profile, ProfileStore},
    runtime::RuntimeState,
    screen::{self, PixelSample, PixelSampleRequest},
};

#[tauri::command]
pub fn get_profiles(app: AppHandle) -> Result<ProfileStore, String> {
    profiles::load_store(&app)
}

#[tauri::command]
pub fn save_profile(app: AppHandle, profile: Profile) -> Result<ProfileStore, String> {
    profiles::save_profile(&app, profile)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, profile_id: String) -> Result<ProfileStore, String> {
    profiles::delete_profile(&app, profile_id)
}

#[tauri::command]
pub fn set_active_profile(app: AppHandle, profile_id: String) -> Result<ProfileStore, String> {
    profiles::set_active_profile(&app, profile_id)
}

#[tauri::command]
pub fn start_runtime(
    app: AppHandle,
    state: State<RuntimeState>,
    profile_id: String,
) -> Result<(), String> {
    let profile = profiles::get_active_profile(&app, &profile_id)?;
    state.start(app, profile)
}

#[tauri::command]
pub fn stop_runtime(app: AppHandle, state: State<RuntimeState>) -> Result<(), String> {
    state.stop(&app)
}

#[tauri::command]
pub fn sample_pixel(request: PixelSampleRequest) -> Result<PixelSample, String> {
    screen::sample_pixel(PixelPoint {
        x: request.x,
        y: request.y,
    })
}

#[tauri::command]
pub fn pick_pixel() -> Result<PixelSample, String> {
    screen::pick_pixel_from_click(15_000)
}

#[tauri::command]
pub fn validate_key_sequence(sequence: Vec<String>) -> Result<ValidationResult, String> {
    let errors = sequence
        .into_iter()
        .filter(|key| !input::supports_key(key))
        .map(|key| format!("Unsupported key: {key}"))
        .collect::<Vec<_>>();

    Ok(ValidationResult {
        valid: errors.is_empty(),
        errors,
    })
}
