# Implementation Notes

## Task 0.2 — OpenSpec structure correction

`openspec validate add-source-aware-cli-environment-management --strict` initially failed with eight
errors. Every one had the same cause: a MODIFIED requirement had *renamed* a scenario the current
main spec still carries. OpenSpec matches scenarios by title and a MODIFIED block replaces the whole
requirement, so a renamed scenario reads as a silent deletion and archive refuses it.

Fix: keep the existing scenario titles, carry the new source-aware semantics in their bodies. No
behavior was dropped and no requirement was weakened.

| Spec | Original title kept | Draft title that was replacing it |
| --- | --- | --- |
| frontend-runtime-architecture | Desktop adapter returns native status | Desktop adapter returns native snapshot |
| frontend-runtime-architecture | Contract shape changes | Contracts change |
| frontend-runtime-architecture | Report refresh start failure / Report package start failure | Operation cannot be started (one merged scenario) |
| native-runtime-architecture | Start CLI package operation | Start CLI lifecycle execution |
| native-runtime-architecture | Capture CLI package operation logs | Capture lifecycle logs |
| native-runtime-architecture | Refresh after successful package operation | Verify after lifecycle execution |
| native-runtime-architecture | Install selected CLI version | Execute selected exact version |
| native-runtime-architecture | Reject unknown CLI operation target | Reject unknown target |
| native-runtime-architecture | Package mutation already running | Same tool mutation already exists |
| native-runtime-architecture | Detection during package mutation | Detection during mutation |
| native-runtime-architecture | Refresh command returns before detection completes / Package command returns before npm completes | Variable-duration command returns operation (one merged scenario) |
| native-runtime-architecture | Resolve package metadata from catalog | Resolve tool and source metadata |
| native-runtime-architecture | Refresh affected CLI after package success | Refresh affected CLI after terminal process |
| unified-log-management | Preserve existing operation UI logs | Preserve operation UI logs |

## Task 0.3 — Files that will be migrated

### Rust: CLI bounded subdomain (`src-tauri/src/contexts/tooling/cli/`)

| File | Lines | Fate |
| --- | --- | --- |
| `domain/mod.rs` | 621 | Split into focused modules; `LifecycleEligibility` removed |
| `application/service.rs` | 485 | Replaced by source-aware use cases |
| `application/ports.rs` | 93 | Replaced by discovery/distribution/probe/repository/coordinator ports |
| `application/models.rs` | 168 | `CliToolStatus` replaced by `CliEnvironmentSnapshot` |
| `application/error.rs` | 28 | Extended with typed categories |
| `application/tests.rs` | 634 | Rewritten against new ports |
| `infrastructure/package_adapter.rs` | 846 | Split into npm / WinGet / vendor source adapters |
| `infrastructure/detection_adapter.rs` | 421 | Split into discovery + probe adapters |
| `infrastructure/sqlite_repository.rs` | 522 | Extended with snapshot/catalog/plan storage |
| `infrastructure/candidates.rs` | 307 | Reused, with version-aware directory ordering |
| `infrastructure/executable_locator.rs` | 90 | Reused |
| `infrastructure/process_adapter.rs` | 58 | Extended with cancellation + output budgets |
| `infrastructure/runtime_adapters.rs` | 332 | Rewired to new ports |
| `infrastructure/support.rs` | 76 | Reused |
| `infrastructure/native_config_reader.rs` | 811 | Untouched (model discovery, unrelated) |
| `api.rs` | 66 | Republished for the new use cases |

### Rust: Tauri command layer

- `src-tauri/src/commands/tooling/cli/{list_cli_tools,refresh_cli_detections,install_cli_version,upgrade_all_cli_versions}.rs` — deleted after callers migrate
- `src-tauri/src/commands/tooling/cli/{dto,mapper,background,mod}.rs` — rewritten
- `src-tauri/src/commands/core_registry.rs:220-223` — command registration replaced
- `src-tauri/src/bootstrap/` — source registry assembly
- `src-tauri/src/platform/database/migrations/mod.rs` — new migrations
- `src-tauri/ARCHITECTURE.md` — ADRs for source adapters, action plans, partial completion

### Frontend

| File | Lines | Fate |
| --- | --- | --- |
| `src/settings/pages/providers-page.tsx` | 231 | Replaced by `src/settings/pages/cli-management/` module |
| `src/settings/pages/providers-page.test.tsx` | 252 | Replaced by interaction tests |
| `src/settings/pages/cli-environment-card.tsx` | 200 | Rewritten against the snapshot contract |
| `src/settings/pages/cli-management-utils.ts` | 73 | Deleted (frontend version/action derivation) |
| `src/settings/pages/cli-management-utils.test.ts` | 149 | Deleted with it |
| `src/settings/pages/cli-conflict-dialog.tsx` | — | Replaced by the action-plan review dialog |
| `src/services/cli-service.ts` | 51 | `CliToolService` methods replaced |
| `src/services/tauri-agent-client.ts:518-538` | — | Rewired to the new commands |
| `src/services/web-cli-tool-client.ts` | 180 | Deterministic plan/bulk/cancellation fixtures |
| `src/types/agent.ts` | — | Flat CLI status types removed |
| `src/types/operation.ts` | 24 | `OperationKind` gains `cli`; optional progress fields |
| `src/types/cli-environment.ts` | — | New |
| `src/contracts/agent.ts`, `src/contracts/operation.ts` | — | Contract drift coverage |
| `src/i18n/*` | — | New keys in every registered locale |

### Tests

- `src/services/web-agent-client.test.ts`, `src/contracts/contract-conformance.test.ts`,
  `src/i18n/i18n-representative-surfaces.test.tsx`
- `tests/desktop/specs/domain-cli-install.e2e.mjs`, `tests/desktop/specs/domain-cli-tooling.e2e.mjs`
- `tests/desktop/fixtures/cli/` — currently only `opencode`; gains fake npm/WinGet/vendor fixtures

## Task 0.5 — Competing change check

No other unarchived change under `openspec/changes/` references `CliToolStatus`, `cli_tool_status`,
`lifecycleEligibility`, `list_cli_tools`, or a CLI environment contract. This change is the sole
owner.

## Migration numbering

Highest migration on `main` is **80** (`retire-plan-execution`). This change claims **81, 82, 83**
for `cli_environment_snapshots`, `cli_version_catalogs`, and `cli_action_plans`.

Two known hazards, both previously observed in this repository:

1. All worktrees share one `%APPDATA%\ai.vanehub.app\vanehub.sqlite`. A colliding number across
   branches makes the second migration silently skip and surfaces as `no such table` at startup.
   `assert_migration_history_is_dense` is the guard.
2. Adding a migration breaks hard-coded version-count assertions that neither the compiler nor
   clippy catches. Re-scan for those when the migrations land.

## Confirmed defects, with source locations

Each of the seven regressions named in the implementation brief exists in the current code:

1. **Selected version never reaches the request** — `src/settings/pages/providers-page.tsx:32-34`
   (`resolveCliPackageActionTargetVersion` returns `tool.latestVersion ?? "latest"`), used at
   line 205 and passed to the mutation at line 222. `selectedVersion` (line 203) is display-only.
2. **Equality treated as upgradeable** — `src/settings/pages/cli-management-utils.ts:48` returns
   `"upgrade"` when the comparison is `0` and the lifecycle is managed. Lines 40, 42, and 45 do the
   same for missing/unparseable versions.
3. **Cross-source catalog** — `src-tauri/.../infrastructure/detection_adapter.rs:114-145` queries the
   npm registry whenever `package_name` is set, regardless of whether the active installation came
   from WinGet, Homebrew, a vendor installer, or a manual path.
4. **WinGet target version dropped** — `src-tauri/.../infrastructure/package_adapter.rs:320-337`
   builds `winget upgrade --id <id> --exact ...` with no `--version`; `target_version` is unused.
5. **Vendor silently falls back to npm** — `package_adapter.rs:138-155` runs npm after an installer
   failure when `fallback_npm_on_failure` is set at line 316.
6. **Stale snapshot written back after a real change** — `application/service.rs:423-427` saves
   `status.clone()` (the pre-operation snapshot) on any `Err`, including the case where the package
   command succeeded and only post-detection failed.
7. **Bash installer selected on Windows** — `domain/mod.rs:47-54` falls through to
   `ScriptInstaller::Shell` when no PowerShell URL exists, so `claude-code` yields a `bash -lc`
   plan on Windows (`package_adapter.rs:389-396`), and defect 5 then switches it to npm.

## Re-audit of the 45 previously checked tasks

Re-run of the baseline showed 7 commits present, a clean working tree, and strict validation
passing. Spot-checking every checked task against `impl` sites found **five checked without
satisfying all four conditions** (production code, matching test, test evidence, no dependency on an
unfinished stub). All five were reverted to `[ ]` with the reason inline in `tasks.md`.

| Task | Why it was not done |
| --- | --- |
| 3.5 | `run_bulk` discarded the real five-state outcome and recorded the literal string `"ran"`. The bulk tests asserted operation status and unit counts, never item outcomes, so the placeholder was never caught. |
| 3.8 | The one-per-tool, one-per-key, cap-two policy existed only inside `FakeCoordinator`, which is `#[cfg(test)]`. No production coordinator existed. |
| 3.9 | Only `a_completed_mutation_releases_its_reservation` existed -- the happy path. Nothing covered duplicate cancellation, release on the error path, or the plan being left in `Executing` when an early `?` skips `finish_action_plan`. |
| 5.7 | The only `impl CliInstallerDownloader` was in the test file. Bounded download, redirect policy, and checksum verification did not exist in production at all. |
| 5.11 | No source matrix test existed. Each adapter asserted its own catalog stamp separately, which does not establish the cross-adapter property. |

Evidence: `grep "impl <Port> for"` across `src-tauri/src` returns production implementations for
only `CliProbePort` (`SystemCliProbe`) and `CliDiscoveryPort` (`SystemCliDiscovery`). Every other
port is satisfied solely by a double in `environment_test_doubles.rs`.

Corrected baseline: **40 done, 115 open** before this round's work.

## Duplicate-installation conflict contract (section 2 audit)

The contract was **not** expressible before this round, so `design.md`, the
`cli-environment-management` delta spec, and `tasks.md` were updated first and re-validated under
`--strict` before any code changed.

- `active_installation_id` is replaced by `path_selected_installation_id` and
  `recommended_installation_id`. PATH order alone decides the first; probe results decide the
  second. The executable axis follows the **PATH-selected** launcher, because it describes what the
  host runs -- reporting Healthy because a working copy sits further down would describe a machine
  the user does not have.
- Conflicts are typed values carrying `severity`, `installations`, `blocks_mutation`,
  `blocks_launch`, and a stable `reason_code`. All nine kinds are implemented and covered.
- Launcher families group `tool` / `tool.cmd` / `tool.ps1` into one logical installation on Windows
  only; folding stems on Unix would merge genuinely different programs.
- `blocks_mutation` now withholds every mutating action in `derive_allowed_actions` and excludes the
  tool from a bulk batch as `installation-conflict`.

**Agent Runtime (task 4b.9): already correct.** `CliProfileSnapshot` launches through
`CliApi::resolve_executable`, which returns a resolved path from the bounded candidate list, not a
bare command name. A contract test now pins that (`the_resolver_never_hands_the_runtime_a_bare_command_name`).
No widening of this change into Agent Runtime was needed.

**Deliberately out of scope this round**, per the instruction: PATH repair, duplicate removal,
source migration, user-preferred-installation persistence, Homebrew/Bun/Volta lifecycle, and
WSL/SSH/container scopes. The platform tests assert classification and grouping only; enumeration
order, the executable bit, and `noexec` need the temporary-PATH desktop fixtures from task 12.10.
