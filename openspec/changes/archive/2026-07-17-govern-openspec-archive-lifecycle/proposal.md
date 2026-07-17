## Why

Completed OpenSpec changes are retained in `openspec/changes/archive/`, but the project documentation names a different path and no repository policy defines archive admission, discoverability, or long-term retention. As the decision history grows, this makes archived work harder to locate and easier to handle inconsistently.

## What Changes

- Establish a documented lifecycle for completed OpenSpec changes, including archive admission checks and immutable historical artifacts.
- Correct the documented archive location to `openspec/changes/archive/`.
- Add a searchable archive index and a documented cold-archive boundary that preserves Git history rather than replacing Markdown artifacts with opaque compressed files.
- Define when `openspec archive` may use `--skip-specs` and prohibit `--no-validate` in the normal workflow.

## Capabilities

### New Capabilities
- `openspec-archive-governance`: Defines how this repository validates, archives, indexes, retains, and cold-archives completed OpenSpec changes.

### Modified Capabilities

- None.

## Impact

- Affects repository documentation under `AGENTS.md` and `openspec/` only; it does not affect the desktop runtime, Web runtime, frontend service boundaries, Tauri commands, or external APIs.
- Adds an archive policy and index maintained alongside archived OpenSpec changes.
- Uses the existing OpenSpec CLI and Git history; no runtime dependency is added.
