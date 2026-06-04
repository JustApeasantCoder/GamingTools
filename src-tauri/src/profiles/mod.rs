use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HumanizationSettings {
    pub enabled: bool,
    pub min_ms: u64,
    pub max_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MacroStep {
    pub id: String,
    pub key: String,
    #[serde(default = "default_press_duration")]
    pub press_duration: HumanizationSettings,
    pub humanized_delay: HumanizationSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MacroRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger_key: String,
    pub steps: Vec<MacroStep>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PixelPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PixelCondition {
    pub target_color: String,
    pub tolerance: u8,
    pub adjacent_pixels: bool,
    pub sample_point: PixelPoint,
    #[serde(default)]
    pub invert_detection: bool,
}

impl Default for PixelCondition {
    fn default() -> Self {
        Self {
            target_color: "#ffffff".into(),
            tolerance: 12,
            adjacent_pixels: false,
            sample_point: PixelPoint { x: 640, y: 360 },
            invert_detection: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PixelRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub target_color: String,
    pub tolerance: u8,
    pub adjacent_pixels: bool,
    pub sample_point: PixelPoint,
    #[serde(default)]
    pub invert_detection: bool,
    #[serde(default)]
    pub secondary_condition_enabled: bool,
    #[serde(default)]
    pub secondary_condition: PixelCondition,
    #[serde(default)]
    pub secondary_condition2_enabled: bool,
    #[serde(default)]
    pub secondary_condition2: PixelCondition,
    #[serde(default = "default_condition_operator")]
    pub secondary_condition_operator: String,
    #[serde(default = "default_pixel_trigger_mode")]
    pub trigger_mode: String,
    #[serde(default = "default_true")]
    pub continue_while_detected: bool,
    #[serde(default)]
    pub action_steps: Vec<MacroStep>,
    #[serde(default)]
    pub output_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToggleHoldRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger_key: String,
    pub hold_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub default_humanization: HumanizationSettings,
    #[serde(default)]
    pub runtime_settings: RuntimeSettings,
    pub macro_rules: Vec<MacroRule>,
    pub pixel_rules: Vec<PixelRule>,
    #[serde(default)]
    pub toggle_hold_rules: Vec<ToggleHoldRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettings {
    pub toggle_hotkey: String,
    pub sound_enabled: bool,
    #[serde(default)]
    pub foreground_guard: ForegroundGuardSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForegroundGuardSettings {
    pub enabled: bool,
    pub executable: String,
    pub on_focus_lost: String,
}

impl Default for ForegroundGuardSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            executable: String::new(),
            on_focus_lost: "pause".into(),
        }
    }
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            toggle_hotkey: "F4".into(),
            sound_enabled: true,
            foreground_guard: ForegroundGuardSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileStore {
    pub active_profile_id: String,
    pub profiles: Vec<Profile>,
}

const STORE_FILE: &str = "profiles.json";
static STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn load_store(app: &AppHandle) -> Result<ProfileStore, String> {
    let _guard = store_lock()?;
    load_store_unlocked(app)
}

fn load_store_unlocked(app: &AppHandle) -> Result<ProfileStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        let store = default_store();
        save_store_unlocked(app, &store)?;
        return Ok(store);
    }

    let data =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read profiles: {err}"))?;
    match serde_json::from_str(&data) {
        Ok(store) => Ok(normalize_store(store)),
        Err(error) => {
            preserve_corrupt_store(&path)?;
            let store = default_store();
            save_store_unlocked(app, &store)?;
            log::error!("Recovered malformed profile store: {error}");
            Ok(store)
        }
    }
}

pub fn save_profile(app: &AppHandle, profile: Profile) -> Result<ProfileStore, String> {
    let validation = crate::macros::validate_profile(&profile);
    if !validation.valid {
        return Err(validation.errors.join("; "));
    }
    let _guard = store_lock()?;
    let mut store = load_store_unlocked(app)?;
    if let Some(existing) = store.profiles.iter_mut().find(|item| item.id == profile.id) {
        *existing = profile;
    } else {
        store.profiles.push(profile);
    }
    save_store_unlocked(app, &store)?;
    Ok(store)
}

pub fn delete_profile(app: &AppHandle, profile_id: String) -> Result<ProfileStore, String> {
    let _guard = store_lock()?;
    let mut store = load_store_unlocked(app)?;
    if store.profiles.len() <= 1 {
        return Err("At least one profile must remain".into());
    }

    let original_len = store.profiles.len();
    store.profiles.retain(|profile| profile.id != profile_id);
    if store.profiles.len() == original_len {
        return Err(format!("Profile not found: {profile_id}"));
    }

    if store.active_profile_id == profile_id {
        store.active_profile_id = store
            .profiles
            .first()
            .map(|profile| profile.id.clone())
            .ok_or_else(|| "At least one profile must remain".to_string())?;
    }

    save_store_unlocked(app, &store)?;
    Ok(store)
}

pub fn set_active_profile(app: &AppHandle, profile_id: String) -> Result<ProfileStore, String> {
    let _guard = store_lock()?;
    let mut store = load_store_unlocked(app)?;
    if !store
        .profiles
        .iter()
        .any(|profile| profile.id == profile_id)
    {
        return Err(format!("Profile not found: {profile_id}"));
    }
    store.active_profile_id = profile_id;
    save_store_unlocked(app, &store)?;
    Ok(store)
}

pub fn get_active_profile(app: &AppHandle, profile_id: &str) -> Result<Profile, String> {
    let store = load_store(app)?;
    store
        .profiles
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {profile_id}"))
}

pub fn export_profile(app: &AppHandle, profile_id: &str) -> Result<String, String> {
    let profile = get_active_profile(app, profile_id)?;
    serde_json::to_string_pretty(&profile).map_err(|err| format!("Failed to export profile: {err}"))
}

pub fn import_profile(app: &AppHandle, json: &str) -> Result<ProfileStore, String> {
    let mut profile: Profile =
        serde_json::from_str(json).map_err(|err| format!("Invalid profile JSON: {err}"))?;
    profile.id = uuid::Uuid::new_v4().to_string();
    profile.name = format!("{} Imported", profile.name.trim());
    regenerate_ids(&mut profile);
    save_profile(app, profile.clone())?;
    set_active_profile(app, profile.id)
}

fn regenerate_ids(profile: &mut Profile) {
    for rule in &mut profile.macro_rules {
        rule.id = uuid::Uuid::new_v4().to_string();
        for step in &mut rule.steps {
            step.id = uuid::Uuid::new_v4().to_string();
        }
    }
    for rule in &mut profile.pixel_rules {
        rule.id = uuid::Uuid::new_v4().to_string();
        for step in &mut rule.action_steps {
            step.id = uuid::Uuid::new_v4().to_string();
        }
    }
    for rule in &mut profile.toggle_hold_rules {
        rule.id = uuid::Uuid::new_v4().to_string();
    }
}

fn save_store_unlocked(app: &AppHandle, store: &ProfileStore) -> Result<(), String> {
    let path = store_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create profile directory: {err}"))?;
    }
    let data = serde_json::to_string_pretty(store)
        .map_err(|err| format!("Failed to encode profiles: {err}"))?;
    let temporary_path = path.with_extension("json.tmp");
    fs::write(&temporary_path, data)
        .map_err(|err| format!("Failed to write temporary profiles: {err}"))?;
    replace_file(&temporary_path, &path)
}

fn store_lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    STORE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Profile store lock poisoned".to_string())
}

fn normalize_store(mut store: ProfileStore) -> ProfileStore {
    if store.profiles.is_empty() {
        return default_store();
    }
    if !store
        .profiles
        .iter()
        .any(|profile| profile.id == store.active_profile_id)
    {
        store.active_profile_id = store.profiles[0].id.clone();
    }
    store
}

fn preserve_corrupt_store(path: &Path) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup = path.with_file_name(format!("profiles.corrupt-{timestamp}.json"));
    fs::rename(path, backup).map_err(|err| format!("Failed to preserve corrupt profiles: {err}"))
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(format!(
            "Failed to replace profiles: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(|err| format!("Failed to replace profiles: {err}"))
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join(STORE_FILE))
        .map_err(|err| format!("Failed to locate app config directory: {err}"))
}

fn default_press_duration() -> HumanizationSettings {
    HumanizationSettings {
        enabled: true,
        min_ms: 50,
        max_ms: 90,
    }
}

fn default_pixel_trigger_mode() -> String {
    "hold".into()
}

fn default_condition_operator() -> String {
    "and".into()
}

fn default_true() -> bool {
    true
}

fn default_store() -> ProfileStore {
    ProfileStore {
        active_profile_id: "default".into(),
        profiles: vec![Profile {
            id: "default".into(),
            name: "Default Profile".into(),
            default_humanization: HumanizationSettings {
                enabled: true,
                min_ms: 100,
                max_ms: 220,
            },
            runtime_settings: RuntimeSettings::default(),
            macro_rules: vec![MacroRule {
                id: "macro-default".into(),
                name: "Farming Loop".into(),
                enabled: true,
                trigger_key: "F6".into(),
                steps: vec![
                    MacroStep {
                        id: "step-a".into(),
                        key: "A".into(),
                        press_duration: HumanizationSettings {
                            enabled: true,
                            min_ms: 50,
                            max_ms: 90,
                        },
                        humanized_delay: HumanizationSettings {
                            enabled: true,
                            min_ms: 100,
                            max_ms: 200,
                        },
                    },
                    MacroStep {
                        id: "step-b".into(),
                        key: "B".into(),
                        press_duration: HumanizationSettings {
                            enabled: true,
                            min_ms: 60,
                            max_ms: 100,
                        },
                        humanized_delay: HumanizationSettings {
                            enabled: true,
                            min_ms: 150,
                            max_ms: 250,
                        },
                    },
                    MacroStep {
                        id: "step-c".into(),
                        key: "C".into(),
                        press_duration: HumanizationSettings {
                            enabled: true,
                            min_ms: 70,
                            max_ms: 110,
                        },
                        humanized_delay: HumanizationSettings {
                            enabled: true,
                            min_ms: 200,
                            max_ms: 300,
                        },
                    },
                ],
            }],
            pixel_rules: vec![PixelRule {
                id: "pixel-default".into(),
                name: "Health Color Watch".into(),
                enabled: true,
                target_color: "#34d399".into(),
                tolerance: 12,
                adjacent_pixels: true,
                sample_point: PixelPoint { x: 640, y: 360 },
                invert_detection: false,
                secondary_condition_enabled: false,
                secondary_condition: PixelCondition::default(),
                secondary_condition2_enabled: false,
                secondary_condition2: PixelCondition::default(),
                secondary_condition_operator: default_condition_operator(),
                trigger_mode: "hold".into(),
                continue_while_detected: true,
                action_steps: vec![MacroStep {
                    id: "pixel-step-q".into(),
                    key: "Q".into(),
                    press_duration: HumanizationSettings {
                        enabled: true,
                        min_ms: 50,
                        max_ms: 90,
                    },
                    humanized_delay: HumanizationSettings {
                        enabled: true,
                        min_ms: 80,
                        max_ms: 150,
                    },
                }],
                output_key: None,
            }],
            toggle_hold_rules: vec![ToggleHoldRule {
                id: "toggle-default".into(),
                name: "Right Click Hold".into(),
                enabled: true,
                trigger_key: "F8".into(),
                hold_key: "RIGHT CLICK".into(),
            }],
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_store_round_trips_json() {
        let store = default_store();
        let json = serde_json::to_string(&store).unwrap();
        let decoded: ProfileStore = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, store);
    }

    #[test]
    fn delete_profile_moves_active_profile() {
        let mut store = default_store();
        store.profiles.push(Profile {
            id: "second".into(),
            name: "Second".into(),
            default_humanization: HumanizationSettings {
                enabled: true,
                min_ms: 10,
                max_ms: 20,
            },
            runtime_settings: RuntimeSettings::default(),
            macro_rules: vec![],
            pixel_rules: vec![],
            toggle_hold_rules: vec![],
        });
        store.active_profile_id = "second".into();

        store.profiles.retain(|profile| profile.id != "second");
        if store.active_profile_id == "second" {
            store.active_profile_id = store.profiles.first().unwrap().id.clone();
        }

        assert_eq!(store.profiles.len(), 1);
        assert_eq!(store.active_profile_id, "default");
    }

    #[test]
    fn validation_rejects_unsupported_keys_and_toggle_conflicts() {
        let mut profile = default_store().profiles.remove(0);
        profile.macro_rules[0].steps[0].key = "ARROWUP".into();
        profile.toggle_hold_rules[0].hold_key = "F4".into();

        let result = crate::macros::validate_profile(&profile);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("unsupported key")));
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("toggle hotkey")));
    }

    #[test]
    fn validation_allows_toggle_hold_button_toggler() {
        let mut profile = default_store().profiles.remove(0);
        profile.toggle_hold_rules[0].trigger_key = "RIGHT CLICK".into();

        let result = crate::macros::validate_profile(&profile);

        assert!(result.valid);
    }

    #[test]
    fn validation_rejects_duplicate_runtime_ids() {
        let mut profile = default_store().profiles.remove(0);
        profile.toggle_hold_rules[0].id = profile.macro_rules[0].id.clone();

        let result = crate::macros::validate_profile(&profile);

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error.contains("Duplicate id")));
    }

    #[test]
    fn regenerated_profile_ids_are_unique() {
        let mut profile = default_store().profiles.remove(0);
        let original_profile_id = profile.id.clone();
        let original_macro_id = profile.macro_rules[0].id.clone();
        regenerate_ids(&mut profile);

        assert_eq!(profile.id, original_profile_id);
        assert_ne!(profile.macro_rules[0].id, original_macro_id);
        assert_ne!(profile.macro_rules[0].steps[0].id, "step-a");
    }
}
