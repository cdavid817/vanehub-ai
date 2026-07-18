## 1. Service And Runtime

- [x] 1.1 Extend the frontend agent service contract and both runtime adapters with a bulk CLI upgrade method.
- [x] 1.2 Add a desktop Tauri command and native asynchronous operation for bulk CLI upgrades.
- [x] 1.3 Add Web/mock behavior that preserves the service contract with localized unsupported-operation feedback.

## 2. Settings UI

- [x] 2.1 Add a compact local environment check toolbar action for diagnosing conflicts, refreshing, and upgrading all eligible CLIs.
- [x] 2.2 Wire bulk operation state into CLI cards so package mutation is isolated per CLI and logs remain visible.
- [x] 2.3 Polish the About page environment summary copy/layout without duplicating CLI lifecycle controls.
- [x] 2.4 Add synchronized zh-CN and en i18n resources for all new visible text.

## 3. Tests And Validation

- [x] 3.1 Add or update focused frontend tests for bulk upgrade eligibility and toolbar state.
- [x] 3.2 Add or update Rust tests for bulk upgrade selection, skip logging, and command registration behavior where practical.
- [x] 3.3 Run `openspec validate optimize-cli-management-and-about --strict`.
- [x] 3.4 Run `npm run test`, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `openspec validate --specs --strict`.

## 4. Follow-up CLI Guidance Optimization

- [x] 4.1 Replace generic manual-action copy with cause-specific guidance modeled after cc-switch's diagnostic flow.
- [x] 4.2 Add focused tests for broken, multi-installation, and non-npm source guidance.
- [x] 4.3 Re-run focused frontend tests and OpenSpec change validation.
- [x] 4.4 Surface non-npm newer-version upgrade state while preventing unsafe npm mutation.

## 5. Source-Aware CLI Install Planning

- [x] 5.1 Add backend lifecycle planning for npm and wget-script methods, preferring wget scripts for first install when available.
- [x] 5.2 Preserve the detected managed source during upgrades and keep unsupported sources manual.
- [x] 5.3 Update frontend eligibility/types/i18n so wget-script managed CLIs can show executable install/upgrade controls.
- [x] 5.4 Add focused Rust and frontend tests for method selection and source-aware UI behavior.
- [x] 5.5 Re-run focused tests, build, and OpenSpec validation.
- [x] 5.6 Keep per-CLI one-click upgrade visible for managed installs even when latest metadata is unavailable.
- [x] 5.7 Avoid launching Windows `.ps1`/shell shims directly during CLI detection and session startup.
- [x] 5.8 Sanitize stale cached Windows script-launch errors so old `%1` failures stop rendering after the fix.
- [x] 5.9 Keep per-CLI upgrade action visible for installed managed CLIs even when already current.
- [x] 5.10 Keep transient CLI refresh warnings out of persisted status data while still showing them in refresh logs.
- [x] 5.11 Replace copy-install-command UI with a one-click latest upgrade action.
- [x] 5.12 Increase CLI version probe timeout to avoid false installed-but-unrunnable status for slower Windows CLIs.
- [x] 5.13 Preserve WinGet-installed CLI upgrades by running verified `winget upgrade` plans.
- [x] 5.14 Reclassify cached WinGet statuses so old manual guidance does not persist after upgrade support is added.
- [x] 5.15 Run CLI refresh and bulk upgrade workers independently so one CLI cannot block unrelated CLIs.
