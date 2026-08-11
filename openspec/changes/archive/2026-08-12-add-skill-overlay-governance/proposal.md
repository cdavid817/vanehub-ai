## Why

Immutable and layered Skill packages need a safe customization path before automated evolution can modify them. VaneHub AI currently lacks a reversible, auditable way to apply user guidance, exact instruction patches, and supporting resource overrides without editing the authoritative package.

## What Changes

- Add versioned Skill Overlay documents containing exact-match patches, learned-guidance blocks, non-executable supporting files, trust metadata, and conflict records.
- Resolve overlays by management context and replay active trusted mutations over the effective Skill package without modifying its base files.
- Add compare-and-swap witnesses, base-content hashes, drift detection, deterministic replay, and explicit reconciliation when a base package changes.
- Add prompt-injection and unsafe-content scanning, executable-file rejection, bounded files and packages, canonical path enforcement, and pinned-Skill refusal.
- Store append-only, redacted overlay history with diff, apply, disable, revert, trust, conflict, and reconciliation events.
- Provide preview, diff, patch, learn, file, trust-promotion, history, disable, revert, and reconcile operations through the existing frontend service boundary with matching Tauri and Web/mock adapters.
- Extend the Skills settings experience with an Overlay tab, effective/base comparison, trust and conflict states, learned-guidance review, supporting-file management, history, and rollback controls.
- Count successful patch and Overlay mutations in the sidecar usage model established by the effective Skill runtime.
- Keep Overlay writes manual and user-initiated in this change; candidate generation, Curator approval, and automatic evolution remain deferred.

## Capabilities

### New Capabilities

- `skill-overlay-governance`: Defines Overlay data, scoped resolution, safe replay, trust, mutation limits, history, conflicts, reconciliation, rollback, and pinned-Skill behavior.

### Modified Capabilities

- `skill-management`: Adds layer-aware Overlay management operations and returns base, effective, trust, conflict, pinned, and Overlay summary state for Skills.
- `agent-skill-injection`: Requires all Skill instruction consumers to use successfully replayed effective content and to fall back safely when an Overlay is invalid or conflicted.
- `settings-skill-management-ui`: Adds per-Skill Overlay inspection, diff, learned-guidance, supporting-file, trust, history, conflict-reconciliation, and rollback interactions.

## Impact

- Depends on the effective catalog, immutable package, resource reader, and usage sidecars introduced by `establish-effective-skill-runtime`; implementation must sequence that change first.
- Affects the Rust Skill domain, application service and ports, filesystem transactions, effective content loading, resource resolution, usage tracking, unified logging, Tauri commands, and recovery behavior.
- Affects shared TypeScript Skill contracts, `agent-service.ts`, the Tauri and Web/mock adapters, Settings Skills components, and localization resources in both desktop and Web runtimes.
- Adds local Overlay and history storage but no remote service, arbitrary script execution, new state-management library, or direct Tauri invocation from React components.
