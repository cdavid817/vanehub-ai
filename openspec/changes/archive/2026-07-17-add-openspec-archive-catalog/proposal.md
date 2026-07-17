## Why

The current archive README lists directory names but does not let maintainers locate historical work by capability, artifact, or change identifier without reopening many archived Markdown files. A generated catalog will make routine archive lookup a single structured-data read rather than a repeated full-text search across the archive.

## What Changes

- Generate a deterministic machine-readable archive catalog alongside the human-readable archive index.
- Index each online archive by its path, date, change name, available planning artifacts, and affected capability names derived from its delta spec paths.
- Update the archive index generator and contributor guidance so archive queries use the catalog first.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `openspec-archive-governance`: Require a generated structured catalog for efficient archive discovery.

## Impact

- Affects `scripts/Update-OpenSpecArchiveIndex.ps1`, generated files under `openspec/changes/archive/`, and archive governance documentation only.
- Does not affect desktop runtime, Web runtime, frontend service boundaries, Tauri commands, or external APIs.
