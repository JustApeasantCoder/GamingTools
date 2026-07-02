use serde::Serialize;
use std::{thread, time::Duration};

use crate::{foreground, input, profiles::TabletScannerRule};

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
        match parse_tablet_text(&text, slot.column, slot.row) {
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

pub fn parse_tablet_text(text: &str, column: u8, row: u8) -> Option<TabletScanItem> {
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
    for line in likely_modifier_lines(&lines) {
        if let Some(classified) = classify_modifier(line, &tablet_type) {
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

fn likely_modifier_lines(lines: &[String]) -> impl Iterator<Item = &str> {
    lines.iter().filter_map(|line| {
        let lowered = line.to_ascii_lowercase();
        if lowered.starts_with("item class:")
            || lowered.starts_with("rarity:")
            || lowered.contains("uses remaining")
            || lowered.contains("place into")
            || lowered.contains("requires")
            || lowered.contains("unidentified")
            || !modifier_words(&lowered)
        {
            None
        } else {
            Some(line.as_str())
        }
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

fn classify_modifier(text: &str, tablet_type: &str) -> Option<TabletValueMod> {
    let normalized = text.to_ascii_lowercase();
    let tablet = tablet_type.to_ascii_lowercase();
    let mut best: Option<(AffixType, ValueTier, u16)> = None;

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
    use std::slice;
    use windows_sys::Win32::System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
            OpenClipboard,
        },
        Memory::{GlobalLock, GlobalUnlock},
    };

    const CF_UNICODETEXT: u32 = 13;

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
            let opened = unsafe { OpenClipboard(std::ptr::null_mut()) };
            if opened == 0 {
                Err(format!(
                    "Unable to open clipboard: {}",
                    std::io::Error::last_os_error()
                ))
            } else {
                Ok(Self)
            }
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
    fn ignores_non_tablet_clipboard_text() {
        assert!(parse_tablet_text("Rarity: Normal\nIron Greaves", 0, 0).is_none());
    }
}
