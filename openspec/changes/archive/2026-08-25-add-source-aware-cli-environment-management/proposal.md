## Why

VaneHub AI already has a useful CLI Management surface and a correctly layered Rust `tooling::cli` bounded subdomain, but the current implementation still behaves as a detector plus a partially managed installer rather than a reliable local CLI environment manager.

The current flat status contract combines installation discovery, executable health, update availability, source manageability, conflicts, and readiness. The frontend therefore has to infer actions from incomplete fields. This has produced concrete correctness defects: the selected version is not used by the actual mutation request, an installed version equal to the latest version is still presented as upgradeable, npm catalog data is applied to installations owned by other sources, WinGet execution does not consistently honor the requested version, vendor installer failure can silently fall back to npm, and a successful machine mutation followed by verification failure can restore stale pre-operation state in SQLite.

The lifecycle model is also source-agnostic. npm, WinGet, audited vendor installers, Homebrew, Bun, Volta, desktop bundles, system packages, and manually installed executables do not have the same version catalog or mutation capabilities. Treating them as one `npm/wget/winget/manual` eligibility enum creates false actions and unsafe cross-source behavior.

This change turns CLI Management into a source-aware environment management workflow. The backend becomes the only authority for active installation selection, source capability, version comparison, allowed actions, action planning, preflight checks, execution, verification, partial-completion semantics, and diagnostics. The frontend renders those decisions and never guesses lifecycle behavior.

## What Changes

- Replace the flat `CliToolStatus` lifecycle model with a normalized local CLI environment snapshot containing:
  - bounded installation discovery;
  - active installation identity and source confidence;
  - executable, authentication, readiness, compatibility, update, freshness, and conflict states;
  - source-specific version catalogs and capabilities;
  - backend-derived allowed actions and overall summary state.
- Replace frontend-derived install/upgrade/downgrade decisions with persisted, expiring backend `CliActionPlan` aggregates.
- Add source adapters with explicit capability declarations:
  - npm: exact-version install, upgrade, downgrade, reinstall, and uninstall;
  - WinGet: source-aware install, exact-version upgrade when supported, uninstall, and dynamic repair support;
  - audited vendor installer: platform-specific latest-version install/update only unless an exact-version template is explicitly declared;
  - Homebrew, Bun, Volta, desktop, system, manual, and unknown sources: detection and guidance only in this change.
- Remove silent source fallback. A plan executes exactly one disclosed source or fails.
- Make Windows installer selection platform-safe. A Bash installer is never selected on Windows unless a future source definition explicitly requires and preflights a supported Bash runtime.
- Download vendor scripts to a bounded temporary file and execute that file; do not use pipe-to-shell or `irm | iex`.
- Add provider-specific read-only Doctor and authentication probes where a documented non-interactive command exists. Unknown is returned instead of guessing.
- Add single-tool and bulk action-plan preparation, review, execution, cancellation, progress, per-item outcome, and post-mutation verification.
- Preserve actual machine state after partial completion. Never overwrite a changed machine with a cached pre-operation snapshot.
- Add additive SQLite persistence for environment snapshots, source catalogs, action plans, and plan state while preserving legacy CLI status data for migration.
- Extend the common operation contract with CLI operation kind, phase, bounded progress, and cancellability without changing existing lifecycle statuses.
- Redesign Settings → CLI Management as a compact operational surface with summary counts, filtering, orthogonal status badges, a details drawer, plan review dialogs, bulk preview, stale-data handling, and per-tool operation state.
- Keep Tauri and Web/mock adapters contract-compatible. Web/mock behavior remains deterministic and does not invent host paths, installed versions, credentials, or package-manager side effects.
- Update user documentation and architecture records so npm-only documentation no longer contradicts the native implementation.

## Capabilities

### New Capabilities

- `cli-environment-management`: Source-aware local CLI discovery, readiness, version catalogs, action planning, lifecycle execution, verification, Doctor probes, conflict reporting, and bulk operations.

### Modified Capabilities

- `native-runtime-architecture`: Replace npm-centric CLI lifecycle requirements with source adapters, persisted action plans, source-specific execution, partial-completion handling, and additive environment persistence.
- `frontend-runtime-architecture`: Replace the flat CLI status adapter contract with normalized environment snapshots and plan-driven async operations.
- `settings-center-ui`: Define the CLI Management information architecture, action review, bulk preview, details drawer, status presentation, accessibility, and stale-data behavior.
- `contract-and-task-foundation`: Make CLI work a first-class observable operation kind with optional phase, progress, and cancellation metadata.
- `unified-log-management`: Cover plan, Doctor, install, upgrade, downgrade, reinstall, uninstall, repair, verification, truncation, and pre-UI redaction.

## Impact

### Native code

- `src-tauri/src/contexts/tooling/cli/domain/`
- `src-tauri/src/contexts/tooling/cli/application/`
- `src-tauri/src/contexts/tooling/cli/infrastructure/`
- `src-tauri/src/contexts/tooling/cli/api.rs`
- `src-tauri/src/commands/tooling/cli/`
- `src-tauri/src/commands/registry.rs`
- `src-tauri/src/bootstrap/`
- `src-tauri/src/contexts/operations/`
- `src-tauri/src/platform/database/`
- `src-tauri/src/platform/process/`
- `src-tauri/src/platform/logging.rs`
- `src-tauri/ARCHITECTURE.md`

### Frontend code

- `src/types/operation.ts`
- new `src/types/cli-environment.ts`
- `src/services/cli-service.ts`
- `src/services/agent-service.ts`
- `src/services/tauri-agent-client.ts`
- `src/services/web-agent-client.ts`
- Settings page registry and CLI Management components
- registered locale resources and i18n regression tests

### Persistence

- Additive tables for versioned environment snapshots, version catalogs, and single-use action plans.
- Legacy `cli_tool_status` rows remain readable during migration but are no longer the authoritative write model after cutover.
- Existing operation and unified log history remains intact.

### Compatibility

- Stable CLI Agent ids remain unchanged.
- Existing routes and Settings navigation destination remain unchanged.
- Existing operation lifecycle statuses remain unchanged.
- Old CLI service methods and Tauri commands may exist only as an explicitly tracked migration step; they must be removed after all internal callers migrate and before the change is marked complete.
- No external credential format, provider config file, or provider login flow is changed.
