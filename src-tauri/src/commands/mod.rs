use tauri::{AppHandle, State};

use crate::{
    input,
    macros::ValidationResult,
    profiles::{
        self, InventorySlotSnapshot, InventoryStashRule, PixelPoint, PixelRule, Profile,
        ProfileStore,
    },
    recorder::RecorderState,
    runtime::RuntimeState,
    screen::{self, PixelSample, PixelSampleRequest},
};

#[tauri::command]
pub fn get_profiles(app: AppHandle) -> Result<ProfileStore, String> {
    profiles::load_store(&app)
}

#[tauri::command]
pub fn save_profile(
    app: AppHandle,
    state: State<RuntimeState>,
    profile: Profile,
) -> Result<ProfileStore, String> {
    let store = profiles::save_profile(&app, profile.clone())?;
    state.refresh_profile(app, profile)?;
    Ok(store)
}

#[tauri::command]
pub fn delete_profile(
    app: AppHandle,
    state: State<RuntimeState>,
    profile_id: String,
) -> Result<ProfileStore, String> {
    let store = profiles::delete_profile(&app, profile_id)?;
    if let Some(profile) = store
        .profiles
        .iter()
        .find(|profile| profile.id == store.active_profile_id)
        .cloned()
    {
        state.refresh_running(app, profile)?;
    }
    Ok(store)
}

#[tauri::command]
pub fn set_active_profile(
    app: AppHandle,
    state: State<RuntimeState>,
    profile_id: String,
) -> Result<ProfileStore, String> {
    let store = profiles::set_active_profile(&app, profile_id)?;
    if let Some(profile) = store
        .profiles
        .iter()
        .find(|profile| profile.id == store.active_profile_id)
        .cloned()
    {
        state.refresh_running(app, profile)?;
    }
    Ok(store)
}

#[tauri::command]
pub fn export_profile(app: AppHandle, profile_id: String) -> Result<String, String> {
    profiles::export_profile(&app, &profile_id)
}

#[tauri::command]
pub fn import_profile(
    app: AppHandle,
    state: State<RuntimeState>,
    json: String,
) -> Result<ProfileStore, String> {
    let store = profiles::import_profile(&app, &json)?;
    if let Some(profile) = store
        .profiles
        .iter()
        .find(|profile| profile.id == store.active_profile_id)
        .cloned()
    {
        state.refresh_running(app, profile)?;
    }
    Ok(store)
}

#[tauri::command]
pub fn get_foreground_app() -> Result<crate::foreground::ForegroundApp, String> {
    crate::foreground::current_app()
}

#[tauri::command]
pub fn start_macro_recording(state: State<RecorderState>) -> Result<(), String> {
    state.start()
}

#[tauri::command]
pub fn stop_macro_recording(
    state: State<RecorderState>,
) -> Result<Vec<crate::profiles::MacroStep>, String> {
    state.stop()
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
pub fn is_runtime_running(state: State<RuntimeState>) -> bool {
    state.is_running()
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
pub fn test_pixel_rule(rule: PixelRule) -> Result<bool, String> {
    crate::runtime::test_pixel_rule(&rule)
}

#[tauri::command]
pub fn test_pixel_actions(app: AppHandle, rule: PixelRule) -> Result<(), String> {
    crate::runtime::test_pixel_actions(&app, &rule)
}

#[tauri::command]
pub fn test_inventory_stash_rule(rule: InventoryStashRule) -> Result<usize, String> {
    crate::inventory::test_rule(&rule)
}

#[tauri::command]
pub fn capture_inventory_stash_snapshot(
    rule: InventoryStashRule,
) -> Result<Vec<InventorySlotSnapshot>, String> {
    crate::inventory::capture_snapshot(&rule)
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
