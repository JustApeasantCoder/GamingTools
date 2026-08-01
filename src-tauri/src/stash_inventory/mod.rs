use std::sync::{atomic::AtomicBool, Arc};

use crate::{
    inventory,
    profiles::{InventoryStashRule, StashInventoryRule},
};

pub fn send_occupied_slots(
    rule: &StashInventoryRule,
    stop: &Arc<AtomicBool>,
    guard_active: &Arc<AtomicBool>,
) -> Result<usize, String> {
    inventory::send_occupied_slots(&as_inventory_rule(rule), stop, guard_active)
}

pub fn test_rule(rule: &StashInventoryRule) -> Result<usize, String> {
    inventory::test_rule(&as_inventory_rule(rule))
}

fn as_inventory_rule(rule: &StashInventoryRule) -> InventoryStashRule {
    InventoryStashRule {
        id: rule.id.clone(),
        name: rule.name.clone(),
        enabled: rule.enabled,
        trigger_key: rule.trigger_key.clone(),
        capture_baseline_key: String::new(),
        detection_mode: "emptyColor".into(),
        columns: rule.columns,
        rows: rule.rows,
        grid: rule.grid.clone(),
        empty_color: rule.empty_color.clone(),
        ignore_waystone: false,
        waystone_color: "#000000".into(),
        tolerance: rule.tolerance,
        ignored_slots: Vec::new(),
        waystone_slots: Vec::new(),
        snapshot_colors: Vec::new(),
        humanization: rule.humanization.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::{HumanizationSettings, InventoryGrid};

    fn test_rule() -> StashInventoryRule {
        StashInventoryRule {
            id: "stash-rule".into(),
            name: "Stash to inventory".into(),
            enabled: true,
            trigger_key: "F10".into(),
            columns: 4,
            rows: 3,
            grid: InventoryGrid {
                x: 100,
                y: 200,
                width: 80,
                height: 60,
            },
            empty_color: "#17130f".into(),
            tolerance: 12,
            humanization: HumanizationSettings {
                enabled: true,
                min_ms: 50,
                max_ms: 90,
            },
        }
    }

    #[test]
    fn reverse_rule_uses_every_configured_stash_slot() {
        let converted = as_inventory_rule(&test_rule());
        let slots = inventory::inventory_slots(&converted).unwrap();

        assert_eq!(slots.len(), 12);
        assert_eq!(slots[0].center.x, 110);
        assert_eq!(slots[0].center.y, 210);
        assert_eq!(slots[11].center.x, 170);
        assert_eq!(slots[11].center.y, 250);
        assert!(converted.ignored_slots.is_empty());
        assert!(converted.waystone_slots.is_empty());
    }

    #[test]
    fn reverse_rule_rejects_an_empty_grid_dimension() {
        let mut rule = test_rule();
        rule.rows = 0;

        assert!(inventory::inventory_slots(&as_inventory_rule(&rule)).is_err());
    }
}
