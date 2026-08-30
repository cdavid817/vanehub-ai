## Context

The existing required desktop suite already contains evaluation domain and UI specs, but its shared configuration shadows every managed Agent with fixtures and selects several Agents at once. The external-provider suite preserves the host `PATH`, yet it has no evaluation-specific prerequisite contract or evidence shape. OnePiece credentials are already resolved through a helper for other live desktop flows and must remain outside application data and artifacts.

## Goals / Non-Goals

**Goals:**

- Make OpenCode evaluation reproducible through a focused fixture run first, then optionally qualify the real installed CLI.
- Add an opt-in OnePiece evaluation run whose missing credential is visibly `BLOCKED`.
- Exercise both native evaluation commands and the rendered evaluation center with stable Agent ids.
- Keep provider secrets out of test arguments and retained evidence.

**Non-Goals:**

- Making paid provider calls part of CI or the default desktop gate.
- Replacing the evaluation service boundary or introducing Agent-specific branches in React.
- Treating a fixture Agent response as quality evidence for a real model.
- Modifying host OpenCode authentication or installing provider credentials.

## Decisions

### Use a dedicated desktop evaluation layer

Add a named orchestrator layer and WebdriverIO config rather than overloading the broad smoke suite. This gives the run its own prerequisite result, evidence directory, timeout, and provider provenance. Reusing `VANEHUB_DESKTOP_SPEC` alone was rejected because it selects from the broad required manifest and cannot express live-provider prerequisites truthfully.

### Split hermetic and live qualification modes

The default evaluation layer uses only the repository OpenCode fixture and must pass without network access. Live OpenCode and OnePiece are explicit opt-ins and preserve the host `PATH`. A live run that lacks authentication reports `BLOCKED`; it never falls back to the fixture. This prevents a green fixture run from being mistaken for real-provider coverage.

### Select Agents by stable id inside focused specs

Focused specs select `opencode` or `onepiece` by their registry ids, start one-Agent arenas, and correlate the rendered row with the persisted arena and attempt. Display names remain assertion text only. This avoids localization and ordering dependence.

### Reuse the existing credential boundary

OnePiece uses `VANEHUB_ONEPIECE_API_KEY` or the existing supported profile lookup. The launcher passes the value only in the child environment. Evidence auditing compares outputs against forbidden secret material without printing it. OpenCode authentication is discovered read-only; the harness does not run login commands or mutate user configuration.

### Fix defects at the owning boundary

UI polling or selection defects stay in React/service code, adapter shape defects stay in both runtime adapters, and dispatch/persistence defects stay in Rust. WebdriverIO may use Tauri IPC only to establish expected persisted state; user workflow assertions continue to read the rendered DOM.

## Risks / Trade-offs

- [Live provider behavior and cost are nondeterministic] → Keep live qualification opt-in, use one bounded minimal task, and report the exact terminal outcome.
- [A credential could leak through failure text] → Redact dispatch diagnostics before persistence and audit every retained evidence file.
- [The existing dirty worktree contains unrelated feature work] → Limit edits to evaluation and desktop harness files and avoid bulk rewrites.
- [OpenCode can be installed without authentication] → Treat executable and authentication as separate prerequisites and report `BLOCKED` before building when either is absent.

## Migration Plan

1. Add the focused layer and deterministic OpenCode fixture spec.
2. Run it against an isolated desktop artifact and fix surfaced defects.
3. Add live prerequisite handling and OnePiece credential-safe qualification.
4. Keep existing broad evaluation specs until the focused layer proves equivalent coverage; remove no gate in this change.
5. Roll back by removing the new layer/config/spec registrations; no production data migration is required.
