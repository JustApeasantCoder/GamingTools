# Gaming Toolkit

Gaming Toolkit is a Windows-first desktop utility for foreground-only mouse and keyboard automation. It is built with Tauri v2, Rust, React, TypeScript, and Vite.

The v0.1.0 MVP includes:

- Macro Builder for trigger keys and ordered key chains such as `A -> B -> C`.
- Multiple customizable macro rules per profile.
- Listen-based key/mouse capture for macro triggers and action steps.
- Humanized press duration and delay timing with per-step min/max millisecond ranges.
- Pixel Trigger rules that sample a screen pixel or adjacent pixels, support inverted matching and an optional second-pixel AND condition, then run or hold a configurable output action chain.
- Toggle Hold rules that press once to hold a key or mouse button down, then press again to release it.
- A foreground app guard that pauses or stops automation and releases held actions when the target executable loses focus.
- Profile JSON import/export for backups and moving profiles between machines.
- Native macro recording that captures supported keyboard and mouse-button actions with measured hold and delay timing.
- A profile-level global Start/Stop hotkey, defaulting to `F4`, with optional on/off sound cues.
- An Execution Log that reports runtime state, detected triggers, and emitted actions.
- Responsive Pixel Trigger polling that runs independently from delayed macro and output action chains.
- Local profile persistence in the app config directory.
- A conservative runtime that uses normal Windows foreground input APIs only.

The Start/Stop hotkey is monitored globally and works while the app is minimized or unfocused. Simulated macro and hold actions still use normal Windows input APIs.

## Development

Install dependencies:

```powershell
npm install
```

Run the web UI only:

```powershell
npm run dev
```

Run the Tauri desktop app:

```powershell
npm run tauri:dev
```

Build the frontend:

```powershell
npm run build
```

Run Rust tests:

```powershell
cd src-tauri
cargo test
```

Build the desktop app:

```powershell
npm run tauri:build
```

## Project Layout

- `src/features/macros` owns macro chain UI and macro-specific frontend behavior.
- `src/features/pixel-trigger` owns pixel detection UI and pixel rule editing.
- `src/features/toggle-hold` owns toggle-hold rule editing.
- `src/features/profiles` owns profile selection and profile list UI.
- `src/features/app-guard` owns foreground application guard settings.
- `src/features/profile-transfer` owns profile import/export UI.
- `src/features/macro-recorder` owns native recorder controls.
- `src/shared` owns frontend types, backend command bridge, and reusable UI primitives.
- `src-tauri/src/profiles` owns profile models and JSON persistence.
- `src-tauri/src/macros` owns validation and humanized delay logic.
- `src-tauri/src/input` owns foreground Windows input simulation.
- `src-tauri/src/screen` owns pixel sampling and color matching.
- `src-tauri/src/runtime` owns start/stop state and automation loops.
- `src-tauri/src/foreground` owns foreground executable detection.
- `src-tauri/src/recorder` owns native macro input recording.
- `src-tauri/src/commands` exposes the Tauri command interface.

## Safety Boundary

This app intentionally avoids game-specific bypass behavior. It does not read game memory, inject into processes, target hidden windows, or evade anti-cheat systems. Runtime input is foreground-only and uses normal OS-level APIs.

Some games or anti-cheat systems may still dislike automation tools. Use profiles carefully and keep the app scoped to local accessibility-style automation.
