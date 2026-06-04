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
    input,
    macros::{random_delay_ms, validate_rules},
    profiles::{MacroStep, PixelCondition, PixelRule, Profile},
    screen,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeEvent {
    kind: String,
    message: String,
}

#[derive(Default)]
pub struct RuntimeState {
    inner: Mutex<Option<RuntimeHandle>>,
}

struct RuntimeHandle {
    stop: Arc<AtomicBool>,
    thread: thread::JoinHandle<()>,
    sound_enabled: bool,
}

impl RuntimeState {
    pub fn start(&self, app: AppHandle, profile: Profile) -> Result<(), String> {
        self.stop_worker()?;

        let validation = validate_rules(&profile.macro_rules);
        if !validation.valid {
            return Err(validation.errors.join("; "));
        }

        let profile_name = profile.name.clone();
        let sound_enabled = profile.runtime_settings.sound_enabled;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread_app = app.clone();
        let thread = thread::spawn(move || runtime_loop(thread_app, profile, thread_stop));
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Runtime lock poisoned".to_string())?;
        *guard = Some(RuntimeHandle {
            stop,
            thread,
            sound_enabled,
        });
        drop(guard);

        emit_event(&app, "runtime", format!("Runtime started: {profile_name}"));
        play_toggle_sound(true, sound_enabled);
        Ok(())
    }

    pub fn stop(&self, app: &AppHandle) -> Result<(), String> {
        if let Some(sound_enabled) = self.stop_worker()? {
            emit_event(app, "runtime", "Runtime stopped");
            play_toggle_sound(false, sound_enabled);
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.inner
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    fn stop_worker(&self) -> Result<Option<bool>, String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "Runtime lock poisoned".to_string())?;
        if let Some(handle) = guard.take() {
            handle.stop.store(true, Ordering::Relaxed);
            let sound_enabled = handle.sound_enabled;
            let _ = handle.thread.join();
            return Ok(Some(sound_enabled));
        }
        Ok(None)
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
                    let _ = state.stop(&app);
                } else {
                    let _ = state.start(app.clone(), active_profile.clone());
                }
            }

            thread::sleep(Duration::from_millis(20));
        }
    });
}

fn runtime_loop(app: AppHandle, profile: Profile, stop: Arc<AtomicBool>) {
    let input_thread = {
        let app = app.clone();
        let profile = profile.clone();
        let stop = Arc::clone(&stop);
        thread::spawn(move || input_detection_loop(app, profile, stop))
    };
    let pixel_thread = {
        let stop = Arc::clone(&stop);
        thread::spawn(move || pixel_detection_loop(app, profile, stop))
    };

    let _ = input_thread.join();
    let _ = pixel_thread.join();
}

fn input_detection_loop(app: AppHandle, profile: Profile, stop: Arc<AtomicBool>) {
    let mut worker_threads = Vec::new();
    let mut macro_workers = HashMap::new();

    for rule in profile.macro_rules.iter().filter(|rule| rule.enabled) {
        let (sender, receiver) = sync_channel(1);
        macro_workers.insert(rule.id.clone(), sender);
        worker_threads.push(spawn_action_worker(
            app.clone(),
            Arc::clone(&stop),
            rule.name.clone(),
            rule.steps.clone(),
            receiver,
        ));
    }

    let mut pressed_triggers = HashSet::new();
    let mut toggle_waiting_for_release: HashSet<String> = HashSet::new();
    let mut toggle_held_rules: HashSet<String> = HashSet::new();
    let poll_interval = Duration::from_millis(8);
    let mut next_poll = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        for rule in profile.macro_rules.iter().filter(|rule| rule.enabled) {
            let is_down = input::is_key_down(&rule.trigger_key);
            let was_down = pressed_triggers.contains(&rule.id);

            if is_down && !was_down {
                pressed_triggers.insert(rule.id.clone());
                emit_event(
                    &app,
                    "macro",
                    format!("Macro detected: {} ({})", rule.name, rule.trigger_key),
                );
                submit_action(&macro_workers, &rule.id);
            } else if !is_down && was_down {
                pressed_triggers.remove(&rule.id);
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
                    let _ = input::key_up(&rule.hold_key);
                    toggle_held_rules.remove(&rule.id);
                    emit_event(
                        &app,
                        "toggleHold",
                        format!("{} released {}", rule.name, rule.hold_key),
                    );
                } else {
                    let _ = input::key_down(&rule.hold_key);
                    toggle_held_rules.insert(rule.id.clone());
                    emit_event(
                        &app,
                        "toggleHold",
                        format!("{} holding {}", rule.name, rule.hold_key),
                    );
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
        let _ = input::key_up(&rule.hold_key);
    }
}

fn pixel_detection_loop(app: AppHandle, profile: Profile, stop: Arc<AtomicBool>) {
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
            rule.name.clone(),
            pixel_steps(rule),
            receiver,
        ));
    }

    let mut matched_pixel_rules: HashSet<String> = HashSet::new();
    let mut held_pixel_rules: HashSet<String> = HashSet::new();
    let poll_interval = Duration::from_millis(8);
    let mut next_poll = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        for rule in profile.pixel_rules.iter().filter(|rule| rule.enabled) {
            let matched = pixel_rule_matches(rule);
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
                    hold_pixel_rule(&app, rule);
                    held_pixel_rules.insert(rule.id.clone());
                } else if !matched && held_pixel_rules.contains(&rule.id) {
                    release_pixel_rule(&app, rule);
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
        release_pixel_rule(&app, rule);
    }
}

fn pixel_rule_matches(rule: &PixelRule) -> bool {
    let primary = condition_matches(&PixelCondition {
        target_color: rule.target_color.clone(),
        tolerance: rule.tolerance,
        adjacent_pixels: rule.adjacent_pixels,
        sample_point: rule.sample_point,
        invert_detection: rule.invert_detection,
    });
    let secondary_group = if rule.secondary_condition_enabled {
        let secondary1 = condition_matches(&rule.secondary_condition);
        let uses_or = rule.secondary_condition_operator.eq_ignore_ascii_case("or");
        let secondary2 = rule.secondary_condition2_enabled
            && !((uses_or && secondary1) || (!uses_or && !secondary1))
            && condition_matches(&rule.secondary_condition2);
        combine_secondary_conditions(
            secondary1,
            rule.secondary_condition2_enabled,
            secondary2,
            &rule.secondary_condition_operator,
        )
    } else {
        true
    };
    combine_conditions(primary, rule.secondary_condition_enabled, secondary_group)
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

fn condition_matches(condition: &PixelCondition) -> bool {
    let raw_match = screen::sample_rule_points(condition.sample_point, condition.adjacent_pixels)
        .into_iter()
        .filter_map(|point| screen::sample_pixel(point).ok())
        .any(|sample| {
            screen::color_matches(&sample.color, &condition.target_color, condition.tolerance)
        });
    if condition.invert_detection {
        !raw_match
    } else {
        raw_match
    }
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
    rule_name: String,
    steps: Vec<MacroStep>,
    receiver: Receiver<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            if receiver.recv_timeout(Duration::from_millis(25)).is_err() {
                continue;
            }
            execute_action_chain(&app, &rule_name, &steps, &stop);
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
) {
    for step in steps {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let press_ms = random_delay_ms(&step.press_duration);
        if interruptible_tap(&step.key, press_ms, stop).is_ok() {
            emit_event(
                app,
                "action",
                format!("{rule_name} pressed {} for {} ms", step.key, press_ms),
            );
        }
        if !interruptible_sleep(random_delay_ms(&step.humanized_delay), stop) {
            break;
        }
    }
}

fn interruptible_tap(key: &str, press_ms: u64, stop: &Arc<AtomicBool>) -> Result<(), String> {
    input::key_down(key)?;
    let completed = interruptible_sleep(press_ms, stop);
    let release_result = input::key_up(key);
    if completed {
        release_result
    } else {
        Err("Runtime stopped".into())
    }
}

fn interruptible_sleep(duration_ms: u64, stop: &Arc<AtomicBool>) -> bool {
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        thread::sleep((deadline - now).min(Duration::from_millis(5)));
    }
    false
}

fn hold_pixel_rule(app: &AppHandle, rule: &PixelRule) {
    for step in pixel_steps(rule) {
        let _ = input::key_down(&step.key);
        emit_event(app, "action", format!("{} holding {}", rule.name, step.key));
    }
}

fn release_pixel_rule(app: &AppHandle, rule: &PixelRule) {
    for step in pixel_steps(rule) {
        let _ = input::key_up(&step.key);
        emit_event(
            app,
            "action",
            format!("{} released {}", rule.name, step.key),
        );
    }
}

fn emit_event(app: &AppHandle, kind: &str, message: impl Into<String>) {
    let _ = app.emit(
        "runtime-event",
        RuntimeEvent {
            kind: kind.into(),
            message: message.into(),
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

    use super::{combine_conditions, combine_secondary_conditions, submit_action};

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
}
