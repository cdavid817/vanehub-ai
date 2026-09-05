## Why

The System Activity maintenance surface reduced projection health to only the last completion time and ran rebuilds without visible progress or a cancellation path. Operators need safe, localized diagnostics and responsive recovery controls so they can identify domain-specific projection faults and stop a long-running rebuild without losing the last valid projection.

## What Changes

- Present projector lease state, per-domain cursor and backlog data, gaps, failures, and recent rebuild history in the read-only health panel.
- Show deterministic rebuild phases and processed-item progress while preventing duplicate rebuild starts.
- Allow an in-progress rebuild to be cancelled through the existing service boundary and confirm that the previous projection remains available.
- Add localized copy and focused component tests for health diagnostics, progress, and cancellation.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `skill-evolution-system-activity`: Clarify the operator-visible health diagnostics and cancellable, progress-reporting rebuild behavior.

## Impact

- Affects the shared React UI used by both desktop and Web/mock runtimes.
- Uses existing system-activity service contracts; React components continue to avoid direct Tauri calls, and no runtime-adapter or backend API changes are required.
- Touches System Activity presentation, rebuild orchestration state, localized strings, shared health types, and component tests.
- Adds no dependencies and does not alter authoritative Skill Evolution records, Agent execution, or projection generation safety.
