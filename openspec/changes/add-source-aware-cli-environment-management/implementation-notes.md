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

### Re-verification of 81/82/83 (task group 7)

Not taken on trust. `EXPECTED_MIGRATIONS` was parsed and checked mechanically: 83 entries, minimum
1, maximum 83, dense, no duplicate version, no duplicate name — so before this change the table ran
1..=80 with nothing free below 81.

All 84 local and remote refs were then scanned for migrations at 81 or above:

| Branch | Claims |
| --- | --- |
| `worktree-ocr`, `origin/worktree-ocr` | 81 `local-media-profiles` |
| `worktree-workspace` | 81 `execution-evidence-journal` |
| `worktree-skill-plugin-mcp` | 81–85 `extension-platform-*` |

Three unmerged branches already claim 81, and one claims through 85. **The higher free range 86–88
is nevertheless not usable**: `assert_migration_history_is_dense` (`migrations/mod.rs:639`) runs at
every startup and rejects any gap in the recorded history, so a database at 1..=83 that then
recorded 86 would refuse to boot. Density is a runtime invariant here, not a convention. 81/82/83 is
therefore the only genuinely free contiguous range for this branch, and whichever of the four
branches merges second must renumber — the collision is inherent to the shared-database design, not
to this choice. Leaving 84/85 empty for `worktree-skill-plugin-mcp` would have been strictly worse:
it would have broken both branches instead of one.

`migration_versions_are_unique_and_dense` (`migrations/tests.rs:637`) asserts uniqueness, ascending
order, density and name uniqueness over `EXPECTED_MIGRATIONS`, so a future collision fails a test
rather than a user's startup.

Four hard-coded assertions had to move to derived values, one more than the three found in the
earlier round:

| Location | Was | Now |
| --- | --- | --- |
| `migrations/tests.rs` `expected_versions()` | `(1..=80).collect()` | `expected_migration_versions()` |
| `migrations/tests.rs` migration-count assertion | `assert_eq!(migration_count, 80)` | derived from the list |
| `migrations/tests.rs` state assertion | `assert_eq!(migration_state, (79, 80))` | derived from the list |
| `platform/database/mod.rs:330` | `assert_eq!(migration_count, 80)` | derived from the list |

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

## Two defects the lifecycle work uncovered (task group 8)

Both were found by writing the cancellation tests task 8.11 asks for, not by review. Neither was in
the brief's list of seven regressions.

**Post-mutation detection inherited the operation's cancellation flag.** `verify_and_persist` took
its token from `operations.cancellation(operation_id)`, and `discover_and_probe` breaks out of its
probe loop the moment that token is set. So after a cancellation, detection observed nothing --
which made `changed-but-failed` structurally unreachable for exactly the case the five outcome
states exist to distinguish: a package manager interrupted after it had already replaced the binary.
Detection now runs on `CliCancellation::uncancelled()`. Cancelling an upgrade stops the package
manager; it must not also stop VaneHub from looking at what the package manager already did.

**"Not observed" was being read as "changed".** `machine_changed` compared
`Option<String>` versions directly, so a version that could not be re-probed (`Some("1.2.0")` before,
`None` after) counted as a change and reported a cancelled no-op as `changed-but-failed`. It now
requires positive evidence: two observed versions that differ, or an installation-count change from
a detection that actually completed. This is the same silence-is-never-consent rule the readiness
probes follow, applied to the axis where it had been missed.

## Phase reporting crosses the port boundary (task 8.1)

`CliDistributionPort::execute` gained a `&dyn CliPhaseSink`. Only the adapter knows where a download
ends and an install begins, and that boundary is exactly where cancellation stops being safe. The
vendor source reports `downloading` (cancellable) then `mutating` (not cancellable) around its own
steps; npm and WinGet fetch and write inside one invocation with no observable boundary, so they
report `mutating` for the whole call -- the safe direction to be wrong in, since cancel is then
never offered while a package manager may be writing.

Reported from the caller instead, this would have to either label a download `mutating` or keep
offering cancel during an install. Both are false statements about the machine.

## Verification-suite findings (task groups 7-9)

**`native:panic:check` had not been run in the earlier round.** It rejects `expect()` in
`--lib --bins`, and seven identifier constructions were failing it -- five that landed with the
source adapters and discovery, two in the new id factory. All are values this repository produces
itself, so `CliIdentifier::trusted` replaces the panic with a debug assertion. Recorded here
because the earlier round's report did not mention this command.

**Architecture line budget.** `src-tauri/src/platform/database` exceeded its recorded aggregate by
64 lines and the budget was raised in the same commit, with the reason stated inline: three
migration registrations, plus `migration_versions_are_unique_and_dense`, which is what turns a
cross-branch version collision into a failing test instead of an opaque `no such table` at a user's
next launch.

**One workspace test is nondeterministic and is not attributed to this change.**
`code_intelligence::infrastructure::server_test_tests::initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation`
failed once in `cargo test --workspace` (3991 passed, 1 failed). Evidence gathered rather than
assumed:

- `git diff --name-only main..HEAD` lists no file under `contexts/code_intelligence` or
  `platform/process`. The only `platform/` files this branch touches are the three under
  `platform/database`.
- The test spawns `node tests/fixtures/lsp_stdio_server.cjs lsp-hang` and shares one 2-second
  deadline between the initialize phase (which must time out, by design) and the forced
  process-tree cleanup that follows it. The cleanup therefore starts with the budget already spent.
- Re-run three times against **identical binaries** with no code change between runs: pass, pass,
  fail. A deterministic regression cannot produce that sequence.

Not claimed: that it also fails on clean `main`. Verifying that needs a checkout this
worktree-isolated session must not make, so the honest statement is "nondeterministic, and this
change cannot reach it", not "pre-existing".

**The two frontend regression tests remain red on purpose.**
`src/settings/pages/cli-lifecycle-regression.test.tsx` still fails its two assertions, because the
Settings page still calls the old `install_cli_version` path. They go green when task group 10
moves the frontend onto the new commands. Weakening or skipping them was explicitly out of bounds
and was not done.

## Baseline flaky reproduction (task group 10 round)

The `code_intelligence` timeout test was re-tested properly rather than argued about. A read-only
worktree was created at the merge base (`ee3eaf3f`, "Make the workspace scannable and recoverable
(#208)"), built in the same profile, and the single test run serially ten times. The same loop then
ran on this branch.

| Tree | Result | Per-run wall clock after the first build |
| --- | --- | --- |
| merge base `ee3eaf3f` | **10 passed, 0 failed** | 4.2s - 12.3s |
| `worktree-cli-management` | **10 passed, 0 failed** | 5.9s - 12.3s |

Neither tree reproduces it on an idle machine, so the earlier failures were not a property of this
branch. What they share is load: the failure appeared inside a full `cargo test --workspace`, and
again in a 3-run loop while builds were running, and never once in twenty idle runs across both
trees. The test's shape explains why -- a single two-second budget is shared between an initialize
phase that must time out by design and the forced process-tree cleanup that follows it, so the
cleanup starts with the budget already spent and a loaded machine finishes the kill too late.

Recorded as **load-sensitive, not branch-introduced**. The test was not deleted, ignored, or given a
longer timeout. The temporary worktree was removed.

## Vendor downloader (task 5.7)

`HttpsInstallerDownloader` closes the gap that kept the vendor source out of bootstrap, and the
source is now registered. Security properties, each with a test:

| Property | How it is enforced |
| --- | --- |
| HTTPS + host allowlist | `trust.permits_url` on the initial URL **and every redirect hop** |
| Redirect policy | client built with redirects disabled; hops followed manually, capped at 4 |
| Maximum size | checked per chunk while reading, not from a header afterwards |
| Timeout | deadline checked between hops and on every chunk |
| Cancellation | same cadence as the deadline |
| Temporary file | a VaneHub-owned directory removed on drop -- success, failure, timeout, cancel |
| Checksum | SHA-256 verified before execution; mismatch discards the file |
| No shell | `-File` on Windows, an interpreter file on Unix; no `curl|bash`, `wget|sh`, `irm|iex` |

Tests drive an in-memory body. No `https://` request is made and no installer is executed.

## `CliIdentifier::trusted` audit

Seven production call sites, all admitted:

| Site | Argument | Why it cannot come from outside |
| --- | --- | --- |
| `npm_source.rs` | `"npm"` | fixed source name |
| `winget_source.rs` | `"winget"` | fixed source name |
| `vendor_source.rs` | `"vendor"` | fixed source name |
| `environment_discovery.rs` | `"i-unknown"` | fallback literal; the dynamic value went through `new` first |
| `environment_serde.rs` | `"legacy"` | same |
| `environment_runtime_adapters.rs` | `self.next("cli-plan")` | ASCII prefix + counter + UUID, built in-process |
| `environment_runtime_adapters.rs` | `self.next("cli-bulk")` | same |

Two guards were added. Visibility is now `pub(in crate::contexts::tooling::cli)`, which puts it out
of reach of `commands/` entirely. Inside the context,
`no_external_input_reaches_the_trusted_identifier_constructor` pins the list above, so a new call
site has to be added to it deliberately.

## Database architecture budget audit

The 64 lines added to `platform/database` are three `apply_transactional_migration` registrations
(the schema bodies live in `contexts/tooling/cli/infrastructure/environment_schema.rs`), the
test-only `expected_migration_versions` accessor, and `migration_versions_are_unique_and_dense`.
No CLI source policy, action-plan rule, snapshot derivation, or row-to-domain decision is in that
subtree -- those live in `environment_schema.rs`, `environment_serde.rs`, and
`environment_repository.rs`. The raise stands as recorded.

## Migration numbers 81-83 remain provisional

They are the only genuinely free contiguous range on this branch, because
`assert_migration_history_is_dense` rejects a gapped history at startup and main ends at 80. They
are **not** final:

- `worktree-ocr`, `worktree-workspace` and `worktree-skill-plugin-mcp` each already claim 81, and
  the last claims through 85.
- The target branch must be re-scanned before merge or before any full desktop validation run.
- If it has taken 81 or above by then, renumber in a **separate commit** and update the four derived
  version assertions with it.

Every database and native integration test in this round used a temporary SQLite file. The user's
`%APPDATA%\ai.vanehub.app\vanehub.sqlite` was never opened.

## `src/services` line budget: open, not resolved

`npm run architecture:check` fails on one rule:

```
[ARCH-FE-004] src/services: 19485 aggregate physical lines exceeds budget 19234
```

The budget had **zero headroom** before this round: a previous change tightened it to its exact
measured value ("上限按实测值收紧，不保留任何一侧凭预估留下的余量"). Group 10 adds a nine-method
capability across two runtime adapters, which cannot cost zero lines.

What was done rather than raising it:

- The Tauri half was split into `tauri-cli-environment-client.ts` instead of growing
  `tauri-agent-client.ts` past its own 1213-line budget.
- The Web mock's snapshot data moved to `web-cli-environment-snapshots.json`, following
  `src/config/onepiece-provider-catalog.json`. That removed 185 lines of object literals from the
  measured set -- data, not code.
- `web-cli-tool-client.ts` (183 lines) and the four legacy adapter methods were deleted outright.

251 lines remain over. Resolving it needs a decision this round was told not to make: either raise
the budget by 251 with the reason stated in the same commit, which is what the previous entry in
that list did for its own new capability, or delete unrelated code from `src/services` to pay for
it. The overage is reported rather than absorbed.
