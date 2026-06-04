use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
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
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            toggle_hotkey: "F4".into(),
            sound_enabled: true,
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

pub fn load_store(app: &AppHandle) -> Result<ProfileStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        let store = default_store();
        save_store(app, &store)?;
        return Ok(store);
    }

    let data =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read profiles: {err}"))?;
    serde_json::from_str(&data).map_err(|err| format!("Failed to parse profiles: {err}"))
}

pub fn save_profile(app: &AppHandle, profile: Profile) -> Result<ProfileStore, String> {
    let mut store = load_store(app)?;
    if let Some(existing) = store.profiles.iter_mut().find(|item| item.id == profile.id) {
        *existing = profile;
    } else {
        store.profiles.push(profile);
    }
    save_store(app, &store)?;
    Ok(store)
}

pub fn delete_profile(app: &AppHandle, profile_id: String) -> Result<ProfileStore, String> {
    let mut store = load_store(app)?;
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

    save_store(app, &store)?;
    Ok(store)
}

pub fn set_active_profile(app: &AppHandle, profile_id: String) -> Result<ProfileStore, String> {
    let mut store = load_store(app)?;
    if !store
        .profiles
        .iter()
        .any(|profile| profile.id == profile_id)
    {
        return Err(format!("Profile not found: {profile_id}"));
    }
    store.active_profile_id = profile_id;
    save_store(app, &store)?;
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

fn save_store(app: &AppHandle, store: &ProfileStore) -> Result<(), String> {
    let path = store_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create profile directory: {err}"))?;
    }
    let data = serde_json::to_string_pretty(store)
        .map_err(|err| format!("Failed to encode profiles: {err}"))?;
    fs::write(path, data).map_err(|err| format!("Failed to write profiles: {err}"))
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
                trigger_key: "RIGHT CLICK".into(),
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
}
