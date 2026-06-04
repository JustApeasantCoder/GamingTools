use rand::Rng;
use serde::Serialize;

use crate::profiles::{HumanizationSettings, MacroRule, MacroStep};

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
