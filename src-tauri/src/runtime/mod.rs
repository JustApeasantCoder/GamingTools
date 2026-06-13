use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    foreground, input, inventory,
    macros::{random_delay_ms, validate_profile},
    profiles::{
        InventorySlotSnapshot, InventoryStashRule, MacroStep, PixelCondition, PixelRule, Profile,
        ToggleHoldRule,
    },
    screen,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeEvent {
    kind: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot_colors: Option<Vec<InventorySlotSnapshot>>,
}

#[derive(Default)]
pub struct RuntimeState {
    inner: Mutex<Option<RuntimeHandle>>,
}

struct RuntimeHandle {
    stop: Arc<AtomicBool>,
    thread: thread::JoinHandle<()>,
    profile_id: String,
    sound_enabled: bool,
}

impl RuntimeState {
    pub fn start(&self, app: AppHandle, profile: Profile) -> Result<(), String> {
        let validation = validate_profile(&profile);
        if !validation.valid {
            return Err(validation.errors.join("; "));
        }

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Runtime lock poisoned".to_string())?;
        stop_handle(guard.take());

        let profile_name = profile.name.clone();
        let sound_enabled = profile.runtime_settings.sound_enabled;
        *guard = Some(spawn_runtime(app.clone(), profile));
        emit_event(
            &app,
            "runtime",
            format!("Automation started: {profile_name}"),
        );
        play_toggle_sound(true, sound_enabled);
        drop(guard);
        Ok(())
    }

    pub fn refresh_profile(&self, app: AppHandle, profile: Profile) -> Result<bool, String> {
        let validation = validate_profile(&profile);
        if !validation.valid {
            return Err(validation.errors.join("; "));
        }

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Runtime lock poisoned".to_string())?;
        let Some(handle) = guard.as_ref() else {
            return Ok(false);
        };
        if handle.profile_id != profile.id || handle.thread.is_finished() {
            return Ok(false);
        }

        let profile_name = profile.name.clone();
        stop_handle(guard.take());
        *guard = Some(spawn_runtime(app.clone(), profile));
        emit_event(
            &app,
            "runtime",
            format!("Automation updated: {profile_name}"),
        );
        Ok(true)
    }

    pub fn refresh_running(&self, app: AppHandle, profile: Profile) -> Result<bool, String> {
        let validation = validate_profile(&profile);
        if !validation.valid {
            return Err(validation.errors.join("; "));
        }

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Runtime lock poisoned".to_string())?;
        let Some(handle) = guard.as_ref() else {
            return Ok(false);
        };
        if handle.thread.is_finished() {
            return Ok(false);
        }

        let profile_name = profile.name.clone();
        stop_handle(guard.take());
        *guard = Some(spawn_runtime(app.clone(), profile));
        emit_event(
            &app,
            "runtime",
            format!("Automation updated: {profile_name}"),
        );
        Ok(true)
    }

    pub fn stop(&self, app: &AppHandle) -> Result<(), String> {
        if let Some(sound_enabled) = self.stop_worker()? {
            emit_event(app, "runtime", "Automation stopped");
            play_toggle_sound(false, sound_enabled);
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.inner
            .lock()
            .map(|guard| {
                guard
                    .as_ref()
                    .is_some_and(|handle| !handle.thread.is_finished())
            })
            .unwrap_or(false)
    }

    fn stop_worker(&self) -> Result<Option<bool>, String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Runtime lock poisoned".to_string())?;
        Ok(stop_handle(guard.take()))
    }
}

fn spawn_runtime(app: AppHandle, profile: Profile) -> RuntimeHandle {
    let profile_id = profile.id.clone();
    let sound_enabled = profile.runtime_settings.sound_enabled;
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop);
    let thread = thread::spawn(move || runtime_loop(app, profile, thread_stop));
    RuntimeHandle {
        stop,
        thread,
        profile_id,
        sound_enabled,
    }
}

impl Drop for RuntimeState {
    fn drop(&mut self) {
        if let Ok(handle) = self.inner.get_mut() {
            stop_handle(handle.take());
        }
    }
}

fn stop_handle(handle: Option<RuntimeHandle>) -> Option<bool> {
    let handle = handle?;
    handle.stop.store(true, Ordering::Relaxed);
    let sound_enabled = handle.sound_enabled;
    let _ = handle.thread.join();
    Some(sound_enabled)
}

#[derive(Clone, Default)]
struct InputOwners {
    counts: Arc<Mutex<HashMap<String, usize>>>,
}

impl InputOwners {
    fn acquire(&self, key: &str) -> Result<(), String> {
        let normalized = key.trim().to_uppercase();
        let mut counts = self
            .counts
            .lock()
            .map_err(|_| "Input ownership lock poisoned".to_string())?;
        if let Some(count) = counts.get_mut(&normalized) {
            *count += 1;
            return Ok(());
        }
        input::key_down(&normalized)?;
        counts.insert(normalized, 1);
        Ok(())
    }

    fn release(&self, key: &str) -> Result<(), String> {
        let normalized = key.trim().to_uppercase();
        let mut counts = self
            .counts
            .lock()
            .map_err(|_| "Input ownership lock poisoned".to_string())?;
        let Some(count) = counts.get_mut(&normalized) else {
            return Ok(());
        };
        if *count > 1 {
            *count -= 1;
            return Ok(());
        }
        input::key_up(&normalized)?;
        counts.remove(&normalized);
        Ok(())
    }

    fn release_all(&self) {
        if let Ok(mut counts) = self.counts.lock() {
            for key in counts.keys() {
                let _ = input::key_up(key);
            }
            counts.clear();
        }
    }
}

pub fn start_hotkey_monitor(app: AppHandle) {
    thread::spawn(move || {
        let mut waiting_for_release = false;
        let mut profile: Option<Profile> = None;
        let mut last_refresh = Instant::now() - Duration::from_secs(1);
        loop {
            if last_refresh.elapsed() >= Duration::from_millis(500) {
                profile = crate::profiles::load_store(&app).ok().and_then(|store| {
                    store
                        .profiles
                        .into_iter()
                        .find(|item| item.id == store.active_profile_id)
                });
                last_refresh = Instant::now();
            }
            let Some(active_profile) = profile.as_ref() else {
                thread::sleep(Duration::from_millis(100));
                continue;
            };

            let is_down = input::is_key_down(&active_profile.runtime_settings.toggle_hotkey);
            if is_down && !waiting_for_release {
                waiting_for_release = true;
            } else if !is_down && waiting_for_release {
                waiting_for_release = false;
                let state = app.state::<RuntimeState>();
                if state.is_running() {
                    if let Err(error) = state.stop(&app) {
                        emit_event(&app, "error", error);
                    }
                } else if let Err(error) = state.start(app.clone(), active_profile.clone()) {
                    emit_event(&app, "error", error);
                }
            }

            thread::sleep(Duration::from_millis(20));
        }
    });
}

fn runtime_loop(app: AppHandle, profile: Profile, stop: Arc<AtomicBool>) {
    let inputs = InputOwners::default();
    let guard_active = Arc::new(AtomicBool::new(true));
    let guard_thread = {
        let app = app.clone();
        let profile = profile.clone();
        let stop = Arc::clone(&stop);
        let guard_active = Arc::clone(&guard_active);
        let inputs = inputs.clone();
        thread::spawn(move || foreground_guard_loop(app, profile, stop, guard_active, inputs))
    };
    let input_thread = {
        let app = app.clone();
        let profile = profile.clone();
        let stop = Arc::clone(&stop);
        let inputs = inputs.clone();
        let guard_active = Arc::clone(&guard_active);
        thread::spawn(move || input_detection_loop(app, profile, stop, guard_active, inputs))
    };
    let pixel_thread = {
        let stop = Arc::clone(&stop);
        let inputs = inputs.clone();
        let guard_active = Arc::clone(&guard_active);
        thread::spawn(move || pixel_detection_loop(app, profile, stop, guard_active, inputs))
    };

    let _ = input_thread.join();
    let _ = pixel_thread.join();
    let _ = guard_thread.join();
    inputs.release_all();
}

fn foreground_guard_loop(
    app: AppHandle,
    profile: Profile,
    stop: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    inputs: InputOwners,
) {
    let guard = &profile.runtime_settings.foreground_guard;
    if !guard.enabled {
        return;
    }
    let mut was_active = true;
    while !stop.load(Ordering::Relaxed) {
        let is_active = foreground::matches_executable(&guard.executable);
        active.store(is_active, Ordering::Relaxed);
        if is_active != was_active {
            if is_active {
                emit_event(
                    &app,
                    "foregroundGuard",
                    format!("Resumed: {} is foreground", guard.executable),
                );
            } else {
                inputs.release_all();
                emit_event(
                    &app,
                    "foregroundGuard",
                    format!("Target app lost focus: {}", guard.executable),
                );
                if guard.on_focus_lost == "stop" {
                    stop.store(true, Ordering::Relaxed);
                    emit_event(
                        &app,
                        "runtime",
                        "Automation stopped by foreground app guard",
                    );
                }
            }
            was_active = is_active;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn input_detection_loop(
    app: AppHandle,
    profile: Profile,
    stop: Arc<AtomicBool>,
    guard_active: Arc<AtomicBool>,
    inputs: InputOwners,
) {
    let mut worker_threads = Vec::new();
    let mut macro_workers = HashMap::new();
    let mut inventory_workers = HashMap::new();
    let mut inventory_snapshot_workers = HashMap::new();

    for rule in profile.macro_rules.iter().filter(|rule| rule.enabled) {
        let (sender, receiver) = sync_channel(1);
        macro_workers.insert(rule.id.clone(), sender);
        worker_threads.push(spawn_action_worker(
            app.clone(),
            Arc::clone(&stop),
            Arc::clone(&guard_active),
            rule.name.clone(),
            rule.steps.clone(),
            receiver,
            inputs.clone(),
        ));
    }
    for rule in profile
        .inventory_stash_rules
        .iter()
        .filter(|rule| rule.enabled)
    {
        let (sender, receiver) = sync_channel(1);
        inventory_workers.insert(rule.id.clone(), sender);
        let (snapshot_sender, snapshot_receiver) = sync_channel(1);
        inventory_snapshot_workers.insert(rule.id.clone(), snapshot_sender);
        worker_threads.push(spawn_inventory_worker(
            app.clone(),
            Arc::clone(&stop),
            Arc::clone(&guard_active),
            rule.clone(),
            receiver,
            snapshot_receiver,
        ));
    }

    let mut pressed_triggers = HashSet::new();
    let mut toggle_waiting_for_release: HashSet<String> = HashSet::new();
    let mut toggle_held_rules: HashSet<String> = HashSet::new();
    let supported_inputs = auto_release_input_names(&profile);
    let mut pressed_inputs = currently_pressed_inputs(&supported_inputs);
    let poll_interval = Duration::from_millis(8);
    let mut next_poll = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        if !guard_active.load(Ordering::Relaxed) {
            for rule in profile
                .toggle_hold_rules
                .iter()
                .filter(|rule| toggle_held_rules.contains(&rule.id))
            {
                let _ = inputs.release(&rule.hold_key);
            }
            toggle_held_rules.clear();
            pressed_triggers.clear();
            toggle_waiting_for_release.clear();
            pressed_inputs = currently_pressed_inputs(&supported_inputs);
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        let current_pressed_inputs = currently_pressed_inputs(&supported_inputs);
        let newly_pressed_inputs = current_pressed_inputs
            .difference(&pressed_inputs)
            .cloned()
            .collect::<HashSet<_>>();
        pressed_inputs = current_pressed_inputs;

        let auto_release_rule_ids = profile
            .toggle_hold_rules
            .iter()
            .filter(|rule| rule.enabled && toggle_held_rules.contains(&rule.id))
            .filter(|rule| should_auto_release_toggle(rule, &newly_pressed_inputs))
            .map(|rule| rule.id.clone())
            .collect::<Vec<_>>();
        for rule_id in auto_release_rule_ids {
            if let Some(rule) = profile
                .toggle_hold_rules
                .iter()
                .find(|rule| rule.id == rule_id)
            {
                if inputs.release(&rule.hold_key).is_ok() {
                    toggle_held_rules.remove(&rule.id);
                    emit_event(
                        &app,
                        "toggleHold",
                        format!("{} auto-released {}", rule.name, rule.hold_key),
                    );
                }
            }
        }
        for rule in profile.macro_rules.iter().filter(|rule| rule.enabled) {
            let is_down = input::is_key_down(&rule.trigger_key);
            let was_down = pressed_triggers.contains(&rule.id);

            if is_down && !was_down {
                pressed_triggers.insert(rule.id.clone());
                emit_event(
                    &app,
                    "macro",
                    format!(
                        "Macro shortcut pressed: {} ({})",
                        rule.name, rule.trigger_key
                    ),
                );
                submit_action(&macro_workers, &rule.id);
            } else if !is_down && was_down {
                pressed_triggers.remove(&rule.id);
            }
        }

        for rule in profile
            .inventory_stash_rules
            .iter()
            .filter(|rule| rule.enabled)
        {
            let is_down = input::is_key_down(&rule.trigger_key);
            let trigger_id = format!("inventory:{}", rule.id);
            let was_down = pressed_triggers.contains(&trigger_id);

            if is_down && !was_down {
                pressed_triggers.insert(trigger_id.clone());
                emit_event(
                    &app,
                    "inventoryStash",
                    format!(
                        "Inventory stash shortcut pressed: {} ({})",
                        rule.name, rule.trigger_key
                    ),
                );
                submit_action(&inventory_workers, &rule.id);
            } else if !is_down && was_down {
                pressed_triggers.remove(&trigger_id);
            }

            let baseline_is_down = input::is_key_down(&rule.capture_baseline_key);
            let baseline_trigger_id = format!("inventory-baseline:{}", rule.id);
            let baseline_was_down = pressed_triggers.contains(&baseline_trigger_id);

            if baseline_is_down && !baseline_was_down {
                pressed_triggers.insert(baseline_trigger_id.clone());
                emit_event(
                    &app,
                    "inventoryStash",
                    format!(
                        "Capture baseline shortcut pressed: {} ({})",
                        rule.name, rule.capture_baseline_key
                    ),
                );
                submit_action(&inventory_snapshot_workers, &rule.id);
            } else if !baseline_is_down && baseline_was_down {
                pressed_triggers.remove(&baseline_trigger_id);
            }
        }

        for rule in profile.toggle_hold_rules.iter().filter(|rule| rule.enabled) {
            let trigger_is_down = input::is_key_down(&rule.trigger_key);
            let waiting_for_release = toggle_waiting_for_release.contains(&rule.id);

            if trigger_is_down && !waiting_for_release {
                toggle_waiting_for_release.insert(rule.id.clone());
            } else if !trigger_is_down && waiting_for_release {
                toggle_waiting_for_release.remove(&rule.id);
                if toggle_held_rules.contains(&rule.id) {
                    if inputs.release(&rule.hold_key).is_ok() {
                        toggle_held_rules.remove(&rule.id);
                        emit_event(
                            &app,
                            "toggleHold",
                            format!("{} released {}", rule.name, rule.hold_key),
                        );
                    }
                } else {
                    match inputs.acquire(&rule.hold_key) {
                        Ok(()) => {
                            toggle_held_rules.insert(rule.id.clone());
                            emit_event(
                                &app,
                                "toggleHold",
                                format!("{} holding {}", rule.name, rule.hold_key),
                            );
                        }
                        Err(error) => emit_event(&app, "error", error),
                    }
                }
            }
        }

        next_poll += poll_interval;
        if let Some(remaining) = next_poll.checked_duration_since(Instant::now()) {
            thread::sleep(remaining);
        } else {
            next_poll = Instant::now();
        }
    }

    drop(macro_workers);
    for worker in worker_threads {
        let _ = worker.join();
    }

    for rule in profile
        .toggle_hold_rules
        .iter()
        .filter(|rule| rule.enabled && toggle_held_rules.contains(&rule.id))
    {
        let _ = inputs.release(&rule.hold_key);
    }
}

fn currently_pressed_inputs(supported_inputs: &[String]) -> HashSet<String> {
    supported_inputs
        .iter()
        .filter(|key| input::is_key_down(key))
        .map(|key| key.trim().to_uppercase())
        .collect()
}

fn auto_release_input_names(profile: &Profile) -> Vec<String> {
    if profile
        .toggle_hold_rules
        .iter()
        .any(|rule| rule.enabled && rule.release_mode == "anyOther")
    {
        return input::supported_key_names();
    }

    profile
        .toggle_hold_rules
        .iter()
        .filter(|rule| rule.enabled && rule.release_mode == "specific")
        .map(|rule| rule.release_key.trim().to_uppercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn should_auto_release_toggle(
    rule: &ToggleHoldRule,
    newly_pressed_inputs: &HashSet<String>,
) -> bool {
    match rule.release_mode.as_str() {
        "specific" => newly_pressed_inputs.contains(&rule.release_key.trim().to_uppercase()),
        "anyOther" => {
            let trigger_key = rule.trigger_key.trim().to_uppercase();
            let hold_key = rule.hold_key.trim().to_uppercase();
            newly_pressed_inputs
                .iter()
                .any(|key| key != &trigger_key && key != &hold_key)
        }
        _ => false,
    }
}

fn pixel_detection_loop(
    app: AppHandle,
    profile: Profile,
    stop: Arc<AtomicBool>,
    guard_active: Arc<AtomicBool>,
    inputs: InputOwners,
) {
    let mut worker_threads = Vec::new();
    let mut pixel_workers = HashMap::new();
    for rule in profile
        .pixel_rules
        .iter()
        .filter(|rule| rule.enabled && rule.trigger_mode != "hold")
    {
        let (sender, receiver) = sync_channel(1);
        pixel_workers.insert(rule.id.clone(), sender);
        worker_threads.push(spawn_action_worker(
            app.clone(),
            Arc::clone(&stop),
            Arc::clone(&guard_active),
            rule.name.clone(),
            pixel_steps(rule),
            receiver,
            inputs.clone(),
        ));
    }

    let mut matched_pixel_rules: HashSet<String> = HashSet::new();
    let mut held_pixel_rules: HashSet<String> = HashSet::new();
    let mut failed_pixel_rules: HashSet<String> = HashSet::new();
    let poll_interval = Duration::from_millis(8);
    let mut next_poll = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        if !guard_active.load(Ordering::Relaxed) {
            for rule in profile
                .pixel_rules
                .iter()
                .filter(|rule| held_pixel_rules.contains(&rule.id))
            {
                release_pixel_rule(&app, rule, &inputs);
            }
            held_pixel_rules.clear();
            matched_pixel_rules.clear();
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        for rule in profile.pixel_rules.iter().filter(|rule| rule.enabled) {
            let matched = match pixel_rule_matches(rule) {
                Ok(matched) => {
                    if failed_pixel_rules.remove(&rule.id) {
                        emit_event(
                            &app,
                            "pixel",
                            format!("Pixel Trigger {} sampling recovered", rule.name),
                        );
                    }
                    matched
                }
                Err(error) => {
                    if failed_pixel_rules.insert(rule.id.clone()) {
                        emit_event(
                            &app,
                            "error",
                            format!("Pixel Trigger {}: {error}", rule.name),
                        );
                    }
                    false
                }
            };
            let was_matched = matched_pixel_rules.contains(&rule.id);
            if matched {
                matched_pixel_rules.insert(rule.id.clone());
            } else {
                matched_pixel_rules.remove(&rule.id);
            }

            if matched != was_matched {
                let state = if matched {
                    "conditions met"
                } else {
                    "conditions cleared"
                };
                emit_event(
                    &app,
                    "pixel",
                    format!("Pixel Trigger {}: {state}", rule.name),
                );
            }

            if rule.trigger_mode == "hold" {
                if matched && !held_pixel_rules.contains(&rule.id) {
                    if hold_pixel_rule(&app, rule, &inputs) {
                        held_pixel_rules.insert(rule.id.clone());
                    }
                } else if !matched && held_pixel_rules.contains(&rule.id) {
                    release_pixel_rule(&app, rule, &inputs);
                    held_pixel_rules.remove(&rule.id);
                }
            } else if matched && (!was_matched || rule.continue_while_detected) {
                submit_action(&pixel_workers, &rule.id);
            }
        }

        next_poll += poll_interval;
        if let Some(remaining) = next_poll.checked_duration_since(Instant::now()) {
            thread::sleep(remaining);
        } else {
            next_poll = Instant::now();
        }
    }

    drop(pixel_workers);
    for worker in worker_threads {
        let _ = worker.join();
    }

    for rule in profile
        .pixel_rules
        .iter()
        .filter(|rule| rule.enabled && held_pixel_rules.contains(&rule.id))
    {
        release_pixel_rule(&app, rule, &inputs);
    }
}

pub fn test_pixel_rule(rule: &PixelRule) -> Result<bool, String> {
    pixel_rule_matches(rule)
}

pub fn test_pixel_actions(app: &AppHandle, rule: &PixelRule) -> Result<(), String> {
    let steps = pixel_steps(rule)
        .into_iter()
        .map(|mut step| {
            step.press_duration.min_ms = step.press_duration.min_ms.min(500);
            step.press_duration.max_ms = step.press_duration.max_ms.min(500);
            step.humanized_delay.min_ms = step.humanized_delay.min_ms.min(1_000);
            step.humanized_delay.max_ms = step.humanized_delay.max_ms.min(1_000);
            step
        })
        .collect::<Vec<_>>();
    if steps.is_empty() {
        return Err("This rule has no actions to test".into());
    }
    for step in &steps {
        if !input::supports_key(&step.key) {
            return Err(format!("Unsupported action: {}", step.key));
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    let guard_active = Arc::new(AtomicBool::new(true));
    execute_action_chain(
        app,
        &format!("{} test", rule.name),
        &steps,
        &stop,
        &guard_active,
        &InputOwners::default(),
    );
    Ok(())
}

fn pixel_rule_matches(rule: &PixelRule) -> Result<bool, String> {
    let primary = condition_matches(&PixelCondition {
        target_color: rule.target_color.clone(),
        tolerance: rule.tolerance,
        adjacent_pixels: rule.adjacent_pixels,
        sample_point: rule.sample_point,
        invert_detection: rule.invert_detection,
    });
    let secondary_group = if rule.secondary_condition_enabled {
        let secondary1 = condition_matches(&rule.secondary_condition)?;
        let uses_or = rule.secondary_condition_operator.eq_ignore_ascii_case("or");
        let secondary2 = if !rule.secondary_condition2_enabled
            || (uses_or && secondary1)
            || (!uses_or && !secondary1)
        {
            false
        } else {
            condition_matches(&rule.secondary_condition2)?
        };
        combine_secondary_conditions(
            secondary1,
            rule.secondary_condition2_enabled,
            secondary2,
            &rule.secondary_condition_operator,
        )
    } else {
        true
    };
    Ok(combine_conditions(
        primary?,
        rule.secondary_condition_enabled,
        secondary_group,
    ))
}

fn combine_conditions(primary: bool, secondary_enabled: bool, secondary: bool) -> bool {
    primary && (!secondary_enabled || secondary)
}

fn combine_secondary_conditions(
    first: bool,
    second_enabled: bool,
    second: bool,
    operator: &str,
) -> bool {
    if !second_enabled {
        return first;
    }
    if operator.eq_ignore_ascii_case("or") {
        first || second
    } else {
        first && second
    }
}

fn condition_matches(condition: &PixelCondition) -> Result<bool, String> {
    let samples = screen::sample_rule_points(condition.sample_point, condition.adjacent_pixels)
        .into_iter()
        .map(screen::sample_pixel)
        .collect::<Result<Vec<_>, _>>()?;
    let raw_match = samples.iter().any(|sample| {
        screen::color_matches(&sample.color, &condition.target_color, condition.tolerance)
    });
    Ok(if condition.invert_detection {
        !raw_match
    } else {
        raw_match
    })
}

fn pixel_steps(rule: &PixelRule) -> Vec<MacroStep> {
    if !rule.action_steps.is_empty() {
        return rule.action_steps.clone();
    }

    rule.output_key
        .as_ref()
        .map(|key| MacroStep {
            id: format!("{}-legacy-output", rule.id),
            key: key.clone(),
            press_duration: crate::profiles::HumanizationSettings {
                enabled: true,
                min_ms: 50,
                max_ms: 90,
            },
            humanized_delay: crate::profiles::HumanizationSettings {
                enabled: false,
                min_ms: 0,
                max_ms: 0,
            },
        })
        .into_iter()
        .collect()
}

fn spawn_action_worker(
    app: AppHandle,
    stop: Arc<AtomicBool>,
    guard_active: Arc<AtomicBool>,
    rule_name: String,
    steps: Vec<MacroStep>,
    receiver: Receiver<()>,
    inputs: InputOwners,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if receiver.recv_timeout(Duration::from_millis(25)).is_err() {
                continue;
            }
            if guard_active.load(Ordering::Relaxed) {
                execute_action_chain(&app, &rule_name, &steps, &stop, &guard_active, &inputs);
            }
        }
    })
}

fn spawn_inventory_worker(
    app: AppHandle,
    stop: Arc<AtomicBool>,
    guard_active: Arc<AtomicBool>,
    mut rule: InventoryStashRule,
    stash_receiver: Receiver<()>,
    snapshot_receiver: Receiver<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if snapshot_receiver.try_recv().is_ok() {
                if !guard_active.load(Ordering::Relaxed) {
                    continue;
                }
                match inventory::capture_snapshot(&rule) {
                    Ok(snapshot_colors) => {
                        rule.detection_mode = "snapshot".into();
                        rule.snapshot_colors = snapshot_colors.clone();
                        emit_snapshot_event(
                            &app,
                            &rule.id,
                            snapshot_colors,
                            format!(
                                "{} captured {} baseline slot{}",
                                rule.name,
                                rule.snapshot_colors.len(),
                                if rule.snapshot_colors.len() == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            ),
                        );
                    }
                    Err(error) => emit_event(&app, "error", format!("{}: {error}", rule.name)),
                }
            }

            if stash_receiver
                .recv_timeout(Duration::from_millis(25))
                .is_ok()
            {
                if !guard_active.load(Ordering::Relaxed) {
                    continue;
                }
                match inventory::send_occupied_slots(&rule, &stop, &guard_active) {
                    Ok(sent) => emit_event(
                        &app,
                        "inventoryStash",
                        format!(
                            "{} sent {} slot{} to stash",
                            rule.name,
                            sent,
                            if sent == 1 { "" } else { "s" }
                        ),
                    ),
                    Err(error) => emit_event(&app, "error", format!("{}: {error}", rule.name)),
                }
            }
        }
    })
}

fn submit_action(workers: &HashMap<String, SyncSender<()>>, rule_id: &str) -> bool {
    if let Some(worker) = workers.get(rule_id) {
        return worker.try_send(()).is_ok();
    }
    false
}

fn execute_action_chain(
    app: &AppHandle,
    rule_name: &str,
    steps: &[MacroStep],
    stop: &Arc<AtomicBool>,
    guard_active: &Arc<AtomicBool>,
    inputs: &InputOwners,
) {
    for step in steps {
        if stop.load(Ordering::Relaxed) || !guard_active.load(Ordering::Relaxed) {
            break;
        }
        let press_ms = random_delay_ms(&step.press_duration);
        match interruptible_tap(&step.key, press_ms, stop, guard_active, inputs) {
            Ok(()) => emit_event(
                app,
                "action",
                format!("{rule_name} pressed {} for {} ms", step.key, press_ms),
            ),
            Err(error) if error != "Runtime stopped" => emit_event(app, "error", error),
            Err(_) => {}
        }
        if !interruptible_sleep(random_delay_ms(&step.humanized_delay), stop, guard_active) {
            break;
        }
    }
}

fn interruptible_tap(
    key: &str,
    press_ms: u64,
    stop: &Arc<AtomicBool>,
    guard_active: &Arc<AtomicBool>,
    inputs: &InputOwners,
) -> Result<(), String> {
    inputs.acquire(key)?;
    let completed = interruptible_sleep(press_ms, stop, guard_active);
    let release_result = inputs.release(key);
    if completed {
        release_result
    } else {
        Err("Runtime stopped".into())
    }
}

fn interruptible_sleep(
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
        thread::sleep((deadline - now).min(Duration::from_millis(5)));
    }
    false
}

fn hold_pixel_rule(app: &AppHandle, rule: &PixelRule, inputs: &InputOwners) -> bool {
    let mut acquired = Vec::new();
    for step in pixel_steps(rule) {
        match inputs.acquire(&step.key) {
            Ok(()) => {
                acquired.push(step.key.clone());
                emit_event(app, "action", format!("{} holding {}", rule.name, step.key));
            }
            Err(error) => {
                emit_event(app, "error", error);
                for key in acquired {
                    let _ = inputs.release(&key);
                }
                return false;
            }
        }
    }
    true
}

fn release_pixel_rule(app: &AppHandle, rule: &PixelRule, inputs: &InputOwners) {
    for step in pixel_steps(rule) {
        match inputs.release(&step.key) {
            Ok(()) => emit_event(
                app,
                "action",
                format!("{} released {}", rule.name, step.key),
            ),
            Err(error) => emit_event(app, "error", error),
        }
    }
}

fn emit_event(app: &AppHandle, kind: &str, message: impl Into<String>) {
    let _ = app.emit(
        "runtime-event",
        RuntimeEvent {
            kind: kind.into(),
            message: message.into(),
            rule_id: None,
            snapshot_colors: None,
        },
    );
}

fn emit_snapshot_event(
    app: &AppHandle,
    rule_id: &str,
    snapshot_colors: Vec<InventorySlotSnapshot>,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "runtime-event",
        RuntimeEvent {
            kind: "inventorySnapshot".into(),
            message: message.into(),
            rule_id: Some(rule_id.into()),
            snapshot_colors: Some(snapshot_colors),
        },
    );
}

#[cfg(windows)]
fn play_toggle_sound(is_on: bool, enabled: bool) {
    if !enabled {
        return;
    }
    let frequency = if is_on { 880 } else { 620 };
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::Beep(frequency, 70);
    }
}

#[cfg(not(windows))]
fn play_toggle_sound(_is_on: bool, _enabled: bool) {}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::mpsc::sync_channel};

    use super::{
        combine_conditions, combine_secondary_conditions, should_auto_release_toggle, submit_action,
    };
    use crate::profiles::ToggleHoldRule;

    #[test]
    fn secondary_condition_behaves_as_an_and_gate() {
        assert!(combine_conditions(true, false, false));
        assert!(combine_conditions(true, true, true));
        assert!(!combine_conditions(true, true, false));
        assert!(!combine_conditions(false, true, true));
    }

    #[test]
    fn secondary_condition_group_supports_one_and_or_two_targets() {
        assert!(combine_secondary_conditions(true, false, false, "and"));
        assert!(!combine_secondary_conditions(false, false, true, "or"));
        assert!(combine_secondary_conditions(true, true, true, "and"));
        assert!(!combine_secondary_conditions(true, true, false, "and"));
        assert!(combine_secondary_conditions(true, true, false, "or"));
        assert!(combine_secondary_conditions(false, true, true, "or"));
        assert!(!combine_secondary_conditions(false, true, false, "or"));
    }

    #[test]
    fn action_queue_drops_work_when_one_run_is_already_pending() {
        let (sender, _receiver) = sync_channel(1);
        let workers = HashMap::from([("rule".to_string(), sender)]);

        assert!(submit_action(&workers, "rule"));
        assert!(!submit_action(&workers, "rule"));
    }

    #[test]
    fn toggle_hold_auto_release_supports_specific_and_any_other_inputs() {
        let mut rule = ToggleHoldRule {
            id: "toggle".into(),
            name: "Hold".into(),
            enabled: true,
            trigger_key: "F8".into(),
            hold_key: "RIGHT CLICK".into(),
            release_mode: "specific".into(),
            release_key: "SPACE".into(),
        };

        assert!(should_auto_release_toggle(&rule, &["SPACE".into()].into()));
        assert!(!should_auto_release_toggle(&rule, &["A".into()].into()));

        rule.release_mode = "anyOther".into();
        assert!(should_auto_release_toggle(&rule, &["A".into()].into()));
        assert!(!should_auto_release_toggle(&rule, &["F8".into()].into()));
        assert!(!should_auto_release_toggle(
            &rule,
            &["RIGHT CLICK".into()].into()
        ));
    }
}
