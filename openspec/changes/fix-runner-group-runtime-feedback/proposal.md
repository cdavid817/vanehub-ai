## Why

Runner discovery currently collapses a failed optional probe into a global “Runners unavailable” state, even when the local Runner can execute, so both single-Agent and multi-Agent sessions show a false failure before any work starts. Multi-Agent execution also exposes neither a live elapsed duration nor member-level progress, leaving users unable to distinguish active work from a stalled generation.

## What Changes

- Separate local Runner readiness from optional/remote Runner discovery failures and preserve every independently usable Runner.
- Reserve global discovery failure for catalog-critical errors; omit an unresolvable optional Runner without claiming that Local is unavailable.
- Make active multi-Agent elapsed time advance from the canonical run start and freeze at terminal completion.
- Persist managed CLI completion, failure, and cancellation into the canonical Run before restart recovery can classify the execution.
- Project the currently active member, lifecycle phase, first-output/activity milestones, and incremental member output through the existing Agent service stream.
- Render member-attributed streaming output and compact activity indicators while preserving the shared conversation thread and cancellation behavior.
- Keep Web/mock behavior deterministic and contract-compatible with the desktop adapter.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-provider-runtime`: Runner discovery failures become per-runner, fail-soft availability results rather than a global false-unavailable state.
- `agent-run-state-management`: Active run elapsed time and member activity milestones remain observable and monotonic.
- `multi-agent-group-chat`: Each executing member exposes attributed incremental output and an explicit active/waiting/terminal progress state.
- `chat-experience`: Multi-seat stream events remain visible and attributed while a member is producing a response.

## Impact

- Desktop runtime Runner catalog/probe orchestration, safe error projection, and unified diagnostics.
- Managed CLI terminal handling and canonical Run restart recovery evidence.
- Session generation and multi-seat event projection in Rust.
- Shared Agent service contracts plus matching Tauri and Web/mock adapters.
- Multi-Agent conversation header, roster/member status, elapsed display, and streamed message rendering.
- Unit, component, contract, and Playwright tests; no new dependency and no database migration is expected unless persisted run timestamps are currently absent.
