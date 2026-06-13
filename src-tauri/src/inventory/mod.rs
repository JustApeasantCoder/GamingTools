use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use crate::{
    input,
    macros::random_delay_ms,
    profiles::{InventorySlotSnapshot, InventoryStashRule, PixelPoint},
    screen,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventorySlot {
    pub column: u8,
    pub row: u8,
    pub center: PixelPoint,
}

pub fn send_occupied_slots(
    rule: &InventoryStashRule,
    stop: &Arc<AtomicBool>,
    guard_active: &Arc<AtomicBool>,
) -> Result<usize, String> {
    let slots = inventory_slots(rule)?;
    let ignored = ignored_slot_set(rule);
    let waystone_slots = waystone_slot_set(rule);
    let snapshot_colors = snapshot_color_map(rule);
    let mut sent = 0;

    let mut ctrl_guard = HeldInput::new("CTRL")?;
    if !interruptible_inventory_sleep(random_delay_ms(&rule.humanization), stop, guard_active) {
        return Ok(sent);
    }

    for slot in slots {
        if stop.load(Ordering::Relaxed) || !guard_active.load(Ordering::Relaxed) {
            break;
        }
        if !slot_should_stash(rule, &slot, &ignored, &waystone_slots, &snapshot_colors)? {
            continue;
        }

        if let Err(error) = input::left_click_at(
            slot.center.x,
            slot.center.y,
            input::ClickTiming {
                cursor_settle_ms: random_delay_ms(&rule.humanization),
                click_hold_ms: random_delay_ms(&rule.humanization),
                click_release_settle_ms: random_delay_ms(&rule.humanization),
            },
        ) {
            return Err(error);
        }
        sent += 1;
        wait_for_slot_to_clear(rule, slot.center, stop, guard_active);
        if !interruptible_inventory_sleep(random_delay_ms(&rule.humanization), stop, guard_active) {
            break;
        }
    }

    ctrl_guard.release()?;
    Ok(sent)
}

pub fn test_rule(rule: &InventoryStashRule) -> Result<usize, String> {
    let ignored = ignored_slot_set(rule);
    let waystone_slots = waystone_slot_set(rule);
    let snapshot_colors = snapshot_color_map(rule);
    inventory_slots(rule)?
        .into_iter()
        .try_fold(0usize, |count, slot| {
            slot_should_stash(rule, &slot, &ignored, &waystone_slots, &snapshot_colors)
                .map(|should_stash| count + usize::from(should_stash))
        })
}

pub fn capture_snapshot(rule: &InventoryStashRule) -> Result<Vec<InventorySlotSnapshot>, String> {
    inventory_slots(rule)?
        .into_iter()
        .map(|slot| {
            let sample = screen::sample_pixel(slot.center)?;
            Ok(InventorySlotSnapshot {
                slot: slot_id(slot.column, slot.row),
                color: sample.color,
            })
        })
        .collect()
}

pub fn inventory_slots(rule: &InventoryStashRule) -> Result<Vec<InventorySlot>, String> {
    if rule.columns == 0 || rule.rows == 0 {
        return Err("Inventory grid must have at least one row and column".into());
    }
    if rule.grid.width <= 0 || rule.grid.height <= 0 {
        return Err("Inventory grid width and height must be positive".into());
    }

    let cell_width = rule.grid.width as f32 / rule.columns as f32;
    let cell_height = rule.grid.height as f32 / rule.rows as f32;
    let mut slots = Vec::with_capacity(rule.columns as usize * rule.rows as usize);
    for row in 0..rule.rows {
        for column in 0..rule.columns {
            slots.push(InventorySlot {
                column,
                row,
                center: PixelPoint {
                    x: (rule.grid.x as f32 + cell_width * (column as f32 + 0.5)).round() as i32,
                    y: (rule.grid.y as f32 + cell_height * (row as f32 + 0.5)).round() as i32,
                },
            });
        }
    }

    Ok(slots)
}

pub fn slot_is_occupied(rule: &InventoryStashRule, point: PixelPoint) -> Result<bool, String> {
    let sample = screen::sample_pixel(point)?;
    Ok(!screen::color_matches(
        &sample.color,
        &rule.empty_color,
        rule.tolerance,
    ))
}

pub fn slot_matches_waystone(rule: &InventoryStashRule, point: PixelPoint) -> Result<bool, String> {
    if !rule.ignore_waystone {
        return Ok(false);
    }
    let sample = screen::sample_pixel(point)?;
    Ok(screen::color_matches(
        &sample.color,
        &rule.waystone_color,
        rule.tolerance,
    ))
}

fn slot_should_stash(
    rule: &InventoryStashRule,
    slot: &InventorySlot,
    ignored: &HashSet<(u8, u8)>,
    waystone_slots: &HashSet<(u8, u8)>,
    snapshot_colors: &HashMap<String, String>,
) -> Result<bool, String> {
    if rule.detection_mode == "snapshot" {
        let Some(saved_color) = snapshot_colors.get(&slot_id(slot.column, slot.row)) else {
            return Ok(false);
        };
        let sample = screen::sample_pixel(slot.center)?;
        return Ok(!screen::color_matches(
            &sample.color,
            saved_color,
            rule.tolerance,
        ));
    }

    if ignored.contains(&(slot.column, slot.row)) {
        return Ok(false);
    }
    if waystone_slots.contains(&(slot.column, slot.row))
        && slot_matches_waystone(rule, slot.center)?
    {
        return Ok(false);
    }
    slot_is_occupied(rule, slot.center)
}

fn ignored_slot_set(rule: &InventoryStashRule) -> HashSet<(u8, u8)> {
    slot_set(&rule.ignored_slots)
}

fn waystone_slot_set(rule: &InventoryStashRule) -> HashSet<(u8, u8)> {
    slot_set(&rule.waystone_slots)
}

fn snapshot_color_map(rule: &InventoryStashRule) -> HashMap<String, String> {
    rule.snapshot_colors
        .iter()
        .map(|snapshot| (snapshot.slot.clone(), snapshot.color.clone()))
        .collect()
}

fn slot_id(column: u8, row: u8) -> String {
    format!("{column}:{row}")
}

fn slot_set(slots: &[String]) -> HashSet<(u8, u8)> {
    slots
        .iter()
        .filter_map(|slot| {
            let (column, row) = slot.split_once(':')?;
            Some((column.parse().ok()?, row.parse().ok()?))
        })
        .collect()
}

struct HeldInput {
    key: &'static str,
    released: bool,
}

impl HeldInput {
    fn new(key: &'static str) -> Result<Self, String> {
        input::key_down(key)?;
        Ok(Self {
            key,
            released: false,
        })
    }

    fn release(&mut self) -> Result<(), String> {
        if self.released {
            return Ok(());
        }
        input::key_up(self.key)?;
        self.released = true;
        Ok(())
    }
}

impl Drop for HeldInput {
    fn drop(&mut self) {
        if !self.released {
            let _ = input::key_up(self.key);
            self.released = true;
        }
    }
}

fn interruptible_inventory_sleep(
    duration_ms: u64,
    stop: &Arc<AtomicBool>,
    guard_active: &Arc<AtomicBool>,
) -> bool {
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    while !stop.load(Ordering::Relaxed) && guard_active.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        std::thread::sleep((deadline - now).min(Duration::from_millis(5)));
    }
    false
}

fn wait_for_slot_to_clear(
    rule: &InventoryStashRule,
    point: PixelPoint,
    stop: &Arc<AtomicBool>,
    guard_active: &Arc<AtomicBool>,
) {
    let timeout_ms = random_delay_ms(&rule.humanization);
    if timeout_ms == 0 {
        return;
    }
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while !stop.load(Ordering::Relaxed)
        && guard_active.load(Ordering::Relaxed)
        && Instant::now() < deadline
    {
        match slot_is_occupied(rule, point) {
            Ok(false) => return,
            Ok(true) => {}
            Err(_) => return,
        }
        let poll_ms = random_delay_ms(&rule.humanization).clamp(1, timeout_ms);
        std::thread::sleep(Duration::from_millis(poll_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::{HumanizationSettings, InventoryGrid};

    fn test_rule() -> InventoryStashRule {
        InventoryStashRule {
            id: "rule".into(),
            name: "Inventory".into(),
            enabled: true,
            trigger_key: "F6".into(),
            capture_baseline_key: "F8".into(),
            detection_mode: "emptyColor".into(),
            columns: 12,
            rows: 5,
            grid: InventoryGrid {
                x: 0,
                y: 0,
                width: 120,
                height: 50,
            },
            empty_color: "#101010".into(),
            ignore_waystone: false,
            waystone_color: "#7a52c8".into(),
            tolerance: 10,
            ignored_slots: vec!["0:0".into(), "11:4".into()],
            waystone_slots: vec![],
            snapshot_colors: vec![],
            humanization: HumanizationSettings {
                enabled: true,
                min_ms: 50,
                max_ms: 90,
            },
        }
    }

    #[test]
    fn centers_match_configured_grid() {
        let slots = inventory_slots(&test_rule()).unwrap();

        assert_eq!(slots.len(), 60);
        assert_eq!(slots[0].center, PixelPoint { x: 5, y: 5 });
        assert_eq!(slots[59].center, PixelPoint { x: 115, y: 45 });
    }

    #[test]
    fn rejects_invalid_grid_size() {
        let mut rule = test_rule();
        rule.grid.width = 0;

        assert!(inventory_slots(&rule).is_err());
    }

    #[test]
    fn ignored_slots_parse_column_row_pairs() {
        let ignored = ignored_slot_set(&test_rule());

        assert!(ignored.contains(&(0, 0)));
        assert!(ignored.contains(&(11, 4)));
        assert!(!ignored.contains(&(1, 0)));
    }
}
