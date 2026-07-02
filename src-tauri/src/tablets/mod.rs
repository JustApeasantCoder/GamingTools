use serde::Serialize;
use std::{thread, time::Duration};

use crate::{
    foreground, input,
    profiles::{PixelPoint, ScreenPoint, TabletScannerRule, TabletValueRuleConfig},
    screen,
};

const EMPTY_TABLET_SLOT_COLOR: &str = "#000000";
const EMPTY_TABLET_SLOT_TOLERANCE: u8 = 8;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TabletScanReport {
    pub scanned_slots: usize,
    pub tablets: Vec<TabletScanItem>,
    pub skipped_slots: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TabletScanItem {
    pub slot: String,
    pub column: u8,
    pub row: u8,
    pub name: Option<String>,
    pub tablet_type: String,
    pub rarity: String,
    pub uses_remaining: Option<u8>,
    pub value_tier: String,
    pub value_score: u16,
    pub prefixes: Vec<TabletValueMod>,
    pub suffixes: Vec<TabletValueMod>,
    pub unknown_mods: Vec<String>,
    pub reasons: Vec<String>,
    pub raw_text: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TabletValueMod {
    pub text: String,
    pub affix_type: String,
    pub tier: String,
    pub score: u16,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TabletCraftReport {
    pub initial_scan: TabletScanReport,
    pub final_scan: TabletScanReport,
    pub actions: Vec<TabletCraftAction>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TabletCraftAction {
    pub slot: String,
    pub currency: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScannerSlot {
    column: u8,
    row: u8,
    x: i32,
    y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AffixType {
    Prefix,
    Suffix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueTier {
    S,
    A,
    B,
}

pub fn scan_stash(rule: &TabletScannerRule) -> Result<TabletScanReport, String> {
    let _cursor_restore = CursorRestore::capture();
    let slots = scanner_slots(rule)?;
    let mut tablets = Vec::new();
    let mut skipped_slots = Vec::new();
    let delay = Duration::from_millis(rule.scan_delay_ms.clamp(20, 1_000));

    foreground::focus_executable(&rule.target_executable)?;
    thread::sleep(delay);

    for slot in &slots {
        if slot_looks_empty(slot) {
            skipped_slots.push(slot_id(slot.column, slot.row));
            continue;
        }
        input::move_cursor_to(slot.x, slot.y)?;
        thread::sleep(delay);
        clipboard::clear_clipboard()?;
        copy_hovered_item()?;
        thread::sleep(delay);
        let text = clipboard::read_clipboard_text()?;
        if text.trim().is_empty() {
            skipped_slots.push(slot_id(slot.column, slot.row));
            continue;
        }
        match parse_tablet_text_with_rules(&text, slot.column, slot.row, &rule.value_rules) {
            Some(tablet) => tablets.push(tablet),
            None => skipped_slots.push(slot_id(slot.column, slot.row)),
        }
    }

    tablets.sort_by(|a, b| {
        b.value_score
            .cmp(&a.value_score)
            .then_with(|| a.slot.cmp(&b.slot))
    });

    Ok(TabletScanReport {
        scanned_slots: slots.len(),
        tablets,
        skipped_slots,
    })
}

pub fn capture_cursor_location(wait_ms: u64) -> Result<ScreenPoint, String> {
    thread::sleep(Duration::from_millis(wait_ms.clamp(250, 10_000)));
    let (x, y) = input::cursor_position()?;
    Ok(ScreenPoint { x, y })
}

pub fn scan_and_craft(rule: &TabletScannerRule) -> Result<TabletCraftReport, String> {
    validate_craft_settings(rule)?;
    let _cursor_restore = CursorRestore::capture();
    let initial_scan = scan_stash(rule)?;
    let mut actions = Vec::new();

    let normal_slots = transmutation_slots(&initial_scan);
    craft_currency_for_slots(
        rule,
        &normal_slots,
        CurrencyKind::Transmutation,
        "normal tablet",
        &mut actions,
    )?;

    let augment_slots = augmentation_slots(&initial_scan);
    craft_currency_for_slots(
        rule,
        &augment_slots,
        CurrencyKind::Augmentation,
        "normal or one-modifier magic tablet",
        &mut actions,
    )?;

    let after_magic_scan = if normal_slots.is_empty() && augment_slots.is_empty() {
        initial_scan.clone()
    } else {
        scan_stash(rule)?
    };

    let alchemy_slots = alchemy_slots(&after_magic_scan);
    craft_currency_for_slots(
        rule,
        &alchemy_slots,
        CurrencyKind::Alchemy,
        "magic tablet without an A-tier modifier",
        &mut actions,
    )?;

    let rare_exalted_slots = rare_exalted_slots(&after_magic_scan);
    craft_currency_for_slots(
        rule,
        &rare_exalted_slots,
        CurrencyKind::Exalted,
        "rare tablet with three modifiers",
        &mut actions,
    )?;

    let regal_slots = regal_slots(&after_magic_scan);
    craft_currency_for_slots(
        rule,
        &regal_slots,
        CurrencyKind::Regal,
        "magic tablet with an A-tier modifier",
        &mut actions,
    )?;
    craft_currency_for_slots(
        rule,
        &regal_slots,
        CurrencyKind::Exalted,
        "regaled tablet with a protected A-tier modifier",
        &mut actions,
    )?;

    let final_scan = scan_stash(rule)?;
    Ok(TabletCraftReport {
        initial_scan,
        final_scan,
        actions,
    })
}

pub fn highlight_slot(rule: &TabletScannerRule, slot: &str) -> Result<(), String> {
    let (column, row) = parse_slot_id(slot)?;
    let target = scanner_slots(rule)?
        .into_iter()
        .find(|item| item.column == column && item.row == row)
        .ok_or_else(|| format!("Slot is outside the tablet scanner grid: {slot}"))?;
    foreground::focus_executable(&rule.target_executable)?;
    thread::sleep(Duration::from_millis(rule.scan_delay_ms.clamp(20, 1_000)));
    input::move_cursor_to(target.x, target.y)
}

pub fn move_slot_to_inventory(rule: &TabletScannerRule, slot: &str) -> Result<(), String> {
    let _cursor_restore = CursorRestore::capture();
    let (column, row) = parse_slot_id(slot)?;
    let target = scanner_slots(rule)?
        .into_iter()
        .find(|item| item.column == column && item.row == row)
        .ok_or_else(|| format!("Slot is outside the tablet scanner grid: {slot}"))?;
    let delay_ms = rule.scan_delay_ms.clamp(20, 1_000);
    foreground::focus_executable(&rule.target_executable)?;
    thread::sleep(Duration::from_millis(delay_ms));

    input::key_down("CTRL")?;
    let result = input::left_click_at(
        target.x,
        target.y,
        input::ClickTiming {
            cursor_settle_ms: delay_ms,
            click_hold_ms: 40,
            click_release_settle_ms: delay_ms,
        },
    )
    .and_then(|_| input::key_up("CTRL"));
    if result.is_err() {
        let _ = input::key_up("CTRL");
    }
    result
}

struct CursorRestore {
    position: Option<(i32, i32)>,
}

impl CursorRestore {
    fn capture() -> Self {
        Self {
            position: input::cursor_position().ok(),
        }
    }
}

impl Drop for CursorRestore {
    fn drop(&mut self) {
        if let Some((x, y)) = self.position {
            let _ = input::move_cursor_to(x, y);
        }
    }
}

#[cfg(test)]
pub fn parse_tablet_text(text: &str, column: u8, row: u8) -> Option<TabletScanItem> {
    parse_tablet_text_with_rules(text, column, row, &[])
}

fn parse_tablet_text_with_rules(
    text: &str,
    column: u8,
    row: u8,
    custom_rules: &[TabletValueRuleConfig],
) -> Option<TabletScanItem> {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "--------")
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !lines.iter().any(|line| line.contains("Tablet")) {
        return None;
    }

    let rarity = value_after_prefix(&lines, "Rarity:").unwrap_or_else(|| "Unknown".into());
    let tablet_type = lines
        .iter()
        .rev()
        .find(|line| line.contains("Tablet") && !line.starts_with("Item Class:"))
        .cloned()?;
    let name = item_name(&lines, &tablet_type, &rarity);
    let uses_remaining = lines
        .iter()
        .find(|line| line.to_ascii_lowercase().contains("uses remaining"))
        .and_then(|line| first_number(line))
        .and_then(|value| u8::try_from(value).ok());

    let mut prefixes = Vec::new();
    let mut suffixes = Vec::new();
    let mut unknown_mods = Vec::new();
    for line in likely_modifier_lines(&lines, &tablet_type, custom_rules) {
        if let Some(classified) = classify_modifier(line, &tablet_type, custom_rules) {
            match classified.affix_type.as_str() {
                "prefix" => prefixes.push(classified),
                "suffix" => suffixes.push(classified),
                _ => unknown_mods.push(line.to_string()),
            }
        } else {
            unknown_mods.push(line.to_string());
        }
    }

    let (value_score, value_tier, reasons) =
        score_tablet(&tablet_type, uses_remaining, &prefixes, &suffixes);

    Some(TabletScanItem {
        slot: slot_id(column, row),
        column,
        row,
        name,
        tablet_type,
        rarity,
        uses_remaining,
        value_tier,
        value_score,
        prefixes,
        suffixes,
        unknown_mods,
        reasons,
        raw_text: text.to_string(),
    })
}

fn slot_looks_empty(slot: &ScannerSlot) -> bool {
    match screen::sample_pixel(PixelPoint {
        x: slot.x,
        y: slot.y,
    }) {
        Ok(sample) => is_empty_slot_color(&sample.color),
        Err(error) => {
            log::warn!(
                "Tablet scanner could not sample slot {}: {error}",
                slot_id(slot.column, slot.row)
            );
            false
        }
    }
}

fn is_empty_slot_color(color: &str) -> bool {
    screen::color_matches(color, EMPTY_TABLET_SLOT_COLOR, EMPTY_TABLET_SLOT_TOLERANCE)
}

fn scanner_slots(rule: &TabletScannerRule) -> Result<Vec<ScannerSlot>, String> {
    if rule.columns == 0 || rule.rows == 0 {
        return Err("Tablet scanner grid must have at least one row and column".into());
    }
    if rule.grid.width <= 0 || rule.grid.height <= 0 {
        return Err("Tablet scanner grid width and height must be positive".into());
    }

    let cell_width = rule.grid.width as f32 / rule.columns as f32;
    let cell_height = rule.grid.height as f32 / rule.rows as f32;
    let mut slots = Vec::with_capacity(rule.columns as usize * rule.rows as usize);
    for row in 0..rule.rows {
        for column in 0..rule.columns {
            slots.push(ScannerSlot {
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

fn item_name(lines: &[String], tablet_type: &str, rarity: &str) -> Option<String> {
    if rarity != "Unique" {
        return None;
    }
    let type_index = lines.iter().position(|line| line == tablet_type)?;
    lines
        .get(type_index.saturating_sub(1))
        .filter(|name| *name != tablet_type && !name.starts_with("Rarity:"))
        .cloned()
}

fn value_after_prefix(lines: &[String], prefix: &str) -> Option<String> {
    lines.iter().find_map(|line| {
        line.strip_prefix(prefix)
            .map(|value| value.trim().to_string())
    })
}

fn likely_modifier_lines<'a>(
    lines: &'a [String],
    tablet_type: &'a str,
    custom_rules: &'a [TabletValueRuleConfig],
) -> impl Iterator<Item = &'a str> {
    let modifier_start = lines
        .iter()
        .position(|line| line.to_ascii_lowercase().contains("uses remaining"))
        .map(|index| index + 1);

    lines.iter().enumerate().filter_map(move |(index, line)| {
        let lowered = line.to_ascii_lowercase();
        if ignored_item_text_line(line, tablet_type) {
            None
        } else if modifier_start.is_some_and(|start| index >= start)
            || modifier_words(&lowered)
            || custom_modifier_match(&lowered, custom_rules)
        {
            Some(line.as_str())
        } else {
            None
        }
    })
}

fn ignored_item_text_line(line: &str, tablet_type: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    lowered.starts_with("item class:")
        || lowered.starts_with("rarity:")
        || lowered.contains("uses remaining")
        || lowered.contains("place into")
        || lowered.contains("can be used")
        || lowered.contains("requires")
        || lowered.contains("right click")
        || lowered.contains("shift click")
        || lowered.contains("unidentified")
        || lowered == "corrupted"
        || line == tablet_type
}

fn custom_modifier_match(line: &str, custom_rules: &[TabletValueRuleConfig]) -> bool {
    custom_rules.iter().any(|rule| {
        let text_match = rule.text_match.trim().to_ascii_lowercase();
        !text_match.is_empty() && line.contains(&text_match)
    })
}

fn modifier_words(line: &str) -> bool {
    [
        "map",
        "abyss",
        "breach",
        "ritual",
        "delirium",
        "waystone",
        "wombgift",
        "hiveblood",
        "boss",
        "omen",
        "favour",
        "tribute",
        "monster",
        "modifier",
        "effectiveness",
        "quantity",
    ]
    .iter()
    .any(|word| line.contains(word))
}

fn classify_modifier(
    text: &str,
    tablet_type: &str,
    custom_rules: &[TabletValueRuleConfig],
) -> Option<TabletValueMod> {
    let normalized = text.to_ascii_lowercase();
    let tablet = tablet_type.to_ascii_lowercase();
    let mut best: Option<(AffixType, ValueTier, u16)> = None;

    for rule in custom_rules {
        let text_match = rule.text_match.trim().to_ascii_lowercase();
        if text_match.is_empty() || !normalized.contains(&text_match) {
            continue;
        }
        let tablet_match = rule.tablet_match.trim().to_ascii_lowercase();
        if !tablet_match.is_empty() && !tablet.contains(&tablet_match) {
            continue;
        }
        let Some(affix_type) = parse_affix_type(&rule.affix_type) else {
            continue;
        };
        let Some(tier) = parse_value_tier(&rule.tier) else {
            continue;
        };
        let score = rule.score + high_roll_bonus(text, rule.high_roll_at);
        if best
            .as_ref()
            .map(|(_, _, current_score)| score > *current_score)
            .unwrap_or(true)
        {
            best = Some((affix_type, tier, score));
        }
    }

    for rule in value_rules() {
        if !rule.applies_to_tablet(&tablet) || !rule.matches(&normalized) {
            continue;
        }
        let score = rule.score + high_roll_bonus(text, rule.high_roll_at);
        if best
            .as_ref()
            .map(|(_, _, current_score)| score > *current_score)
            .unwrap_or(true)
        {
            best = Some((rule.affix_type, rule.tier, score));
        }
    }

    best.map(|(affix_type, tier, score)| TabletValueMod {
        text: text.to_string(),
        affix_type: match affix_type {
            AffixType::Prefix => "prefix",
            AffixType::Suffix => "suffix",
        }
        .into(),
        tier: match tier {
            ValueTier::S => "S",
            ValueTier::A => "A",
            ValueTier::B => "B",
        }
        .into(),
        score,
    })
}

fn score_tablet(
    tablet_type: &str,
    uses_remaining: Option<u8>,
    prefixes: &[TabletValueMod],
    suffixes: &[TabletValueMod],
) -> (u16, String, Vec<String>) {
    let mut score = prefixes
        .iter()
        .chain(suffixes)
        .map(|item| item.score)
        .sum::<u16>();
    let mut reasons = prefixes
        .iter()
        .chain(suffixes)
        .map(|item| format!("{}-tier {}", item.tier, item.text))
        .collect::<Vec<_>>();

    if !prefixes.is_empty() && !suffixes.is_empty() {
        score += 18;
        reasons.push("Prefix and suffix both have value".into());
    }
    if has_mechanic_pair(tablet_type, suffixes) {
        score += 18;
        reasons.push("Mechanic-specific suffix matches tablet type".into());
    }
    if uses_remaining == Some(10) {
        score += 8;
        reasons.push("10 uses remaining".into());
    } else if uses_remaining.is_some_and(|uses| uses < 10) {
        score = score.saturating_sub(18);
        reasons.push("Below 10 uses remaining".into());
    }

    let tier = if score >= 100 {
        "S"
    } else if score >= 70 {
        "A"
    } else if score >= 40 {
        "B"
    } else if score >= 20 {
        "C"
    } else {
        "Low"
    };

    if reasons.is_empty() {
        reasons.push("No high-value tablet rolls matched the local rules".into());
    }

    (score, tier.into(), reasons)
}

fn has_mechanic_pair(tablet_type: &str, suffixes: &[TabletValueMod]) -> bool {
    let tablet = tablet_type.to_ascii_lowercase();
    suffixes.iter().any(|modifier| {
        let text = modifier.text.to_ascii_lowercase();
        (tablet.contains("abyss") && text.contains("abyss"))
            || (tablet.contains("breach")
                && (text.contains("breach")
                    || text.contains("wombgift")
                    || text.contains("hiveblood")))
            || (tablet.contains("ritual")
                && (text.contains("ritual") || text.contains("favour") || text.contains("tribute")))
            || (tablet.contains("delirium") && text.contains("delirium"))
            || (tablet.contains("expedition") && text.contains("expedition"))
            || (tablet.contains("overseer") && text.contains("boss"))
    })
}

fn high_roll_bonus(text: &str, threshold: Option<u16>) -> u16 {
    let Some(threshold) = threshold else {
        return 0;
    };
    if numbers(text).into_iter().any(|value| value >= threshold) {
        8
    } else {
        0
    }
}

fn first_number(text: &str) -> Option<u16> {
    numbers(text).into_iter().next()
}

fn numbers(text: &str) -> Vec<u16> {
    let mut values = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_digit() {
            current.push(character);
        } else if !current.is_empty() {
            if let Ok(value) = current.parse() {
                values.push(value);
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        if let Ok(value) = current.parse() {
            values.push(value);
        }
    }
    values
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CurrencyKind {
    Transmutation,
    Augmentation,
    Regal,
    Exalted,
    Alchemy,
}

impl CurrencyKind {
    fn label(self) -> &'static str {
        match self {
            CurrencyKind::Transmutation => "transmutation",
            CurrencyKind::Augmentation => "augmentation",
            CurrencyKind::Regal => "regal",
            CurrencyKind::Exalted => "exalted",
            CurrencyKind::Alchemy => "alchemy",
        }
    }
}

fn validate_craft_settings(rule: &TabletScannerRule) -> Result<(), String> {
    for (label, point) in [
        ("Transmutation", rule.craft.transmutation),
        ("Augmentation", rule.craft.augmentation),
        ("Regal", rule.craft.regal),
        ("Exalted", rule.craft.exalted),
        ("Alchemy", rule.craft.alchemy),
    ] {
        if point.x <= 0 && point.y <= 0 {
            return Err(format!("{label} orb location has not been picked"));
        }
    }
    if rule.craft.craft_delay_ms > 5_000 {
        return Err("Craft wait is too high".into());
    }
    Ok(())
}

fn currency_point(rule: &TabletScannerRule, currency: CurrencyKind) -> ScreenPoint {
    match currency {
        CurrencyKind::Transmutation => rule.craft.transmutation,
        CurrencyKind::Augmentation => rule.craft.augmentation,
        CurrencyKind::Regal => rule.craft.regal,
        CurrencyKind::Exalted => rule.craft.exalted,
        CurrencyKind::Alchemy => rule.craft.alchemy,
    }
}

fn craft_currency_for_slots(
    rule: &TabletScannerRule,
    slot_ids: &[String],
    currency: CurrencyKind,
    reason: &str,
    actions: &mut Vec<TabletCraftAction>,
) -> Result<(), String> {
    if slot_ids.is_empty() {
        return Ok(());
    }

    let slots = scanner_slots(rule)?;
    let targets = slot_ids
        .iter()
        .map(|slot_id| {
            let (column, row) = parse_slot_id(slot_id)?;
            slots
                .iter()
                .find(|slot| slot.column == column && slot.row == row)
                .copied()
                .ok_or_else(|| format!("Slot is outside the tablet scanner grid: {slot_id}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    foreground::focus_executable(&rule.target_executable)?;
    thread::sleep(Duration::from_millis(rule.scan_delay_ms.clamp(20, 1_000)));
    switch_to_currency_tab(rule)?;
    let orb = currency_point(rule, currency);
    right_click_orb(rule, orb)?;
    switch_to_tablet_tab(rule)?;

    shift_left_click_slots(rule, &targets, currency, reason, actions)?;

    Ok(())
}

fn shift_left_click_slots(
    rule: &TabletScannerRule,
    targets: &[ScannerSlot],
    currency: CurrencyKind,
    reason: &str,
    actions: &mut Vec<TabletCraftAction>,
) -> Result<(), String> {
    if targets.is_empty() {
        return Ok(());
    }

    input::key_down("SHIFT")?;
    let mut result = Ok(());

    for slot in targets {
        if let Err(error) = input::left_click_at(slot.x, slot.y, craft_click_timing(rule)) {
            result = Err(error);
            break;
        }
        actions.push(TabletCraftAction {
            slot: slot_id(slot.column, slot.row),
            currency: currency.label().into(),
            reason: reason.into(),
        });
    }

    match (result, input::key_up("SHIFT")) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(release_error)) => Err(format!(
            "Unable to release Shift after crafting: {release_error}"
        )),
        (Err(click_error), Ok(())) => Err(click_error),
        (Err(click_error), Err(release_error)) => Err(format!(
            "{click_error}; also unable to release Shift after crafting: {release_error}"
        )),
    }
}

fn switch_to_currency_tab(rule: &TabletScannerRule) -> Result<(), String> {
    ctrl_wheel(120, rule.craft.tab_switch_delay_ms)
}

fn switch_to_tablet_tab(rule: &TabletScannerRule) -> Result<(), String> {
    ctrl_wheel(-120, rule.craft.tab_switch_delay_ms)
}

fn ctrl_wheel(delta: i32, delay_ms: u64) -> Result<(), String> {
    let delay = Duration::from_millis(delay_ms.clamp(20, 1_000));
    input::key_down("CTRL")?;
    let result = input::mouse_wheel(delta).and_then(|_| input::key_up("CTRL"));
    if result.is_err() {
        let _ = input::key_up("CTRL");
    }
    thread::sleep(delay);
    result
}

fn right_click_orb(rule: &TabletScannerRule, point: ScreenPoint) -> Result<(), String> {
    let timing = craft_click_timing(rule);
    input::right_click_at(point.x, point.y, timing)?;
    thread::sleep(craft_delay(rule));
    Ok(())
}

fn craft_click_timing(rule: &TabletScannerRule) -> input::ClickTiming {
    let delay_ms = rule.craft.craft_delay_ms.clamp(20, 2_000);
    input::ClickTiming {
        cursor_settle_ms: delay_ms,
        click_hold_ms: 40,
        click_release_settle_ms: delay_ms,
    }
}

fn craft_delay(rule: &TabletScannerRule) -> Duration {
    Duration::from_millis(rule.craft.craft_delay_ms.clamp(20, 2_000))
}

fn transmutation_slots(report: &TabletScanReport) -> Vec<String> {
    report
        .tablets
        .iter()
        .filter(|tablet| tablet.rarity.eq_ignore_ascii_case("normal"))
        .map(|tablet| tablet.slot.clone())
        .collect()
}

fn augmentation_slots(report: &TabletScanReport) -> Vec<String> {
    report
        .tablets
        .iter()
        .filter(|tablet| {
            tablet.rarity.eq_ignore_ascii_case("normal")
                || (tablet.rarity.eq_ignore_ascii_case("magic") && modifier_count(tablet) <= 1)
        })
        .map(|tablet| tablet.slot.clone())
        .collect()
}

fn alchemy_slots(report: &TabletScanReport) -> Vec<String> {
    report
        .tablets
        .iter()
        .filter(|tablet| {
            tablet.rarity.eq_ignore_ascii_case("magic") && !has_at_least_a_tier(tablet)
        })
        .map(|tablet| tablet.slot.clone())
        .collect()
}

fn regal_slots(report: &TabletScanReport) -> Vec<String> {
    report
        .tablets
        .iter()
        .filter(|tablet| tablet.rarity.eq_ignore_ascii_case("magic") && has_at_least_a_tier(tablet))
        .map(|tablet| tablet.slot.clone())
        .collect()
}

fn rare_exalted_slots(report: &TabletScanReport) -> Vec<String> {
    report
        .tablets
        .iter()
        .filter(|tablet| tablet.rarity.eq_ignore_ascii_case("rare") && modifier_count(tablet) == 3)
        .map(|tablet| tablet.slot.clone())
        .collect()
}

fn modifier_count(tablet: &TabletScanItem) -> usize {
    tablet.prefixes.len() + tablet.suffixes.len() + tablet.unknown_mods.len()
}

fn has_at_least_a_tier(tablet: &TabletScanItem) -> bool {
    tablet
        .prefixes
        .iter()
        .chain(&tablet.suffixes)
        .any(|modifier| matches!(modifier.tier.as_str(), "S" | "A"))
}

fn parse_affix_type(value: &str) -> Option<AffixType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "prefix" => Some(AffixType::Prefix),
        "suffix" => Some(AffixType::Suffix),
        _ => None,
    }
}

fn parse_value_tier(value: &str) -> Option<ValueTier> {
    match value.trim().to_ascii_uppercase().as_str() {
        "S" => Some(ValueTier::S),
        "A" => Some(ValueTier::A),
        "B" => Some(ValueTier::B),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct ValueRule {
    tablets: &'static [&'static str],
    contains: &'static [&'static str],
    affix_type: AffixType,
    tier: ValueTier,
    score: u16,
    high_roll_at: Option<u16>,
}

impl ValueRule {
    fn applies_to_tablet(&self, tablet_type: &str) -> bool {
        self.tablets.is_empty()
            || self
                .tablets
                .iter()
                .any(|tablet| tablet_type.contains(tablet))
    }

    fn matches(&self, text: &str) -> bool {
        self.contains.iter().all(|part| text.contains(part))
    }
}

fn value_rules() -> &'static [ValueRule] {
    &[
        ValueRule {
            tablets: &[],
            contains: &["increased effect of explicit modifiers"],
            affix_type: AffixType::Prefix,
            tier: ValueTier::S,
            score: 54,
            high_roll_at: Some(18),
        },
        ValueRule {
            tablets: &[],
            contains: &["increased effect of tablet modifiers"],
            affix_type: AffixType::Prefix,
            tier: ValueTier::S,
            score: 54,
            high_roll_at: Some(18),
        },
        ValueRule {
            tablets: &[],
            contains: &["increased effectiveness"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 38,
            high_roll_at: Some(10),
        },
        ValueRule {
            tablets: &[],
            contains: &["increased number of rare monsters"],
            affix_type: AffixType::Prefix,
            tier: ValueTier::A,
            score: 42,
            high_roll_at: Some(34),
        },
        ValueRule {
            tablets: &[],
            contains: &["increased pack size"],
            affix_type: AffixType::Prefix,
            tier: ValueTier::B,
            score: 28,
            high_roll_at: Some(7),
        },
        ValueRule {
            tablets: &[],
            contains: &["increased monster rarity"],
            affix_type: AffixType::Prefix,
            tier: ValueTier::B,
            score: 28,
            high_roll_at: Some(19),
        },
        ValueRule {
            tablets: &[],
            contains: &["rarity of items found"],
            affix_type: AffixType::Prefix,
            tier: ValueTier::B,
            score: 26,
            high_roll_at: Some(11),
        },
        ValueRule {
            tablets: &[],
            contains: &["quantity of waystones found"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 26,
            high_roll_at: Some(38),
        },
        ValueRule {
            tablets: &[],
            contains: &["additional random modifier"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::S,
            score: 92,
            high_roll_at: Some(1),
        },
        ValueRule {
            tablets: &[],
            contains: &["surpassing chance", "additional modifier"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 40,
            high_roll_at: Some(70),
        },
        ValueRule {
            tablets: &[],
            contains: &["unique monsters", "additional rare modifiers"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 30,
            high_roll_at: None,
        },
        ValueRule {
            tablets: &["abyss"],
            contains: &["additional rare monsters", "abysses"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::S,
            score: 68,
            high_roll_at: Some(2),
        },
        ValueRule {
            tablets: &["abyss"],
            contains: &["abyssal monsters", "abyssal modifiers"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 44,
            high_roll_at: Some(28),
        },
        ValueRule {
            tablets: &["abyss"],
            contains: &["abysses", "spawn", "increased monsters"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 38,
            high_roll_at: Some(28),
        },
        ValueRule {
            tablets: &["abyss"],
            contains: &["four additional abyss"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 28,
            high_roll_at: Some(35),
        },
        ValueRule {
            tablets: &["abyss"],
            contains: &["abyss pits", "twice as likely"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 28,
            high_roll_at: None,
        },
        ValueRule {
            tablets: &["breach"],
            contains: &["breaches", "additional rare monsters"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 44,
            high_roll_at: Some(3),
        },
        ValueRule {
            tablets: &["breach"],
            contains: &["unstable breaches", "additional rare monsters"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::S,
            score: 70,
            high_roll_at: Some(3),
        },
        ValueRule {
            tablets: &["breach"],
            contains: &["rare breach monsters", "effectiveness"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 38,
            high_roll_at: Some(18),
        },
        ValueRule {
            tablets: &["breach"],
            contains: &["wombgifts", "level higher"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 30,
            high_roll_at: Some(25),
        },
        ValueRule {
            tablets: &["breach"],
            contains: &["quantity of wombgifts"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 28,
            high_roll_at: Some(50),
        },
        ValueRule {
            tablets: &["breach"],
            contains: &["quantity of hiveblood"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 28,
            high_roll_at: Some(50),
        },
        ValueRule {
            tablets: &["ritual"],
            contains: &["allow rerolling favours", "additional times"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::S,
            score: 70,
            high_roll_at: Some(3),
        },
        ValueRule {
            tablets: &["ritual"],
            contains: &["favours", "chance", "omens"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::S,
            score: 66,
            high_roll_at: Some(60),
        },
        ValueRule {
            tablets: &["ritual"],
            contains: &["monsters sacrificed", "tribute"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 40,
            high_roll_at: Some(26),
        },
        ValueRule {
            tablets: &["ritual"],
            contains: &["rerolling favours", "reduced tribute"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 30,
            high_roll_at: Some(28),
        },
        ValueRule {
            tablets: &["ritual"],
            contains: &["chance to cost no tribute"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 30,
            high_roll_at: Some(5),
        },
        ValueRule {
            tablets: &["delirium"],
            contains: &["delirium monsters", "pack size"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 38,
            high_roll_at: Some(28),
        },
        ValueRule {
            tablets: &["delirium"],
            contains: &["fracturing mirrors"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::A,
            score: 38,
            high_roll_at: Some(28),
        },
        ValueRule {
            tablets: &["delirium"],
            contains: &["mirror timer"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 30,
            high_roll_at: Some(5),
        },
        ValueRule {
            tablets: &["delirium"],
            contains: &["deliriousness"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 30,
            high_roll_at: Some(28),
        },
        ValueRule {
            tablets: &["expedition"],
            contains: &["expedition remnants"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 30,
            high_roll_at: Some(17),
        },
        ValueRule {
            tablets: &["overseer"],
            contains: &["quantity of items dropped by map bosses"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 28,
            high_roll_at: Some(18),
        },
        ValueRule {
            tablets: &["overseer"],
            contains: &["rarity of items dropped by map bosses"],
            affix_type: AffixType::Suffix,
            tier: ValueTier::B,
            score: 26,
            high_roll_at: Some(55),
        },
    ]
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
        Err("Clipboard scanning is only supported on Windows".into())
    }

    pub fn read_clipboard_text() -> Result<String, String> {
        Err("Clipboard scanning is only supported on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_scores_high_value_abyss_tablet() {
        let text = r#"Item Class: Tablets
Rarity: Magic
Empowering Abyss Precursor Tablet of Champions
Abyss Precursor Tablet
--------
10 Uses Remaining
--------
Map has 35% increased number of Rare Monsters
2 additional Rare Monsters are spawned from Abysses in Map
"#;

        let item = parse_tablet_text(text, 2, 3).unwrap();

        assert_eq!(item.slot, "2:3");
        assert_eq!(item.tablet_type, "Abyss Precursor Tablet");
        assert_eq!(item.uses_remaining, Some(10));
        assert_eq!(item.value_tier, "S");
        assert_eq!(item.prefixes.len(), 1);
        assert_eq!(item.suffixes.len(), 1);
    }

    #[test]
    fn parses_ritual_omen_roll_as_high_value_suffix() {
        let text = r#"Item Class: Tablets
Rarity: Magic
Ritual Precursor Tablet
--------
10 Uses Remaining
--------
Ritual Favours in Map have 65% increased chance to be Omens
"#;

        let item = parse_tablet_text(text, 0, 0).unwrap();

        assert_eq!(item.value_tier, "S");
        assert!(item.reasons.iter().any(|reason| reason.contains("Omens")));
    }

    #[test]
    fn treats_additional_random_modifier_as_valuable() {
        let text = r#"Item Class: Tablets
Rarity: Magic
Precursor Tablet
--------
10 Uses Remaining
--------
Map has 1 additional random Modifier
"#;

        let item = parse_tablet_text(text, 0, 0).unwrap();

        assert_eq!(item.value_tier, "S");
        assert!(item
            .suffixes
            .iter()
            .any(|modifier| modifier.text.contains("additional random Modifier")));
    }

    #[test]
    fn custom_value_rule_can_classify_user_tier_text() {
        let text = r#"Item Class: Tablets
Rarity: Magic
Breach Precursor Tablet
--------
10 Uses Remaining
--------
Strongboxes contain extra Artifacts
"#;
        let rules = vec![TabletValueRuleConfig {
            id: "custom-artifacts".into(),
            label: "Artifacts".into(),
            tablet_match: "breach".into(),
            text_match: "extra artifacts".into(),
            affix_type: "suffix".into(),
            tier: "A".into(),
            score: 45,
            high_roll_at: None,
        }];

        let item = parse_tablet_text_with_rules(text, 1, 1, &rules).unwrap();

        assert!(item.suffixes.iter().any(|modifier| {
            modifier.tier == "A" && modifier.text == "Strongboxes contain extra Artifacts"
        }));
        assert!(has_at_least_a_tier(&item));
    }

    #[test]
    fn craft_plan_starts_normals_with_transmutation_and_augmentation() {
        let report = TabletScanReport {
            scanned_slots: 2,
            tablets: vec![
                test_tablet("0:0", "Normal", vec![], vec![], vec![]),
                test_tablet(
                    "1:0",
                    "Magic",
                    vec![test_mod("A", "prefix")],
                    vec![],
                    vec![],
                ),
            ],
            skipped_slots: vec![],
        };

        assert_eq!(transmutation_slots(&report), vec!["0:0"]);
        assert_eq!(augmentation_slots(&report), vec!["0:0", "1:0"]);
    }

    #[test]
    fn craft_plan_sends_bad_magic_to_alchemy_after_augmentation_rescan() {
        let report = TabletScanReport {
            scanned_slots: 2,
            tablets: vec![
                test_tablet("0:0", "Magic", vec![], vec![], vec!["bad mod"]),
                test_tablet(
                    "1:0",
                    "Magic",
                    vec![test_mod("B", "prefix")],
                    vec![test_mod("B", "suffix")],
                    vec![],
                ),
            ],
            skipped_slots: vec![],
        };

        assert_eq!(alchemy_slots(&report), vec!["0:0", "1:0"]);
        assert!(regal_slots(&report).is_empty());
    }

    #[test]
    fn craft_plan_sends_a_tier_magic_to_regal_and_exalted() {
        let report = TabletScanReport {
            scanned_slots: 2,
            tablets: vec![
                test_tablet(
                    "0:0",
                    "Magic",
                    vec![test_mod("A", "prefix")],
                    vec![],
                    vec![],
                ),
                test_tablet(
                    "1:0",
                    "Magic",
                    vec![],
                    vec![test_mod("S", "suffix")],
                    vec![],
                ),
            ],
            skipped_slots: vec![],
        };

        assert!(alchemy_slots(&report).is_empty());
        assert_eq!(regal_slots(&report), vec!["0:0", "1:0"]);
    }

    #[test]
    fn craft_plan_exalts_rare_tablets_with_three_modifiers() {
        let report = TabletScanReport {
            scanned_slots: 3,
            tablets: vec![
                test_tablet(
                    "0:0",
                    "Rare",
                    vec![test_mod("A", "prefix"), test_mod("B", "prefix")],
                    vec![test_mod("B", "suffix")],
                    vec![],
                ),
                test_tablet(
                    "1:0",
                    "Rare",
                    vec![test_mod("A", "prefix"), test_mod("B", "prefix")],
                    vec![test_mod("B", "suffix"), test_mod("B", "suffix")],
                    vec![],
                ),
                test_tablet(
                    "2:0",
                    "Magic",
                    vec![test_mod("A", "prefix")],
                    vec![test_mod("B", "suffix")],
                    vec![],
                ),
            ],
            skipped_slots: vec![],
        };

        assert_eq!(rare_exalted_slots(&report), vec!["0:0"]);
    }

    #[test]
    fn rare_tablet_counts_post_uses_modifier_lines_without_known_keywords() {
        let text = r#"Item Class: Tablets
Rarity: Rare
Warding Etched Vessel
Precursor Tablet
--------
10 Uses Remaining
--------
Areas contain 2 additional Strongboxes
Strongboxes are Corrupted
Areas have 15% increased chance to contain Essences
Can be used in a personal Map Device
"#;

        let item = parse_tablet_text(text, 4, 5).unwrap();

        assert_eq!(item.rarity, "Rare");
        assert_eq!(modifier_count(&item), 3);
        assert_eq!(
            rare_exalted_slots(&TabletScanReport {
                scanned_slots: 1,
                tablets: vec![item],
                skipped_slots: vec![],
            }),
            vec!["4:5"]
        );
    }

    #[test]
    fn black_slot_center_counts_as_empty() {
        assert!(is_empty_slot_color("#000000"));
        assert!(is_empty_slot_color("#070606"));
        assert!(!is_empty_slot_color("#151923"));
    }

    #[test]
    fn ignores_non_tablet_clipboard_text() {
        assert!(parse_tablet_text("Rarity: Normal\nIron Greaves", 0, 0).is_none());
    }

    fn test_tablet(
        slot: &str,
        rarity: &str,
        prefixes: Vec<TabletValueMod>,
        suffixes: Vec<TabletValueMod>,
        unknown_mods: Vec<&str>,
    ) -> TabletScanItem {
        let (column, row) = parse_slot_id(slot).unwrap();
        TabletScanItem {
            slot: slot.into(),
            column,
            row,
            name: None,
            tablet_type: "Precursor Tablet".into(),
            rarity: rarity.into(),
            uses_remaining: Some(10),
            value_tier: "Low".into(),
            value_score: 0,
            prefixes,
            suffixes,
            unknown_mods: unknown_mods.into_iter().map(String::from).collect(),
            reasons: vec![],
            raw_text: String::new(),
        }
    }

    fn test_mod(tier: &str, affix_type: &str) -> TabletValueMod {
        TabletValueMod {
            text: format!("{tier} test roll"),
            affix_type: affix_type.into(),
            tier: tier.into(),
            score: 40,
        }
    }
}
