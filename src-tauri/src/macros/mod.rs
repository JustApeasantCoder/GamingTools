use rand::Rng;
use serde::Serialize;

use std::collections::HashSet;

use crate::{
    input,
    profiles::{HumanizationSettings, MacroRule, MacroStep, Profile},
    screen,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn validate_rules(rules: &[MacroRule]) -> ValidationResult {
    let mut errors = Vec::new();

    for rule in rules {
        if rule.enabled && rule.trigger_key.trim().is_empty() {
            errors.push(format!("{} is missing a trigger key", rule.name));
        }
        if rule.enabled && rule.steps.is_empty() {
            errors.push(format!("{} has no action steps", rule.name));
        }
        for step in &rule.steps {
            validate_step(step, &mut errors);
        }
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn validate_profile(profile: &Profile) -> ValidationResult {
    let mut errors = validate_rules(&profile.macro_rules).errors;
    let mut ids = HashSet::new();
    let toggle_hotkey = profile.runtime_settings.toggle_hotkey.trim();

    validate_key(toggle_hotkey, "Runtime toggle hotkey", &mut errors);
    if profile.runtime_settings.foreground_guard.enabled {
        if profile
            .runtime_settings
            .foreground_guard
            .executable
            .trim()
            .is_empty()
        {
            errors.push("Foreground app guard is enabled without an executable".into());
        }
        if !matches!(
            profile
                .runtime_settings
                .foreground_guard
                .on_focus_lost
                .as_str(),
            "pause" | "stop"
        ) {
            errors.push("Foreground app guard has an invalid focus-lost behavior".into());
        }
    }

    for rule in profile.macro_rules.iter().filter(|rule| rule.enabled) {
        validate_id(&rule.id, "macro rule", &mut ids, &mut errors);
        validate_key(
            &rule.trigger_key,
            &format!("{} trigger", rule.name),
            &mut errors,
        );
        reject_toggle_conflict(
            &rule.trigger_key,
            toggle_hotkey,
            &format!("{} trigger", rule.name),
            &mut errors,
        );
        for step in &rule.steps {
            validate_id(&step.id, "macro step", &mut ids, &mut errors);
            validate_key(&step.key, &format!("{} action", rule.name), &mut errors);
            reject_toggle_conflict(
                &step.key,
                toggle_hotkey,
                &format!("{} action", rule.name),
                &mut errors,
            );
        }
    }

    for rule in profile.pixel_rules.iter().filter(|rule| rule.enabled) {
        validate_id(&rule.id, "pixel rule", &mut ids, &mut errors);
        if !matches!(rule.trigger_mode.as_str(), "trigger" | "hold") {
            errors.push(format!("{} has an invalid trigger mode", rule.name));
        }
        if !screen::is_valid_hex_color(&rule.target_color) {
            errors.push(format!("{} has an invalid target color", rule.name));
        }
        if rule.secondary_condition_enabled
            && !screen::is_valid_hex_color(&rule.secondary_condition.target_color)
        {
            errors.push(format!("{} has an invalid Target B1 color", rule.name));
        }
        if rule.secondary_condition_enabled
            && rule.secondary_condition2_enabled
            && !screen::is_valid_hex_color(&rule.secondary_condition2.target_color)
        {
            errors.push(format!("{} has an invalid Target B2 color", rule.name));
        }
        if rule.secondary_condition2_enabled
            && !matches!(rule.secondary_condition_operator.as_str(), "and" | "or")
        {
            errors.push(format!("{} has an invalid condition operator", rule.name));
        }
        let steps = if rule.action_steps.is_empty() {
            rule.output_key.iter().collect::<Vec<_>>()
        } else {
            rule.action_steps.iter().map(|step| &step.key).collect()
        };
        if steps.is_empty() {
            errors.push(format!("{} has no output actions", rule.name));
        }
        for key in steps {
            validate_key(key, &format!("{} output", rule.name), &mut errors);
            reject_toggle_conflict(
                key,
                toggle_hotkey,
                &format!("{} output", rule.name),
                &mut errors,
            );
        }
        for step in &rule.action_steps {
            validate_id(&step.id, "pixel action step", &mut ids, &mut errors);
            validate_step(step, &mut errors);
        }
    }

    for rule in profile.toggle_hold_rules.iter().filter(|rule| rule.enabled) {
        validate_id(&rule.id, "toggle-hold rule", &mut ids, &mut errors);
        validate_key(
            &rule.trigger_key,
            &format!("{} trigger", rule.name),
            &mut errors,
        );
        validate_key(
            &rule.hold_key,
            &format!("{} hold action", rule.name),
            &mut errors,
        );
        reject_toggle_conflict(
            &rule.trigger_key,
            toggle_hotkey,
            &format!("{} trigger", rule.name),
            &mut errors,
        );
        reject_toggle_conflict(
            &rule.hold_key,
            toggle_hotkey,
            &format!("{} hold action", rule.name),
            &mut errors,
        );
    }

    ValidationResult {
        valid: errors.is_empty(),
        errors,
    }
}

fn validate_key(key: &str, label: &str, errors: &mut Vec<String>) {
    if key.trim().is_empty() {
        errors.push(format!("{label} is missing a key"));
    } else if !input::supports_key(key) {
        errors.push(format!("{label} uses unsupported key: {key}"));
    }
}

fn reject_toggle_conflict(key: &str, toggle_hotkey: &str, label: &str, errors: &mut Vec<String>) {
    if !toggle_hotkey.is_empty() && key.trim().eq_ignore_ascii_case(toggle_hotkey) {
        errors.push(format!("{label} conflicts with the runtime toggle hotkey"));
    }
}

fn validate_id(id: &str, label: &str, ids: &mut HashSet<String>, errors: &mut Vec<String>) {
    if id.trim().is_empty() {
        errors.push(format!("A {label} is missing an id"));
    } else if !ids.insert(id.to_string()) {
        errors.push(format!("Duplicate id used by {label}: {id}"));
    }
}

pub fn random_delay_ms(settings: &HumanizationSettings) -> u64 {
    if !settings.enabled {
        return 0;
    }

    if settings.min_ms == settings.max_ms {
        return settings.min_ms;
    }

    let min = settings.min_ms.min(settings.max_ms);
    let max = settings.min_ms.max(settings.max_ms);
    rand::thread_rng().gen_range(min..=max)
}

fn validate_step(step: &MacroStep, errors: &mut Vec<String>) {
    if step.key.trim().is_empty() {
        errors.push(format!("Step {} is missing a key", step.id));
    }
    if step.press_duration.enabled && step.press_duration.min_ms > step.press_duration.max_ms {
        errors.push(format!(
            "Step {} press duration min ms is greater than max ms",
            step.id
        ));
    }
    if step.press_duration.max_ms > 10_000 {
        errors.push(format!("Step {} press duration is too high", step.id));
    }
    if step.humanized_delay.enabled && step.humanized_delay.min_ms > step.humanized_delay.max_ms {
        errors.push(format!(
            "Step {} humanized min ms is greater than max ms",
            step.id
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_delay_stays_inside_range() {
        let settings = HumanizationSettings {
            enabled: true,
            min_ms: 35,
            max_ms: 42,
        };

        for _ in 0..100 {
            let value = random_delay_ms(&settings);
            assert!((35..=42).contains(&value));
        }
    }

    #[test]
    fn validation_rejects_bad_ranges() {
        let rules = vec![MacroRule {
            id: "rule".into(),
            name: "Bad Rule".into(),
            enabled: true,
            trigger_key: "F6".into(),
            steps: vec![MacroStep {
                id: "step".into(),
                key: "A".into(),
                press_duration: HumanizationSettings {
                    enabled: true,
                    min_ms: 10,
                    max_ms: 20,
                },
                humanized_delay: HumanizationSettings {
                    enabled: true,
                    min_ms: 300,
                    max_ms: 100,
                },
            }],
        }];

        let result = validate_rules(&rules);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn disabled_humanized_delay_returns_zero() {
        let settings = HumanizationSettings {
            enabled: false,
            min_ms: 100,
            max_ms: 200,
        };

        assert_eq!(random_delay_ms(&settings), 0);
    }
}
