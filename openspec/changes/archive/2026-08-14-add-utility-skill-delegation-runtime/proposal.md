## Why

Utility Skills are already modeled and discoverable, but VaneHub deliberately marks them unavailable because no authoritative delegation runtime exists. This prevents bounded specialist work and leaves the evolution evidence pipeline without canonical Utility revision and terminal facts.

## What Changes

- Add a native Utility Skill delegation runtime with explicit start, terminal, cancellation, timeout, and limit states.
- Resolve the exact effective Utility Skill revision for the active canonical workspace before execution and preserve that immutable observation for the whole attempt.
- Expose a fixed-schema `delegate_utility_skill` tool to supported native API Agents; ordinary provider tool calls and CLI output are not reclassified as Utility execution.
- Enforce bounded instructions, task input, tool calls, approvals, duration, nested-delegation depth, and result summaries at the runtime boundary.
- Publish safe structured lifecycle projections to execution observability, unified logging, and the existing Skill evolution evidence sink without persisting raw prompts or outputs in evidence storage.
- Make supported Utility Skills available in effective Skill inventory and update Settings/Web behavior to describe native-only delegation honestly.

## Capabilities

### New Capabilities

- `utility-skill-delegation-runtime`: Authoritative resolution, execution lifecycle, limits, cancellation, terminal facts, and safe observability for Utility Skill delegation.

### Modified Capabilities

- `effective-skill-runtime`: Discover supported Utility Skills and expose bounded delegation separately from Role Skill loading.
- `skill-management`: Report Utility availability from runtime support instead of treating every Utility Skill as unsupported.
- `settings-skill-management-ui`: Present delegatable Utility Skills and native/Web capability differences without offering Role Skill actions.

## Impact

- Desktop runtime: adds native application/domain ports and adapter wiring in `src-tauri`, plus lifecycle projection into the existing evidence and observability boundaries.
- Web runtime: retains deterministic contract parity but reports execution as unavailable because browser mock mode has no trusted native executor.
- Frontend: shared service types and Skills detail state gain delegation capability metadata; React remains isolated from Tauri commands.
- Persistence and privacy: existing run/evidence stores receive bounded metadata only; raw Utility task input, instructions, tool arguments, and output stay outside evidence tables and unified logs.
- No breaking API changes are intended; the new delegation operation is additive.
