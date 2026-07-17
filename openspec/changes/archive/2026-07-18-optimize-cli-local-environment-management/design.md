## Context

The current CLI Management flow stores one resolved executable path per supported CLI and derives package actions from npm versions. Detection uses `where`/`which`, takes the first result, and package mutation always runs npm. This is insufficient on developer machines where a desktop bundle, WinGet installation, and npm global shim can coexist. The first PATH entry can differ from the path npm mutates, and an executable can exist while `--version` fails.

CC Switch provides useful reference behavior for enumerating installations, marking the PATH default, reporting broken tools, confirming conflicts, and anchoring updates. VaneHub will reimplement the relevant ideas behind its existing Agent service boundary, Tauri/Web adapters, Rust process-safety helpers, SQLite cache, task registry, unified logging, semantic theme tokens, and zh-CN/en resources. The reference repository is MIT licensed, but no components, branding, or additional dependencies will be copied.

## Goals / Non-Goals

**Goals:**

- Make the active CLI executable and all discovered installations visible and testable.
- Prevent a package operation from silently mutating a different installation than the active CLI.
- Distinguish missing, runnable, broken, conflicting, unsupported, and undetected states.
- Support cached initial rendering, all-tool refresh, and single-tool refresh without blocking React or Tauri command boundaries.
- Keep command strings backend-owned and serialize global package mutations.
- Provide a compact, localized CLI environment UI that works in `futuristic` and `minimal` styles.
- Keep a technical note that records the first-version implementation and prioritized optimization paths.

**Non-Goals:**

- Adding new managed CLI products beyond the existing four.
- Full WSL discovery or WSL-specific lifecycle execution in the first version.
- Automatically invoking every detected source's native updater such as WinGet, Homebrew, Volta, pnpm, bun, or vendor-specific installers.
- Removing stable-version selection or the existing task/log experience.
- Duplicating full CLI lifecycle controls on the About page.

## Decisions

1. Extend the existing CLI status contract instead of adding a second diagnostics service.

   `CliToolStatus` remains the cached service-facing aggregate and gains `environmentType`, `installations`, `activeInstallationPath`, and `conflictState`. Each installation reports path, version, runnable state, error, source, and whether it is the active PATH entry. This keeps the CLI page, About summary, Tauri adapter, Web adapter, and contract conformance checks on one normalized model.

   Alternative considered: add a separate `probeCliInstallations` contract used only by a diagnostics dialog. That would create two sources of truth and make cached rendering inconsistent with diagnostics.

2. Enumerate bounded backend-owned candidate paths and deduplicate canonical executable targets.

   The Rust layer starts with all PATH results, then checks a small platform-specific catalog of known user-level locations for npm and common desktop/native installs. Candidate paths are normalized and deduplicated before bounded `--version` probes. The PATH-first candidate becomes active. Source classification is descriptive and based on path shape; it does not grant permission to execute arbitrary package-manager commands.

   Alternative considered: recursively scan disks. That is slow, invasive, and unbounded.

3. Keep remote version discovery separate from local installation health.

   Local probing and npm metadata can partially succeed independently. `installed` reflects whether at least one installation exists, `currentVersion` reflects the active runnable installation, and `versionCheckStatus`/`lastError` retain partial failure information. A present but non-runnable executable is not shown as missing.

4. Implement a conservative first-version lifecycle plan.

   A backend planner derives whether an npm operation is safe from the active installation source. Missing tools can use the existing backend-owned npm catalog. A single npm-managed active installation can use the existing versioned npm operation. Multiple installations, a non-npm active source, or a broken active installation require confirmation and show a localized explanation; non-npm active sources default to manual/source-native guidance rather than silently installing another npm copy. The frontend never submits command text.

   Alternative considered: immediately implement native updates for every source. That would multiply platform-specific command, elevation, rollback, and test requirements beyond a safe first version.

5. Serialize package mutations but keep read-only interaction responsive.

   CLI install/upgrade/downgrade operations share a backend mutation guard because global package managers can write overlapping directories. While a mutation runs, mutation controls are disabled across CLI cards; navigation, card expansion, cached reads, and safe detection remain responsive. Detection refresh remains independently asynchronous.

6. Keep CLI Management as the operational surface and About as a summary.

   CLI Management adopts compact environment cards, per-tool refresh, diagnostics, conflict disclosure, manual commands, and operation logs. About keeps product identity/update information and adds only installed/attention counts plus a link to CLI Management. This follows CC Switch's information hierarchy without creating duplicate lifecycle state machines.

7. Use semantic UI roles and synchronized locale keys.

   Cards, badges, warnings, dialogs, and logs use existing `ucd-*` classes and semantic tokens. No component branches on `futuristic` or `minimal`. New user-facing copy is added to both locale files and covered by parity and visible-text guardrails.

8. Keep the technical note as an implementation handoff document.

   `implementation-notes.md` records the shipped first-version behavior, deliberate limitations, data/command boundaries, and prioritized follow-ups. Tasks require updating it when implementation details change so it remains useful after the change is archived.

## Risks / Trade-offs

- [Known-path catalogs can become stale] -> Keep discovery helpers isolated, PATH enumeration authoritative, and document extension points and fixtures.
- [Source classification can be imperfect] -> Treat it as descriptive metadata; only allow automatic npm mutation when the active target is positively classified as npm-managed.
- [Probing many candidates can be slow] -> Bound candidate count and per-command timeout, deduplicate first, cache results, and support targeted refresh.
- [Multiple global writes can corrupt an installation] -> Add a backend mutation guard and matching UI busy state.
- [SQLite migration can invalidate old cache rows] -> Add nullable/defaulted columns or a separate JSON field and map old rows to empty installation details.
- [About and CLI pages can drift] -> Both consume the same service model; About renders summary only.
- [Cross-platform source handling differs] -> First-version automatic mutation stays conservative and tests source classification with platform-neutral path fixtures plus Windows-specific cases.

## Migration Plan

1. Extend shared TypeScript and Rust models with additive fields and update adapter fixtures.
2. Add an additive SQLite migration for detailed installation data and lifecycle/conflict metadata.
3. Implement bounded enumeration, probing, classification, persistence, and targeted refresh.
4. Add conservative lifecycle planning and the mutation guard while preserving existing operation ids/logs.
5. Update CLI Management and About UI, locale resources, and tests.
6. Update `implementation-notes.md`, validate the OpenSpec change, and run the full project verification suite.

Rollback restores the previous UI/service/backend code. Additive database fields remain harmless and older cached rows continue to map to an empty detailed-installation list.

## Open Questions

- WSL discovery, source-native package updates, and richer automatic repair remain explicitly deferred and prioritized in `implementation-notes.md`.

