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

## Migration numbers 81-83 were provisional (superseded: see round 5)

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

## Round 4: Playwright, the legacy cutover, the native desktop layer, and documentation

Tasks 12.9-12.13, Group 13, and 2.1 / 2.4 / 7.8. The ledger stands at 144 / 164, with only Group
14's twenty validation items open.

### Ledger corrections

Two claims from the previous round's report were bookkeeping errors, corrected here rather than
repeated:

- **99 to 105 was two commits, not one.** `90152282` ticked 3.5, 3.8, 3.9 and `8d10231f` ticked 4.8,
  5.10, 5.11. The six tasks are right; they did not land together.
- **105 to 113 to 129 never happened as two steps.** Group 11 and 12.1-12.8 were both ticked in a
  single commit, `7499011b`, taking the ledger from 105 straight to 129. The *work* landed in
  separate commits (`eb7505ef` for the module, `95b0a653` for the tests); only the ledger update was
  deferred to the end.

Nothing was ticked without implementation behind it. Five tasks were ticked and then un-ticked at
`306e6faa` when an earlier round found the claims premature, and re-ticked later with evidence.

### The legacy cutover made the launch path a single authority

The Agent Runtime resolved which executable to launch from the pre-change `cli_tool_status` row
while the CLI Management page reported the source-aware snapshot. On a host with two installations
those are different answers. `CliApi` is now a one-method facade over `CliEnvironmentApi` that reads
the same snapshot the page renders.

**No `LegacyLifecycleEligibility` compatibility type was needed.** The read-only legacy reader
selects `detected_path`, `current_version`, and `last_checked_at` and never decoded an eligibility
column, so `LifecycleEligibility` was deleted outright rather than surviving in a decoder.

`the_legacy_cli_table_has_exactly_one_reader_and_no_writer` fails the build if a second reader or
any writer appears.

### The native layer found four defects nothing else could

`npm run test:desktop:cli-management` is a new sixth layer. Building it, and running it, surfaced:

1. **Bulk preparation inserted every item plan twice** and died on the primary key. The in-memory
   double overwrote by key and hid it; it now refuses duplicates the way the table does.
2. **Discovery stopped at the first launcher per directory**, so the launcher family was never fully
   seen and the alias list the details drawer promises was always empty.
3. **An expired, consumed, or superseded plan returned an operation id** and reported the refusal as
   a failed operation, which closed the review dialog as though the change had started. Refusals
   that are already decided now come back from preparation.
4. **A completed operation with a `no-change-failed` outcome rendered a green success badge.**

Two more were in the fixture harness itself: the wdio config is re-imported per worker and its
rebuild wiped the tree out from under a running spec, and appending the inherited PATH let discovery
walk past the fixture into the developer's real npm global directory and report it as a finding.

### Side-effect safety is audited, not assumed

`scripts/desktop/cli-side-effect-guard.mjs` checks a finished run and fails closed on a missing
invocation log, a binary that answered from outside the fixture, a record without the fixture
marker, a command naming a real registry or vendor host, a pipeline into a shell, a credential in
the environment, or a data directory inside the user's profile. Its own tests feed it each violation
and check that it refuses.

The fixture owns `PATH`, `PATHEXT`, `APPDATA`, `LOCALAPPDATA`, `USERPROFILE`, and `HOME`. All of
those turned up a real installation on the machine this was first run on.

### `src/services` line budget: resolved

Raised 19234 to 19516 in `af031088`, the exact measured value with no headroom and the reason stated
in the source. This round kept the subtree inside it by moving more mock data to JSON rather than
raising it again.

### Localization

234 keys were added across this change. Round 3 shipped them with `zh-TW` copying `zh-CN` and
`ja`/`ko` copying `en`, which passed key parity and still shipped an untranslated page. All 234 are
now translated in all three, and `i18n-resource-parity.test.ts` asserts that representative
sentences differ from both reference locales and that every CLI placeholder survives.

### Still provisional

- **Migrations 81-83.** (Superseded in round 5: main took 81, so these are now 82-84.) Re-scanned at the start and end of this round; still the only dense range,
  and still provisional because three unmerged branches also claim 81. Renumbering is a merge-time
  decision, and `assert_migration_history_is_dense` refuses a gap, so a higher free range cannot be
  reserved in advance.
- **Screenshots.** `assets/screenshots/cli-en.png` and `cli-zh-CN.png` predate the page rebuild and
  show the old summary counters. Regenerating them needs a clean `npm ci` worktree, which this round
  did not have.
- **Two error variants and two request fields are declared and unraised.** `MissingDependency`,
  `ElevationRequired`, `CliPlanRequest::channel`, and `CliPlanRequest::package_reference` carry
  `expect(dead_code)` with the reason on each; no shipped adapter reaches them yet.

## Round 5: merged with main, migrations finalized at 82-84

### The target branch had moved

`origin/main` advanced to `c37caa4a feat(cli-parameters): upgrade CLI parameter management (#210)`
between the last round and this one. It is merged in, not rebased over: the 38 commits before it
are untouched.

Fifteen files conflicted. Three resolutions were decisions rather than concatenations:

**Migration numbering is final at 82, 83, 84.** main now occupies 81 (`cli-parameter-profiles`), so
this change's three tables take the next consecutive numbers. Not a higher range:
`assert_migration_history_is_dense` refuses a gapped history at startup, so reserving room for
unmerged branches would make the app unbootable. Every derived assertion in this repository already
reads the migration list rather than a literal, which is why renumbering touched the registry and
one doc comment and nothing else. 69 migration tests pass at the new numbers.

**The parameter context's compatibility adapter.** `#210` added
`CliLifecycleSnapshotAdapter`, which read `CliApi::list_tools` and `ConflictState` -- both deleted
with the flat lifecycle stack in round 4. It now asks `CliApi::installation_facts`: five facts read
off the environment snapshot rather than the snapshot itself, because the parameter context needs to
know which version a flag would reach, not how to change it. The path it gets back is the one the
runtime would launch, so a parameter is judged against the binary that will actually receive it
rather than against a second detector's answer.

**The `src/services` budget.** Both sides raised it -- 19516 here, 19727 there. The merged value is
measured on the merged tree, **19803**, not the sum of the two: main's deletion of
`cli-parameter-catalog.ts` pays for part of this side's adapters.

Both settings pages stay code-split. The parameters page moved directory on main, so its old path
leaves the chunk check and the visible-text guard rather than sitting beside the new one.

## Round 6: final validation, and the two audits that found something

### The target branch moved twice more

`origin/main` advanced twice during this round: first to `48d6ea90` (two
dependency bumps, one of them `expect-webdriverio` 5.6.5 -> 6.0.5), then to
`42b6a649` (`c48f556f fix(test): stabilize desktop WebDriver worker lifecycle`,
`a4ce6b28 chore(release): prepare v1.1.0`, and a docs correction). Both merged,
neither rebased -- the branch's own commits are untouched. The first merge
conflicted only in `package.json`/`package-lock.json`; the second was clean.

### Migration numbering: re-verified mechanically, unchanged

Scanned rather than recalled. `origin/main`'s highest migration is still **81**
(`cli-parameter-profiles`), so this change keeps **82, 83, 84**. The registry
holds 84 registrations across both `apply_migration` and
`apply_transactional_migration`, dense from 1 to 84, no duplicates, and the three
this change owns are exactly the next three consecutive numbers after main's
maximum. No renumbering was needed, so no renumbering commit was made. Every
derived assertion still reads the migration list rather than a literal, so
nothing else needed syncing.

### Audit 1: the Agent Runtime resolution path

`CliApi` holds one field, `CliEnvironmentApi`, and both its methods read the
snapshot the CLI Management page renders. It has four consumers -- Agent Runtime
availability and CLI profile, CLI delegation's tool execution and passive probe
-- all through that one entry point. The two other functions in this repository
named `resolve_executable` belong to `code_execution` (locates `python`/`node`
via `where`/`which`) and `skill_tools` (locates a skill binary); neither manages
a CLI agent, so neither is a second authority.

The contracts hold, each with a test that would notice if it stopped holding:
the answer is always absolute, a `blocks_launch` conflict refuses rather than
picking a winner, a broken recommended installation refuses rather than silently
swapping to another copy, a scanned-and-empty host refuses **without** a live
lookup while a never-scanned tool gets exactly one bounded one, and
recommended-before-PATH-selected is what `launch_target_of` encodes. The legacy
table's `active_installation_path`, `lifecycle_eligibility` and `conflict_state`
columns are read by nothing.

### Audit 2: the legacy boundary, where the split had been rebuilt

`cli_tool_status` survives, no production statement writes it, and the
architecture test that enforces this passes. But the audit found the
page-and-runtime divergence this context exists to end, reassembled inside the
compatibility shim.

`load_snapshot` fell back to a leftover legacy row when no authoritative snapshot
existed. `list_snapshots` did not. `list_cli_environments` -- what the page reads
-- synthesises `never_scanned` for an agent with no row. So an upgrading user,
before their first refresh, saw a never-scanned tool on the page while
`resolve_launch_target` read the legacy row, found `FoundOne` and an `Unknown`
(therefore not faulty) executable, and resolved a launch to the old
`detected_path`. The runtime started a binary the page did not show and nothing
had verified. `installation_facts` inherited the same split, so the parameter
context judged flags against a version the user could not see.

Fixed by having one query serve both reads, which is also what makes "exactly one
reader" mean something: the invariant was never that one SQL statement exists, it
was that both consumers see the same thing.

A second, smaller finding: the `LEGACY_FINGERPRINT` comment claimed the value
made a legacy snapshot "always rejected as the basis for a mutation". It does not
-- planning computes the live fingerprint and stamps the plan with that, so
`legacy-import` is never on either side of that comparison. The guarantee holds
by another route: a legacy snapshot carries no `allowed_actions`, and the row's
`Unknown` source confidence fails `owns_active_installation`. The comment and the
test now say that instead.

### Three desktop specs still called commands this change deleted

Running every layer -- not only the CLI one -- found `ui-cli-management`,
`domain-cli-install` and `native-flows` still invoking `list_cli_tools`,
`refresh_cli_detections` and `install_cli_version`. The first two are superseded
by the `desktop-cli-management` layer, which covers the same ground against a
fixture instead of the developer's machine, and were deleted. `native-flows`
kept its opencode reinstall, ported onto prepare/execute: it is the only test
here that drives a real package manager against a real host, and a fixture cannot
show that the real npm, the real global prefix and the real PATH agree.

### Group 14 command results

Final HEAD `1081655b`, `origin/main` `42b6a649`, 47 commits ahead. Every command
below ran on that HEAD, serially, one at a time.

| Task | Command | Result |
| --- | --- | --- |
| 14.1 | `openspec validate add-source-aware-cli-environment-management --strict` | exit 0 |
| 14.2 | `npm run lint:ci` | exit 0 |
| 14.3 | `npm run test` | 306 files, 1462 tests, 0 failed |
| 14.4 | `npm run test:coverage` | 306 files, 1462 tests, 0 failed; lines 76.13%, statements 72.44%, branches 67.93%, functions 68.95% |
| 14.5 | `npm run coverage:policy:test` | 5 passed |
| 14.6 | `npm run version:unit:test` | 9 passed |
| 14.7 | `npm run contracts:check` | 3 files, 16 tests |
| 14.8 | `npm run architecture:check` | 48 passed |
| 14.9 | `npm run build` | 16 lazy chunks, 137.9 KiB gzip static closure |
| 14.10 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | exit 0 |
| 14.11 | `cargo check --workspace` | exit 0 |
| 14.12 | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| 14.13 | `npm run native:panic:check` | exit 0 |
| 14.14 | `cargo test --workspace` | 4263 passed, 0 failed, 15 ignored |
| 14.15 | `npx playwright test` | 175 passed, exit 0 |
| 14.16 | `npm run desktop:unit:test` | 30 passed |
| 14.18 | `openspec validate --specs --strict` | 136 passed, 0 failed |

Three flakes, all handled by isolating and re-running rather than by loosening
anything. No timeout was widened, no assertion weakened, nothing skipped.

- `test:coverage` once failed `onepiece-context-health-section.test.tsx:80` on a
  `findByText` timeout. Alone with coverage: 8/8. Full re-run: green.
- `npx playwright test` failed once on `hybrid-local-model-runtime.spec.ts:39`
  and once, on the next attempt, on `application-locales.spec.ts:74` -- a
  different test each time. Each passed in isolation (5/5 and 4/4), and the third
  full run was green at 175/175.

None of the three is in a file this change touches. The suite duration over
those three Playwright runs went 14.4m, 18.1m, 16.4m against 12.0m earlier in the
day, which is the same host-load signal the desktop layer shows below.

### 14.17: the native desktop matrix is incomplete

**Windows x64 -- five of six layers PASSED.**

| Layer | Result |
| --- | --- |
| `desktop-cli-terminal` | PASSED |
| `desktop-cli-management` | PASSED (2 specs) |
| `desktop-session-workspace` | PASSED |
| `desktop-dialogs` | PASSED |
| `desktop-settings-persistence` | PASSED (2 specs) |
| `desktop-smoke` | **FAILED** |

`desktop-cli-management` is the layer that covers this change, and it passes
against a fixture that owns `PATH`, `PATHEXT`, `APPDATA`, `LOCALAPPDATA`,
`USERPROFILE` and `HOME`, so its SQLite is a temporary database and its CLIs and
package manager are fakes. Its side-effect guard runs as an assertion inside the
passing spec and reported no violation: no real npm, no WinGet, no vendor URL, no
credential store, no user database.

`desktop-smoke` does not pass on this host, and not for a reason this change
introduced. It sets none of those variables, so it runs against the developer's
real PATH and real OS home. Six runs:

> **Round 7 corrects two claims in this section.** The application's own database
> was never the developer's: `createRunContext` has always given each run its own
> `VANEHUB_APP_DATA_DIR`, and `validateIsolatedDataPath` refuses a path that
> aliases real application data. What was shared is the OS home and PATH. And the
> six results below are not, as this section concludes, host contention: two
> harness defects were doing most of the work, and with them fixed all six layers
> pass on this host in one run. See "Round 7".

| Run | Result | Failing spec | Symptom |
| --- | --- | --- | --- |
| A | 29 passed / 3 failed | `ui-cli-management`, `domain-cli-install`, `native-flows` | called deleted commands -- **a real defect, fixed** |
| B | 28 / 2 | `domain-loop`, `screen-sweep` | app process fast-fail (`0xC0000409`) |
| C | 29 / 1 | `domain-prompt-hooks` | SQLite `database is locked` |
| D | 29 / 1 | `feature-sweep` | `EBUSY` removing a temp terminal directory |
| E | 27 started, 0 failed | 3 specs never started | embedded WebDriver not ready on fixed port 4445 within 120s |
| F | 29 / 1 | `domain-skills` | failed, retried, no assertion recorded |

After run A's real defect was fixed, five runs produced five different failing
specs and five different infrastructure symptoms, none CLI-related, and every
spec passed in at least one run -- runs C and D used identical code and failed on
different specs. Concurrent load on the host was visible throughout: a foreign
`vanehub_ai_lib-*` test binary under a different build hash, and 13 orphaned
`msedgewebview2.exe` processes. `origin/main` shipped
`c48f556f fix(test): stabilize desktop WebDriver worker lifecycle` mid-round,
which is the same class of problem; it is merged here and run E still hit the
port-4445 timeout.

This is reported as FAILED -- not as PASSED, and not as flaky-therefore-green.

**macOS -- NOT RUN. Linux -- NOT RUN.** No runner is reachable from this
session, the branch is not pushed, and there is no PR, so no CI has executed. The
repository's existing three-platform `Desktop Smoke` matrix runs the smoke layer
only, by design, so a `desktop-full` job was added to `.github/workflows/ci.yml`:
`workflow_dispatch` with a `desktop_full_suite` input, the same three runners,
`VANEHUB_DESKTOP_FULL_SUITE=1`, `xvfb-run` on Linux, evidence uploaded
`if: always()`. It is wiring on the existing harness rather than a parallel
framework, and it has never run.

**14.17 and 14.20 stay unchecked.** Completing them needs a real macOS run, a
real Linux run, and a green Windows `desktop-smoke` on an uncontended host. A
fixture unit test is not a substitute for a platform result, and NOT RUN is not
PASSED.

## Round 7: the matrix actually ran, and what it found

### Two harness defects, not host contention

Round 6 attributed six consecutive desktop failures to load on the developer's
machine, because the failing spec moved every run and every spec passed in some
run. Non-determinism was real; the cause was not the machine.

**`browser.tauri.execute` never used the WebDriver session.** It opens its own
connection and resolves the port from `TAURI_WEBDRIVER_PORT` *in the worker
process*, defaulting to 4445, while `@wdio/tauri-service` sets that variable only
for the application it spawns. The two agreed solely because both defaulted to
the same number, which made the port unconfigurable in practice: on any other
port the session connected and every `core.invoke` a spec made failed as
`TypeError: fetch failed`. Round 6's diagnosis had it backwards -- the port was
not incidental, it was load-bearing and silently pinned.

With both names exported, the port could finally move, which mattered because
**4445 was shared by every layer in a run and every checkout on the machine**. A
layer could be told to bind a port the previous layer's driver had not released:
`Embedded WebDriver server did not become ready on port 4445 within 120000ms`,
one whole layer lost to an address. Each run now reserves a free port.

Each run also gets an isolated OS home, passed as `VANEHUB_DESKTOP_*` and mapped
onto `HOME`/`USERPROFILE`/`APPDATA`/`LOCALAPPDATA` for the application alone --
not for this process, which would repoint the npm and node running the harness.

With those three changes all six layers pass on Windows in one run, exit 0.

**A third defect surfaced only in CI.** `prepareCliFixture` shelled out to
`rustc` on every call, though the stub it produces is committed. Three layers
call it, and by the second one an earlier layer had just executed that stub;
Windows had not released the handle, so the link failed with
`LNK1104: cannot open file` and took the layer down at config load, before a
single spec ran. It now rebuilds only when the source is newer. That fix turned
`cli-terminal`, `session-workspace` and `dialogs` green on the Windows runner.

### Getting the matrix to run at all

`workflow_dispatch` inputs are validated against the **default branch's** copy of
a workflow. The `desktop_full_suite` input exists only on this branch, so the job
could never have been dispatched with it -- which is why, after a round of being
described as "available", it had never once executed. The job now also triggers
on a `desktop-full-suite` pull-request label, which is a trigger a branch can
actually use. Note that `pull_request` does not include `labeled` by default, so
the label takes effect on the next push rather than when it is applied.

### Three-platform result

PR **#218**, head **`5187cf06`**, base `main` at `42b6a649`. CI checks out the
pull-request *merge* ref, so the tree tested is `12e73a60` (macOS job), not the
head commit itself. Workflow run **32733693411**.

| Layer | Windows x64 | macOS ARM64 | Linux x64 |
| --- | --- | --- | --- |
| `desktop-cli-terminal` | PASSED | PASSED | PASSED |
| `desktop-cli-management` | PASSED | PASSED | PASSED |
| `desktop-session-workspace` | PASSED | PASSED | PASSED |
| `desktop-dialogs` | PASSED | PASSED | PASSED |
| `desktop-settings-persistence` | PASSED | PASSED | PASSED |
| `desktop-smoke` | FAILED 27/3 | FAILED 24/6 | FAILED 25/5 |

Five of six layers pass on all three platforms, including the one that covers
this change. Preceding steps in every job -- `npm run desktop:unit:test`,
`cargo check --workspace`, `cargo test --workspace tooling::cli` -- passed on all
three.

### Why `desktop-smoke` cannot pass on a hosted runner

Every failure across the three platforms has one cause:

```
Error: agent is unavailable: Command 'codex' was not found on PATH.
AssertionError: the session tab sweep needs one installed CLI Agent
AssertionError: no session survived in the native database
```

The broad spec sweep needs a real CLI Agent installed on the machine. Hosted
runners have none, and installing one would mean a real vendor download and a
real credential, which this change's own rules forbid. `wdio.conf.mjs` says as
much in its own comment: the sweep "remains opt-in while its host-dependent cases
are promoted into the gate individually". Locally these specs pass because the
developer's machine has `claude`, `opencode`, `gemini` and `agy` on PATH.

This is an environment block, not a defect in this change, and it is **not**
being resolved by editing the specs. They assert rather than recording a BLOCKED
reason the way `native-flows` does; making them self-block would turn a real
prerequisite into a green tick, which is exactly the move this change's task list
forbids. Promoting those cases into the gate is its own piece of work.

An earlier attempt in this round also ran `test:desktop:build` and
`test:desktop:cli-management` before the full suite. That built the desktop app
twice per job and left macOS and Windows failing on persistence -- no session
surviving in the database, a workspace still on its bootstrap shell -- so the job
now makes the single `npm run test:desktop` call, which already builds once and
runs all six layers including CLI management.

### Documentation screenshots

The guide still showed the flat lifecycle page, so `docs:screenshots:check`
failed in CI. Regenerating rewrote 37 files; 35 of them differed by roughly fifty
bytes at identical 1440x900 dimensions, which is renderer noise inside the
comparison tolerance. Only the two `cli-*` captures are committed, and the check
passes with the other 35 left alone.

### Final status

| Platform | Full `npm run test:desktop` |
| --- | --- |
| Windows | **BLOCKED** -- 5/6 layers PASSED; `desktop-smoke` needs a host CLI Agent |
| macOS | **BLOCKED** -- same |
| Linux | **BLOCKED** -- same |

**14.17 stays unchecked**, and therefore **14.20 stays unchecked**. Tasks remain
at 162/164. Unblocking 14.17 needs the host-dependent smoke specs either given a
provisioned CLI Agent on the runners or converted to the repository's BLOCKED
convention -- a decision about the desktop gate, not about this change.

## Round 8: two gates, and the one spec still standing between them and green

Round 7 ended by naming the choice: provision a real Agent on the runners, or
change what the gate is. Provisioning is what this change's own side-effect rules
forbid, so the gate changed.

### The decision, recorded before the code

`specs/desktop-runtime-verification/spec.md` in this change now defines two gates
as requirements rather than as a convention:

- **Required Hermetic Desktop Gate** -- every pull request, all three native
  runners, temporary HOME/PATH/user-data/SQLite, fixture CLI and package manager,
  no real provider, credential, vendor network, or user state. Any failing
  required spec fails the gate.
- **External Provider Desktop Suite** -- real CLI, login, and model response;
  manual, scheduled, or protected-label trigger only; never a required check;
  `BLOCKED` when its prerequisites are absent, and `BLOCKED` is not `PASSED`.

A third requirement makes the split enforceable rather than aspirational: every
spec carries exactly one classification, and the desktop unit tests fail on an
unclassified spec, a manifest entry with no file, a required spec declaring an
external prerequisite, an external spec reaching the required command, or a
replaced spec naming a replacement that does not exist.

### Classification

`tests/desktop/spec-manifest.mjs` is the source of truth. 31 entries, no
unclassified spec.

| Gate | Count | Specs |
| --- | --- | --- |
| `required-fixture` | 29 | every `domain-*`, `ui-*`, `feature-sweep`, `screen-sweep`, `sessions`, `smoke` |
| `external-provider` | 1 | `native-flows` -- real npm global install, real host Python environment, real SSH |
| `duplicate-replaced` | 2 | `ui-cli-management`, `domain-cli-install`, both replaced by `specs-cli-management/cli-lifecycle` |

`native-flows` is the only genuinely external file: every case in it already
refused to run without an explicit opt-in variable, which is the shape of an
external-suite spec rather than a gate spec. Everything else verifies this
application's behaviour -- process lifecycle, session creation, tabs, operations,
cancellation, persistence, PATH resolution, the Agent Runtime boundary -- which a
fixture can stand up, so per the rule above none of it was moved out.

### Fixture Agents, reusing what exists

The gate places a fixture executable for all five managed Agent names --
`claude`, `codex`, `gemini`, `opencode`, `agy` -- ahead of the inherited PATH.
It is the same stub and the same protocol the CLI-terminal layer already drives,
copied under each name rather than recompiled, so there is no second fixture
framework. Three defects in it surfaced once it had to serve probes as well as an
interactive session:

- The POSIX stub never handled `--version` at all.
- Neither stub exited for a non-interactive invocation, so a readiness probe such
  as `auth list` blocked on stdin until its budget expired.
- The fixture directory was created per config load, and wdio evaluates the
  config in every worker -- so each spec file in one layer got a different
  directory, a different PATH, and a different environment fingerprint.

### Command semantics

| Command | Meaning |
| --- | --- |
| `npm run test:desktop` | the required hermetic gate: build plus all six layers |
| `npm run test:desktop:core-smoke` | launch, IPC and shutdown contract alone |
| `npm run test:desktop:external-provider` | the external suite; `BLOCKED` without prerequisites |
| `npm run test:desktop:all` | both, with the external result appended and never folded into the required verdict |

`desktop_full_suite` runs the required gate only. A separate non-gating
`desktop-external-provider` job runs the external suite on dispatch, schedule, or
a protected label.

### Result: Windows green, the other two short by a credential store

The gate no longer needs a real Agent anywhere. Windows CI now passes it
completely -- run `32752686297`, and again on `32758070852`:

| Layer | Windows x64 | macOS ARM64 | Linux x64 |
| --- | --- | --- | --- |
| `desktop-smoke` (29 required specs) | **PASSED 29/0** | FAILED 27/2 | FAILED 27/2 |
| `desktop-cli-terminal` | PASSED | PASSED | PASSED |
| `desktop-cli-management` | PASSED | PASSED | PASSED |
| `desktop-session-workspace` | PASSED | PASSED | PASSED |
| `desktop-dialogs` | PASSED | PASSED | PASSED |
| `desktop-settings-persistence` | PASSED | PASSED | PASSED |
| **Required gate** | **PASSED** | FAILED | FAILED |

`Desktop External Provider` was correctly skipped: it is not a required check and
its label was not set. Run locally it prints `BLOCKED`, names all four missing
prerequisites, writes evidence, exits 0, and does not build the application.

Every remaining macOS and Linux failure is one thing -- the OS credential store:

```
macOS: Platform failure: A default keychain could not be found.
Linux: No default store has been set, so cannot search or create entries
```

Provisioning an empty keychain and an empty gnome-keyring on the runners moved it
but did not fix it, and the reason is this change's own doing: round 7 gave the
application an isolated OS `HOME`, so it looks for the keychain under the run's
temporary home, where there is none. A store provisioned in the runner's real
home is somewhere the application under test can no longer see.

So the fix is not more runner provisioning. It is either a credential store
created *inside* the isolated home, or a credential backend the harness can point
at -- and choosing between those is a decision about the desktop gate's
isolation model. Windows is unaffected because Credential Manager is per-user and
not resolved through `HOME`.

Not done: relaxing home isolation to make the failures disappear, or moving the
affected specs to the external suite. Neither is honest -- they verify local
credential storage, not a provider, so by this change's own classification rule
they are `required-fixture`.

### Local Windows note

Locally, `domain-loop` fails inside the layer while passing alone against the
same fixture PATH; on Windows CI it passes both ways. The developer machine has
real Agents installed and CI does not, which is the difference the gate exists to
remove. The CI result is the one that counts.

**14.17 and 14.20 stay unchecked. Tasks remain 162/164.** *(Superseded in round 10:
the matrix is green on all three platforms.)*

## Round 9: the credential store, and what is left

### The gate was reaching the real OS store

Round 8 ended blaming the runners for having no keychain. That was half the
story, and the less important half. On Windows the gate *passed* by writing
connector tokens and provider configuration into the developer's own Credential
Manager, where they outlived the run -- a side effect this change's own rules
forbid, hiding inside a green result. Provisioning empty stores on the macOS and
Linux runners moved their errors without fixing them, because the application
runs with an isolated OS `HOME` and so looked for a store under the run's
temporary home rather than the one the runner had prepared.

`OsCredentialStore` now has two backends behind one `cfg` each: the real OS store
in a production build, and a JSON file inside `VANEHUB_APP_DATA_DIR` in a desktop
test build. That directory is created per run, validated against the real
application data directory, and deleted afterwards, so the file survives the
relaunches the persistence layers depend on and nothing in it outlives the run.
One file, no new dependency, at the seam every credential already crossed. A
boundary test holds the split: the production backend must still use the OS
store, and the test backend must not mention `keyring` at all.

### Merged with main again

`origin/main` reached `2ede7d27`, and the pull request had gone `CONFLICTING` --
which is also why no CI ran for two pushes: GitHub cannot build the merge ref for
a conflicting pull request, so `pull_request` events cannot fire at all. Merged,
not rebased. `#221` had independently added two opt-in real-CLI desktop layers;
they are external-provider layers in the sense this change now defines, and both
sides' additions were kept. Migrations re-scanned: still 82/83/84, dense, no
duplicates.

One self-inflicted mistake worth recording: the credential commit also staged
`src-tauri/gen/schemas/*`, 168 lines of wdio automation ACL that the desktop test
build injects. Reverted in the following commit. The rule that these are never
committed only works if it is checked before every commit, not remembered.

### Three-platform result, run 32797174954, HEAD `600508bc`

| Layer | Windows x64 | macOS ARM64 | Linux x64 |
| --- | --- | --- | --- |
| `desktop-smoke` (29 required specs) | **PASSED 29/0** | FAILED 27/2 | FAILED 28/1 |
| `desktop-cli-terminal` | PASSED | PASSED | PASSED |
| `desktop-cli-management` | PASSED | PASSED | PASSED |
| `desktop-session-workspace` | PASSED | PASSED | PASSED |
| `desktop-dialogs` | PASSED | PASSED | PASSED |
| `desktop-settings-persistence` | PASSED | PASSED | PASSED |
| **Required gate** | **PASSED** | FAILED | FAILED |

Every credential-store failure is gone. What remains is three specs in contexts
this change does not touch:

- Linux: `sessions` -- `validation error: Session participants changed since they
  were loaded`, an optimistic-concurrency conflict in the multi-Agent roster.
- macOS: `screen-sweep` rendering session workspace tabs, and `ui-chat` slash
  completions.

They are platform-specific behaviours in the session and chat contexts, surfaced
for the first time because the sweep had never run to completion on those
platforms before. Diagnosing them is work in those contexts, not in this one.

**14.17 and 14.20 stay unchecked. Tasks remain 162/164.**

## Round 10: the matrix is green on all three platforms

### The three remaining specs were three separate defects, none platform-specific

**`screen-sweep` and `ui-chat`** pushed a route and trusted it. The application
resumes the destination it was last on, and that restore reads persisted state
over IPC, so it can land *after* a push that already waited for
`data-vanehub-bootstrap === "ready"` and silently replace it. On Windows the
restore happened to land early and nothing was noticed; on the slower macOS
runner it landed late, leaving the sweep on the settings page it had just
finished capturing while it waited thirty seconds for a session tab bar that was
never going to appear. A shared helper now repeats the push until the location
stays put -- and accepts a deeper path, because the workspace canonicalises
`/workspace/sessions` to `/workspace/sessions/<active id>` by design. The first
version of that helper did not, and reported "something kept navigating away"
about the application doing exactly what it should.

**`sessions`** reused the `updatedAt` that `create_session` returned as the
expected version for `update_session_seats`. Creating a session starts work that
touches the row again, so that value can already be a revision behind, and the
optimistic-concurrency guard rejected the write. The guard was right; the spec
was reading a stale version, and now re-reads immediately before writing. It only
surfaced now because fixture Agents made two Agents "available" on a hosted
runner for the first time -- before that the case skipped itself for want of a
second CLI.

### Result: every layer, every platform

Run **32810072826**, product commit **`368ce0e2`**, CI merge ref `b4fd6124`.

| Layer | Windows x64 | macOS ARM64 | Linux x64 |
| --- | --- | --- | --- |
| `desktop-smoke` (30 required specs) | **PASSED 30/0** | **PASSED 30/0** | **PASSED 30/0** |
| `desktop-cli-terminal` | PASSED | PASSED | PASSED |
| `desktop-cli-management` | PASSED | PASSED | PASSED |
| `desktop-session-workspace` | PASSED | PASSED | PASSED |
| `desktop-dialogs` | PASSED | PASSED | PASSED |
| `desktop-settings-persistence` | PASSED | PASSED | PASSED |
| `desktop-agent-mcp` | PASSED | PASSED | PASSED |
| **Required gate** | **PASSED** | **PASSED** | **PASSED** |

All three ran the same commit. `Desktop External Provider` was skipped, as a
non-gating job with no label should be, and its `BLOCKED` result is not counted
anywhere as a pass.

### Merging while main moved five times

`origin/main` landed five pull requests during this round -- roughly one every
forty minutes, against a merge-and-verify cycle of sixty to ninety. Twice the
pull request went `CONFLICTING` before CI could start, and that is worth
recording precisely: **GitHub cannot build a merge ref for a conflicting pull
request, so `pull_request` events do not fire at all.** Two pushes produced no CI
run of any kind, which reads exactly like a broken workflow.

Each merge was a merge, never a rebase. Two of them were substantive:

- **`#209` took migration 82** for `local-media-profiles`, colliding with this
  change's three tables. They moved to **83, 84, 85** -- the next three
  consecutive numbers, not a higher range, because
  `assert_migration_history_is_dense` refuses a gapped history at startup. Both
  of main's own renumber-collision tests then had to be reconciled with a third
  branch in the history: each rewinds a database to what the other branch's build
  leaves behind, and each had to clear everything above 81 and drop this branch's
  tables too, or the rewind produces a gap rather than a real earlier state.
  Their totals are now derived from the migration list instead of named.
- **`#222` added `desktop-agent-mcp`** to the full-suite layer list. It is a
  required hermetic layer by this change's own classification, so it joined the
  required list rather than sitting beside it.

Two subtree budgets were re-measured on the merged tree rather than picked:
`src/services` at 20243 and `platform/database` at 3245. Neither side's figure
described the merge, and they are not additive -- they were taken against
different bases.

### Two mistakes made and corrected in this round

- A commit staged `src-tauri/gen/schemas/*` -- 168 lines of wdio automation ACL
  that the desktop test build injects. Reverted in the next commit. Main has
  since made this structurally impossible with `withDirectoryRestored`.
- The regenerated documentation screenshots were discarded after judging them
  "noise" by byte size. That was the wrong measure: `#209` added a settings entry
  that lengthens the left navigation and shifts every capture below it, which is
  a handful of bytes and a real visual change. CI's diff showed it in the
  sidebar. All forty-eight are regenerated.

### 14.20: the completion conditions, each checked rather than assumed

| Condition | Evidence |
| --- | --- |
| Windows / macOS / Linux full desktop PASSED | run 32810072826 on `368ce0e2`, re-confirmed by run 32814119050 on `c11ce015` |
| All three on one commit | `commit-under-test.txt` in each platform's artifact |
| Side-effect guard PASSED | asserted inside the passing `cli-lifecycle` spec: no real npm, WinGet, vendor URL, credential store, or user database |
| Every desktop spec classified | `spec-manifest.mjs`, enforced by eight tests; an unclassified spec fails the desktop unit tests |
| No required spec skipped | `desktop-smoke` reports 30 passed / 0 failed on each platform |
| Migrations agree with `origin/main` | main's max is 82; this change owns 83, 84, 85, consecutive and dense |
| Runtime resolver is the only production path | `architecture.rs`, 54 tests |
| Legacy table: one reader, zero writers | `the_legacy_cli_table_has_exactly_one_reader_and_no_writer` |
| Old lifecycle APIs gone | no caller of `list_cli_tools`, `refresh_cli_detections`, `install_cli_version`, `upgrade_all_cli_versions`, `CliToolStatus`, or `LifecycleEligibility` outside one historical comment |
| Whole CI run green | run 32814119050: every job success, `Desktop External Provider` correctly skipped |

The product commit the matrix ran against is `368ce0e2`; `c11ce015` re-ran it with
regenerated documentation images and no code change; this evidence commit adds
only `tasks.md` and these notes.

**Tasks: 164/164. Not archived.**
