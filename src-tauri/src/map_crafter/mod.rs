use serde::Serialize;
use std::{thread, time::Duration};

use crate::{
    foreground, input,
    profiles::{MapCrafterRule, PixelPoint, ScreenPoint},
    screen,
};

const EMPTY_MAP_SLOT_COLOR: &str = "#000000";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapScanReport {
    pub scanned_slots: usize,
    pub maps: Vec<MapScanItem>,
    pub skipped_slots: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapScanItem {
    pub slot: String,
    pub column: u8,
    pub row: u8,
    pub name: Option<String>,
    pub item_type: String,
    pub rarity: String,
    pub raw_text: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapCraftReport {
    pub initial_scan: MapScanReport,
    pub actions: Vec<MapCraftAction>,
    pub crafted_slots: Vec<String>,
    pub skipped_slots: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapCraftAction {
    pub slot: String,
    pub currency: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MapSlot {
    column: u8,
    row: u8,
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CurrencyKind {
    Scouring,
    Alchemy,
    Exalted,
}

impl CurrencyKind {
    fn label(self) -> &'static str {
        match self {
            Self::Scouring => "scouring",
            Self::Alchemy => "alchemy",
            Self::Exalted => "exalted",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CraftPlan {
    magic_slots: Vec<String>,
    alchemy_slots: Vec<String>,
    exalted_slots: Vec<String>,
    skipped_slots: Vec<String>,
}

pub fn scan(rule: &MapCrafterRule) -> Result<MapScanReport, String> {
    validate_scan_settings(rule)?;
    let _cursor_restore = CursorRestore::capture();
    scan_without_restore(rule)
}

pub fn craft(rule: &MapCrafterRule) -> Result<MapCraftReport, String> {
    validate_craft_settings(rule)?;
    let _cursor_restore = CursorRestore::capture();
    let initial_scan = scan_without_restore(rule)?;
    let plan = craft_plan(&initial_scan);
    let mut actions = Vec::new();

    apply_currency(
        rule,
        &plan.magic_slots,
        CurrencyKind::Scouring,
        "magic map reset to normal rarity",
        1,
        &mut actions,
    )?;
    apply_currency(
        rule,
        &plan.alchemy_slots,
        CurrencyKind::Alchemy,
        "first map crafting step",
        1,
        &mut actions,
    )?;
    apply_currency(
        rule,
        &plan.exalted_slots,
        CurrencyKind::Exalted,
        "Exalted application",
        2,
        &mut actions,
    )?;

    Ok(MapCraftReport {
        initial_scan,
        actions,
        crafted_slots: plan.exalted_slots,
        skipped_slots: plan.skipped_slots,
    })
}

pub fn capture_cursor_location(wait_ms: u64) -> Result<ScreenPoint, String> {
    thread::sleep(Duration::from_millis(wait_ms.clamp(250, 10_000)));
    let (x, y) = input::cursor_position()?;
    Ok(ScreenPoint { x, y })
}

fn scan_without_restore(rule: &MapCrafterRule) -> Result<MapScanReport, String> {
    let slots = map_slots(rule)?;
    let delay = scan_delay(rule);
    let mut maps = Vec::new();
    let mut skipped_slots = Vec::new();

    foreground::focus_executable(&rule.target_executable)?;
    thread::sleep(delay);

    let (parking_x, parking_y) = tooltip_parking_point(rule);
    input::move_cursor_to(parking_x, parking_y)?;
    thread::sleep(tooltip_dismiss_delay(rule));

    let mut occupied_slots = Vec::with_capacity(slots.len());
    for slot in &slots {
        if slot_looks_empty(slot, rule.empty_tolerance) {
            skipped_slots.push(slot_id(slot.column, slot.row));
        } else {
            occupied_slots.push(*slot);
        }
    }

    for slot in &occupied_slots {
        input::move_cursor_to(slot.x, slot.y)?;
        thread::sleep(delay);
        clipboard::clear_clipboard()?;
        copy_hovered_item()?;
        thread::sleep(delay);
        let text = clipboard::read_clipboard_text()?;
        match parse_map_text(&text, slot.column, slot.row) {
            Some(map) => maps.push(map),
            None => skipped_slots.push(slot_id(slot.column, slot.row)),
        }
    }

    Ok(MapScanReport {
        scanned_slots: slots.len(),
        maps,
        skipped_slots,
    })
}

fn tooltip_parking_point(rule: &MapCrafterRule) -> (i32, i32) {
    (
        rule.grid.x.saturating_add(rule.grid.width / 2),
        rule.grid
            .y
            .saturating_add(rule.grid.height)
            .saturating_add(16),
    )
}

fn tooltip_dismiss_delay(rule: &MapCrafterRule) -> Duration {
    Duration::from_millis(rule.scan_delay_ms.clamp(120, 1_000))
}

fn slot_looks_empty(slot: &MapSlot, tolerance: u8) -> bool {
    match screen::sample_pixel(PixelPoint {
        x: slot.x,
        y: slot.y,
    }) {
        Ok(sample) => is_empty_slot_color(&sample.color, tolerance),
        Err(error) => {
            log::warn!(
                "Map crafter could not sample slot {}: {error}",
                slot_id(slot.column, slot.row)
            );
            false
        }
    }
}

fn is_empty_slot_color(color: &str, tolerance: u8) -> bool {
    screen::color_matches(color, EMPTY_MAP_SLOT_COLOR, tolerance)
}

fn parse_map_text(text: &str, column: u8, row: u8) -> Option<MapScanItem> {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let item_class = value_after_prefix(&lines, "Item Class:")?;
    let class = item_class.to_ascii_lowercase();
    if !class.contains("map") && !class.contains("waystone") {
        return None;
    }

    let rarity = value_after_prefix(&lines, "Rarity:").unwrap_or_else(|| "Unknown".into());
    let rarity_index = lines.iter().position(|line| line.starts_with("Rarity:"));
    let item_lines = rarity_index
        .map(|index| {
            lines
                .iter()
                .skip(index + 1)
                .take_while(|line| !line.chars().all(|character| character == '-'))
                .filter(|line| !line.starts_with("Item Class:"))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let item_type = item_lines
        .last()
        .cloned()
        .unwrap_or_else(|| item_class.clone());
    let name = (rarity.eq_ignore_ascii_case("unique") && item_lines.len() > 1)
        .then(|| item_lines[0].clone());

    Some(MapScanItem {
        slot: slot_id(column, row),
        column,
        row,
        name,
        item_type,
        rarity,
        raw_text: text.to_string(),
    })
}

fn value_after_prefix(lines: &[String], prefix: &str) -> Option<String> {
    lines.iter().find_map(|line| {
        line.strip_prefix(prefix)
            .map(|value| value.trim().to_string())
    })
}

fn craft_plan(report: &MapScanReport) -> CraftPlan {
    let magic_slots = report
        .maps
        .iter()
        .filter(|map| map.rarity.eq_ignore_ascii_case("magic"))
        .map(|map| map.slot.clone())
        .collect::<Vec<_>>();
    let alchemy_slots = report
        .maps
        .iter()
        .filter(|map| {
            map.rarity.eq_ignore_ascii_case("normal") || map.rarity.eq_ignore_ascii_case("magic")
        })
        .map(|map| map.slot.clone())
        .collect::<Vec<_>>();
    let exalted_slots = report
        .maps
        .iter()
        .filter(|map| {
            map.rarity.eq_ignore_ascii_case("normal")
                || map.rarity.eq_ignore_ascii_case("magic")
                || map.rarity.eq_ignore_ascii_case("rare")
        })
        .map(|map| map.slot.clone())
        .collect::<Vec<_>>();
    let mut skipped_slots = report.skipped_slots.clone();
    skipped_slots.extend(
        report
            .maps
            .iter()
            .filter(|map| {
                !map.rarity.eq_ignore_ascii_case("normal")
                    && !map.rarity.eq_ignore_ascii_case("magic")
                    && !map.rarity.eq_ignore_ascii_case("rare")
            })
            .map(|map| map.slot.clone()),
    );

    CraftPlan {
        magic_slots,
        alchemy_slots,
        exalted_slots,
        skipped_slots,
    }
}

fn apply_currency(
    rule: &MapCrafterRule,
    slot_ids: &[String],
    currency: CurrencyKind,
    reason: &str,
    applications_per_slot: u8,
    actions: &mut Vec<MapCraftAction>,
) -> Result<(), String> {
    if slot_ids.is_empty() {
        return Ok(());
    }

    let slots = map_slots(rule)?;
    let targets = slot_ids
        .iter()
        .map(|slot_id| {
            let (column, row) = parse_slot_id(slot_id)?;
            slots
                .iter()
                .find(|slot| slot.column == column && slot.row == row)
                .copied()
                .ok_or_else(|| format!("Slot is outside the map crafter grid: {slot_id}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    foreground::focus_executable(&rule.target_executable)?;
    thread::sleep(scan_delay(rule));
    let point = currency_point(rule, currency);
    input::right_click_at(point.x, point.y, click_timing(rule))?;
    thread::sleep(craft_delay(rule));
    shift_left_click_slots(
        rule,
        &targets,
        currency,
        reason,
        applications_per_slot,
        actions,
    )
}

fn shift_left_click_slots(
    rule: &MapCrafterRule,
    targets: &[MapSlot],
    currency: CurrencyKind,
    reason: &str,
    applications_per_slot: u8,
    actions: &mut Vec<MapCraftAction>,
) -> Result<(), String> {
    input::key_down("SHIFT")?;
    let mut result = Ok(());

    for (slot, application) in repeated_targets(targets, applications_per_slot) {
        if let Err(error) = input::left_click_at(slot.x, slot.y, click_timing(rule)) {
            result = Err(error);
            break;
        }
        actions.push(MapCraftAction {
            slot: slot_id(slot.column, slot.row),
            currency: currency.label().into(),
            reason: application_reason(reason, application, applications_per_slot),
        });
    }

    match (result, input::key_up("SHIFT")) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(release_error)) => Err(format!(
            "Unable to release Shift after map crafting: {release_error}"
        )),
        (Err(click_error), Ok(())) => Err(click_error),
        (Err(click_error), Err(release_error)) => Err(format!(
            "{click_error}; also unable to release Shift after map crafting: {release_error}"
        )),
    }
}

fn repeated_targets(targets: &[MapSlot], applications_per_slot: u8) -> Vec<(MapSlot, u8)> {
    targets
        .iter()
        .flat_map(|slot| (1..=applications_per_slot).map(move |application| (*slot, application)))
        .collect()
}

fn application_reason(reason: &str, application: u8, applications_per_slot: u8) -> String {
    if applications_per_slot <= 1 {
        return reason.into();
    }
    let ordinal = match application {
        1 => "first",
        2 => "second",
        3 => "third",
        _ => "next",
    };
    format!("{ordinal} {reason}")
}

fn validate_scan_settings(rule: &MapCrafterRule) -> Result<(), String> {
    if rule.target_executable.trim().is_empty() {
        return Err("Target app has not been set".into());
    }
    map_slots(rule).map(|_| ())
}

fn validate_craft_settings(rule: &MapCrafterRule) -> Result<(), String> {
    validate_scan_settings(rule)?;
    for (label, point) in [
        ("Alchemy", rule.craft.alchemy),
        ("Exalted", rule.craft.exalted),
        ("Scouring", rule.craft.scouring),
    ] {
        if point.x == 0 && point.y == 0 {
            return Err(format!("{label} currency location has not been picked"));
        }
    }
    if rule.craft.craft_delay_ms > 5_000 {
        return Err("Craft wait is too high".into());
    }
    Ok(())
}

fn map_slots(rule: &MapCrafterRule) -> Result<Vec<MapSlot>, String> {
    if rule.columns == 0 || rule.rows == 0 {
        return Err("Map crafter grid must have at least one row and column".into());
    }
    if rule.grid.width <= 0 || rule.grid.height <= 0 {
        return Err("Map crafter grid width and height must be positive".into());
    }

    let cell_width = rule.grid.width as f32 / rule.columns as f32;
    let cell_height = rule.grid.height as f32 / rule.rows as f32;
    let mut slots = Vec::with_capacity(rule.columns as usize * rule.rows as usize);
    for row in 0..rule.rows {
        for column in 0..rule.columns {
            slots.push(MapSlot {
                column,
                row,
                x: (rule.grid.x as f32 + cell_width * (column as f32 + 0.5)).round() as i32,
                y: (rule.grid.y as f32 + cell_height * (row as f32 + 0.5)).round() as i32,
            });
        }
    }
    Ok(slots)
}

fn copy_hovered_item() -> Result<(), String> {
    input::key_down("CTRL")?;
    let result = input::key_down("C")
        .and_then(|_| input::key_up("C"))
        .and_then(|_| input::key_up("CTRL"));
    if result.is_err() {
        let _ = input::key_up("C");
        let _ = input::key_up("CTRL");
    }
    result
}

fn currency_point(rule: &MapCrafterRule, currency: CurrencyKind) -> ScreenPoint {
    match currency {
        CurrencyKind::Scouring => rule.craft.scouring,
        CurrencyKind::Alchemy => rule.craft.alchemy,
        CurrencyKind::Exalted => rule.craft.exalted,
    }
}

fn click_timing(rule: &MapCrafterRule) -> input::ClickTiming {
    let delay_ms = rule.craft.craft_delay_ms.clamp(20, 2_000);
    input::ClickTiming {
        cursor_settle_ms: delay_ms,
        click_hold_ms: 40,
        click_release_settle_ms: delay_ms,
    }
}

fn scan_delay(rule: &MapCrafterRule) -> Duration {
    Duration::from_millis(rule.scan_delay_ms.clamp(20, 1_000))
}

fn craft_delay(rule: &MapCrafterRule) -> Duration {
    Duration::from_millis(rule.craft.craft_delay_ms.clamp(20, 2_000))
}

fn slot_id(column: u8, row: u8) -> String {
    format!("{column}:{row}")
}

fn parse_slot_id(slot: &str) -> Result<(u8, u8), String> {
    let (column, row) = slot
        .split_once(':')
        .ok_or_else(|| format!("Invalid slot id: {slot}"))?;
    Ok((
        column
            .parse()
            .map_err(|_| format!("Invalid slot column: {slot}"))?,
        row.parse()
            .map_err(|_| format!("Invalid slot row: {slot}"))?,
    ))
}

struct CursorRestore(Option<(i32, i32)>);

impl CursorRestore {
    fn capture() -> Self {
        Self(input::cursor_position().ok())
    }
}

impl Drop for CursorRestore {
    fn drop(&mut self) {
        if let Some((x, y)) = self.0 {
            let _ = input::move_cursor_to(x, y);
        }
    }
}

#[cfg(windows)]
mod clipboard {
    use std::{slice, thread, time::Duration};
    use windows_sys::Win32::System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
            OpenClipboard,
        },
        Memory::{GlobalLock, GlobalUnlock},
    };

    const CF_UNICODETEXT: u32 = 13;
    const OPEN_CLIPBOARD_ATTEMPTS: usize = 12;
    const OPEN_CLIPBOARD_RETRY_MS: u64 = 12;

    pub fn clear_clipboard() -> Result<(), String> {
        let _guard = ClipboardGuard::open()?;
        let emptied = unsafe { EmptyClipboard() };
        if emptied == 0 {
            Err(format!(
                "Unable to clear clipboard: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(())
        }
    }

    pub fn read_clipboard_text() -> Result<String, String> {
        let _guard = ClipboardGuard::open()?;
        let available = unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) };
        if available == 0 {
            return Ok(String::new());
        }
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
        if handle.is_null() {
            return Ok(String::new());
        }
        let pointer = unsafe { GlobalLock(handle) } as *const u16;
        if pointer.is_null() {
            return Err("Unable to read clipboard text".into());
        }
        let mut len = 0usize;
        unsafe {
            while *pointer.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(slice::from_raw_parts(pointer, len));
            let _ = GlobalUnlock(handle);
            Ok(text)
        }
    }

    struct ClipboardGuard;

    impl ClipboardGuard {
        fn open() -> Result<Self, String> {
            let mut last_error = None;
            for attempt in 0..OPEN_CLIPBOARD_ATTEMPTS {
                let opened = unsafe { OpenClipboard(std::ptr::null_mut()) };
                if opened != 0 {
                    return Ok(Self);
                }
                last_error = Some(std::io::Error::last_os_error());
                if attempt + 1 < OPEN_CLIPBOARD_ATTEMPTS {
                    thread::sleep(Duration::from_millis(OPEN_CLIPBOARD_RETRY_MS));
                }
            }
            Err(format!(
                "Unable to open clipboard after {OPEN_CLIPBOARD_ATTEMPTS} attempts: {}",
                last_error
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "unknown clipboard error".into())
            ))
        }
    }

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
}

#[cfg(not(windows))]
mod clipboard {
    pub fn clear_clipboard() -> Result<(), String> {
        Err("Map scanning is only supported on Windows".into())
    }

    pub fn read_clipboard_text() -> Result<String, String> {
        Err("Map scanning is only supported on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_item(slot: &str, rarity: &str) -> MapScanItem {
        let (column, row) = parse_slot_id(slot).unwrap();
        MapScanItem {
            slot: slot.into(),
            column,
            row,
            name: None,
            item_type: "Waystone (Tier 16)".into(),
            rarity: rarity.into(),
            raw_text: String::new(),
        }
    }

    #[test]
    fn parses_waystones_and_maps_but_not_other_inventory_items() {
        let waystone = parse_map_text(
            "Item Class: Waystones\nRarity: Magic\nWaystone (Tier 16)",
            2,
            3,
        )
        .unwrap();
        let map = parse_map_text("Item Class: Maps\nRarity: Normal\nBeach Map", 4, 5).unwrap();

        assert_eq!(waystone.slot, "2:3");
        assert_eq!(waystone.rarity, "Magic");
        assert_eq!(map.item_type, "Beach Map");
        assert!(parse_map_text("Item Class: Currency\nRarity: Normal\nChaos Orb", 0, 0).is_none());
    }

    #[test]
    fn black_slot_center_counts_as_empty() {
        assert!(is_empty_slot_color("#000000", 8));
        assert!(is_empty_slot_color("#070605", 8));
        assert!(!is_empty_slot_color("#17130f", 8));
        assert!(is_empty_slot_color("#17130f", 24));
    }

    #[test]
    fn tooltip_parking_point_sits_below_the_grid() {
        let rule = MapCrafterRule {
            id: "map".into(),
            name: "Map crafter".into(),
            target_executable: "Game.exe".into(),
            columns: 12,
            rows: 6,
            grid: crate::profiles::InventoryGrid {
                x: 40,
                y: 80,
                width: 600,
                height: 300,
            },
            scan_delay_ms: 90,
            empty_tolerance: 8,
            craft: Default::default(),
        };

        assert_eq!(tooltip_parking_point(&rule), (340, 396));
        assert_eq!(tooltip_dismiss_delay(&rule), Duration::from_millis(120));
    }

    #[test]
    fn plan_scours_magic_alchemizes_normal_and_magic_and_exalts_rare_maps() {
        let report = MapScanReport {
            scanned_slots: 5,
            maps: vec![
                scan_item("0:0", "Normal"),
                scan_item("1:0", "Magic"),
                scan_item("2:0", "Rare"),
                scan_item("3:0", "Unique"),
            ],
            skipped_slots: vec!["4:0".into()],
        };

        let plan = craft_plan(&report);

        assert_eq!(plan.magic_slots, vec!["1:0"]);
        assert_eq!(plan.alchemy_slots, vec!["0:0", "1:0"]);
        assert_eq!(plan.exalted_slots, vec!["0:0", "1:0", "2:0"]);
        assert_eq!(plan.skipped_slots, vec!["4:0", "3:0"]);
    }

    #[test]
    fn repeated_applications_finish_each_map_before_moving_to_the_next() {
        let targets = vec![
            MapSlot {
                column: 0,
                row: 0,
                x: 100,
                y: 200,
            },
            MapSlot {
                column: 1,
                row: 0,
                x: 150,
                y: 200,
            },
        ];

        let applications = repeated_targets(&targets, 2)
            .into_iter()
            .map(|(slot, application)| (slot.column, slot.row, application))
            .collect::<Vec<_>>();

        assert_eq!(
            applications,
            vec![(0, 0, 1), (0, 0, 2), (1, 0, 1), (1, 0, 2)]
        );
    }
}
