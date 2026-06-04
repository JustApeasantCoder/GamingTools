# Agent Notes

## Architecture

Keep the app feature-owned. New user-facing capabilities should usually add a folder under `src/features/<feature-name>` and a matching Rust capability module under `src-tauri/src/<capability>` when native behavior is required.

Avoid turning `src/App.tsx` into a feature implementation file. It should stay as shell composition, active profile state, runtime state, and cross-feature routing.

## Frontend Rules

- Use React + TypeScript with small feature components.
- Put reusable visual primitives in `src/shared/ui`.
- Put frontend-facing DTOs in `src/shared/types`.
- Call native commands through `src/shared/api/client.ts`; keep direct `invoke` usage out of feature components.
- Preserve the dark, compact desktop utility design unless a new approved design replaces it.
- Keep future features as first-class tabs or feature sections rather than piling controls into Macro Builder.
- Use `KeyCaptureButton` for key or mouse-button capture instead of manual key text inputs.

## Rust Rules

- `commands` should stay thin and delegate to capability modules.
- `profiles` owns serialized models and app-config JSON persistence.
- `macros` owns validation, timing, and macro execution helpers.
- `input` owns foreground OS input APIs.
- `screen` owns pixel sampling and color matching.
- `runtime` owns threads, start/stop, cancellation, and held-key cleanup.
- Runtime detection loops must not execute delayed action chains directly. Keep detection responsive and submit tap-style actions to bounded per-rule workers.
- Runtime activity that matters to the user should emit a structured `runtime-event`; avoid logging unchanged pixel samples every poll cycle.
- Toggle-hold behavior must always release held actions during runtime shutdown.
- Add tests beside the module that owns the behavior.

## Safety Boundary

Do not add process injection, game memory reading, hidden-window targeting, anti-cheat evasion, or bypass language. The runtime is foreground-only and should stay based on normal operating system APIs unless the product direction is explicitly changed.

## Future Feature Pattern

For a new feature:

1. Add `src/features/<feature-name>`.
2. Add shared types only if another feature or backend command needs them.
3. Add a Rust module only for native or persisted behavior.
4. Expose the smallest needed command in `src-tauri/src/commands`.
5. Add focused tests for validation, persistence, and native-safe boundary behavior.
