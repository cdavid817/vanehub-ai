## Why

The main OpenSpec corpus is growing and requires a repeatable way to find redundant requirements, control specification size, and propose consolidation without losing normative behavior. Manual full-spec review is slow and difficult to audit.

## What Changes

- Add a project-local Codex skill that analyzes `openspec/specs/` and produces an auditable optimization report.
- Detect semantic duplicate candidates, capability clusters, and token or line budget exceptions without editing main specs.
- Produce a requirement and scenario mapping, SHALL/MUST coverage result, and proposed OpenSpec delta specs for human review.
- Require explicit review before any proposal is applied to main specs.

## Capabilities

### New Capabilities
- `spec-optimization`: Defines safe, reviewable optimization analysis for the repository's main OpenSpec corpus.

### Modified Capabilities

- None.

## Impact

- Adds only a project-local Codex skill and OpenSpec documentation; no desktop runtime, Web runtime, frontend service, Tauri command, or external API changes.
