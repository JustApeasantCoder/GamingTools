use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use crate::{
    input,
    profiles::{HumanizationSettings, MacroStep},
};

#[derive(Default)]
pub struct RecorderState {
    inner: Mutex<Option<RecorderHandle>>,
}

struct RecorderHandle {
    stop: Arc<AtomicBool>,
    thread: thread::JoinHandle<Vec<RecordedPress>>,
}

struct RecordedPress {
    key: String,
    pressed_at: Instant,
    released_at: Instant,
}

impl RecorderState {
    pub fn start(&self) -> Result<(), String> {
        let mut guard = self.inner.lock().map_err(|_| "Recorder lock poisoned")?;
        if guard.is_some() {
            return Err("Macro recording is already active".into());
        }
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || record_loop(thread_stop));
        *guard = Some(RecorderHandle { stop, thread });
        Ok(())
    }

    pub fn stop(&self) -> Result<Vec<MacroStep>, String> {
        let handle = self
            .inner
            .lock()
            .map_err(|_| "Recorder lock poisoned")?
            .take()
            .ok_or_else(|| "Macro recording is not active".to_string())?;
        handle.stop.store(true, Ordering::Relaxed);
        let mut presses = handle
            .thread
            .join()
            .map_err(|_| "Macro recorder thread failed".to_string())?;
        presses.sort_by_key(|press| press.pressed_at);
        trim_stop_click(&mut presses);
        Ok(to_steps(presses))
    }
}

fn record_loop(stop: Arc<AtomicBool>) -> Vec<RecordedPress> {
    let keys = input::supported_key_names();
    let mut down: HashMap<String, Instant> = HashMap::new();
    let mut presses = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        for key in &keys {
            let is_down = input::is_key_down(key);
            if is_down && !down.contains_key(key) {
                down.insert(key.clone(), Instant::now());
            } else if !is_down {
                if let Some(pressed_at) = down.remove(key) {
                    presses.push(RecordedPress {
                        key: key.clone(),
                        pressed_at,
                        released_at: Instant::now(),
                    });
                }
            }
        }
        thread::sleep(Duration::from_millis(4));
    }
    presses
}

fn trim_stop_click(presses: &mut Vec<RecordedPress>) {
    if presses.last().is_some_and(|press| {
        press.key == "LEFT CLICK" && press.released_at.elapsed() < Duration::from_secs(1)
    }) {
        presses.pop();
    }
}

fn to_steps(presses: Vec<RecordedPress>) -> Vec<MacroStep> {
    presses
        .iter()
        .enumerate()
        .map(|(index, press)| {
            let hold_ms = press
                .released_at
                .duration_since(press.pressed_at)
                .as_millis()
                .clamp(1, 10_000) as u64;
            let delay_ms = presses
                .get(index + 1)
                .map(|next| {
                    next.pressed_at
                        .saturating_duration_since(press.released_at)
                        .as_millis() as u64
                })
                .unwrap_or(0)
                .min(60_000);
            MacroStep {
                id: uuid::Uuid::new_v4().to_string(),
                key: press.key.clone(),
                press_duration: fixed_timing(hold_ms),
                humanized_delay: HumanizationSettings {
                    enabled: delay_ms > 0,
                    min_ms: delay_ms,
                    max_ms: delay_ms,
                },
            }
        })
        .collect()
}

fn fixed_timing(ms: u64) -> HumanizationSettings {
    HumanizationSettings {
        enabled: true,
        min_ms: ms,
        max_ms: ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_presses_become_fixed_timing_steps() {
        let start = Instant::now();
        let steps = to_steps(vec![
            RecordedPress {
                key: "A".into(),
                pressed_at: start,
                released_at: start + Duration::from_millis(40),
            },
            RecordedPress {
                key: "B".into(),
                pressed_at: start + Duration::from_millis(65),
                released_at: start + Duration::from_millis(100),
            },
        ]);

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].press_duration.min_ms, 40);
        assert_eq!(steps[0].humanized_delay.min_ms, 25);
        assert_eq!(steps[1].humanized_delay.min_ms, 0);
    }
}
