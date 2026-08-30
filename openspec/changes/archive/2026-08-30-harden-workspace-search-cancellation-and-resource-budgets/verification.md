# Verification

Run on Windows 11, on the host that holds this worktree. **Nothing below is inferred from another
platform**; where a platform was not exercised it says so rather than being left out.

## Command set from `AGENTS.md`

| Command | Result | Notes |
| --- | --- | --- |
| `npm run lint:ci` | PASSED | |
| `npm run test` | PASSED with one environmental failure | 2 810 of 2 811 passed; see below |
| `npm run build` | PASSED | 16 lazy chunks; main static closure 156.8 KiB gzip |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED | |
| `cargo check --workspace` | PASSED | |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSED | |
| `npm run native:panic:check` | PASSED | |
| `cargo test --workspace` | PASSED | 6 205 lib tests, plus every integration target |
| `openspec validate --specs --strict` | PASSED | 142 items |
| `npm run architecture:check` | PASSED | 63 + 9 + 11 + 8 |
| `npx playwright test` | PASSED with one pre-existing failure | 212 of 213; see below |

### The two failures, and why neither is this change

**One test in `src/settings`, and not the same one twice.** The first full run failed
`settings-pages.test.ts › keeps the CLI management route lazy`; a later run failed
`onepiece-context-health-section.test.tsx › shows aggregate coverage` instead, on a `findByText`
in a file that takes twenty-five seconds to run. Both pass in isolation — 13/13 and 8/8 — and the
second passed three consecutive times on this branch after failing once. `src/settings` has no diff
against the merge base: `git diff bece2e3c..HEAD -- src/settings` is empty. These are load-sensitive
async assertions, recorded here rather than silenced; the machine was building a desktop client and
running cargo suites at the same time.

**`multi-agent-session.spec.ts › participant mentions`** — a scroll-anchoring assertion,
`|scrollHeight − scrollTop − clientHeight| < 4`, receiving 58. Checked against the merge base rather
than assumed: `git checkout bece2e3c -- src` and re-running the single spec reproduces it with the
identical value. It is a pre-existing failure on `main`, not a regression, and it is outside this
change's scope to fix.

## Focused suites

| Area | Count | Where |
| --- | --- | --- |
| Workspaces context (lib) | 492 | `cargo test --lib contexts::workspaces` |
| Quick Open | 26 | `infrastructure::path_search_tests` |
| Content search | 30 | `infrastructure::content_search_tests` |
| Remote helper protocol and transport | 45 | `infrastructure::remote_helper` |
| Local/remote provider parity | 30 | `infrastructure::provider_contract_tests` |
| Structural performance gates | 5 | `infrastructure::structural_performance_tests` |
| Real-interpreter helper | 15 | `tests/remote_workspace_ssh.rs` |
| Frontend services and workspace panels | 1 583 | `src/services`, `src/session-workspace` |
| Workspace search in a browser | 3 | `tests/e2e/session-workspace-search.spec.ts` |

## Per-platform results

The rule is that a result belongs to the platform that produced it. This host is Windows.

| Behaviour | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Local filesystem walk, budgets, ignore policy | PASSED | NOT RUN | NOT RUN |
| Directory cursor v2, invalid and stale refusals | PASSED | NOT RUN | NOT RUN |
| Cancellation and supersession | PASSED | NOT RUN | NOT RUN |
| Remote provider against fakes | PASSED | NOT RUN | NOT RUN |
| Remote helper under a real Python interpreter | PASSED | NOT RUN | NOT RUN |
| Remote helper against a real SSH host | NOT RUN | NOT RUN | NOT RUN |
| Desktop `session-workspace` layer | PASSED | NOT RUN | NOT RUN |
| Desktop `smoke` layer | FAILED — see below | NOT RUN | NOT RUN |

The unreadable-entry cases run on both platform families rather than one: `chmod 000` on Unix, an
exclusive directory or file handle on Windows. Only the Windows half has been executed here, and the
Unix half is claimed by CI rather than by this run.

### Desktop layers

`npm run test:desktop:session-workspace` — **PASSED**, four specs, including a new one that builds a
twelve-level tree on the real filesystem, searches it through the native adapter, and requires the
incomplete-coverage notice. Depth is the one native ceiling a fixture can reach without writing
hundreds of thousands of entries, and the notice it produces is the same one every other budget
produces.

`npm run test:desktop:smoke` — **FAILED**, 24 of 25 specs passed. The failure is
`domain-observability.e2e.mjs › guards the IM session binding surface when nothing is paired`,
asserting that a session with no pairing reports no binding. This branch changes nothing under
`contexts/communications` or the IM surfaces — `git diff bece2e3c..HEAD -- src-tauri/src/contexts/communications`
is empty — and the same run logged `database is locked` during teardown, which points at the shared
user database rather than at this code. **No baseline was taken**, so this is recorded as FAILED and
unattributed rather than as a known-good failure.

### What each layer proves about partial, cancel, and busy

| | Partial | Cancel | Busy |
| --- | --- | --- | --- |
| Rust unit and integration | yes | yes, deterministically | yes |
| Frontend component | yes | yes | yes |
| Playwright (Web adapter) | yes | no | yes |
| Desktop (native adapter) | yes | no | no |

Cancellation is deliberately not asserted in either end-to-end layer. Observing a stopped scan from
outside requires the scan to still be running when the key is pressed, which on a fixture workspace
is a race against the search finishing — and a test that sleeps until the odds are good is the kind
of test this change's own rules forbid. Where it *is* proved, it is proved by construction: a clock
that signals the token on a chosen reading places the cancel between two chunks of a file.

Busy is not reachable from the desktop layer either. Admission refuses the third concurrent walk
against one workspace, and nothing a single UI session does starts three.

**No remote host was contacted.** Every remote assertion is either against a scripted channel or
against the helper program running under a local interpreter. What that does *not* cover is SSH
itself: connection loss mid-exchange, a host whose Python differs from this one's, and the actual
wall-clock behaviour of a channel close. Those need the opt-in integration path and a host.

## Chosen defaults, and the evidence for them

Every value below is a `Default` impl. None was raised by this change; the two that changed moved
*down*.

### `WorkspaceInspectionBudgetLimits`

| Profile | Entries | Files | Bytes | Metadata | Candidates | Results | Depth | Deadline |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `content_search` | 20 000 | 2 000 | 512 MiB | 60 000 | 4 096 | 200 | 10 | 20 s |
| `path_search` | 20 000 | 0 | 0 | 60 000 | 256 | 50 | 10 | 10 s |
| `document_discovery` | 20 000 | 0 | 0 | 60 000 | 300 | 300 | 6 | 10 s |
| `directory_listing(limit)` | 200 000 | 0 | 0 | 400 000 | limit + 1 | limit | 0 | 10 s |

`directory_listing` visits one directory and does not descend, which is why its depth is zero and
its entry ceiling is the largest here: a single directory with two hundred thousand entries in it is
a real thing, and paging it is the whole point of that profile.

The entry, depth, result and file numbers restate bounds that already existed —
`SEARCH_SCAN_LIMIT = 20_000`, `SEARCH_DEPTH_LIMIT = 10`, `MAX_CONTENT_MATCHES = 200`,
`DOCUMENT_LIMIT = 300`, `DOCUMENT_DEPTH_LIMIT = 6`. The byte, metadata and deadline ceilings are new and every one of them is
*tighter* than what was there, which was nothing: 2 000 files at the existing 1 MiB per-file bound
is 2 GiB of reads with nothing counting them, and 512 MiB is a quarter of that.

`max_retained_candidates` is the one that changed meaning. It was `MATCH_COLLECTION_LIMIT = 2_000`
retained candidates for ranking; it is now the breadth-first frontier for content search, and for
Quick Open it is narrowed again to `limit + 1` by the page bound before the walk begins. The memory a search costs is now a property of the page rather
than of the repository.

### `WorkspaceInspectionAdmission`

| Limit | Value | What it bounds |
| --- | --- | --- |
| `global_active` | 4 | Inspection walks anywhere in the process |
| `per_workspace_active` | 2 | Inspection walks against one workspace |
| `wait` | 750 ms | How long a request waits before it is refused |

Both ceilings were unbounded before. Four is chosen from the shape of the work rather than from core
count: these are I/O-bound walks on the blocking pool, and a fifth concurrent walk on one disk
finishes no sooner than the first four would have. Two per workspace covers the real overlap — a
directory listing while a content search runs — without letting one project hold the whole global
allowance.

`wait` is finite on purpose. A queue with no deadline is an unbounded queue wearing a different name;
750 ms is long enough to absorb a walk that is about to finish and short enough that a refusal
arrives while the reader is still looking at the box they typed into.

## Residual O(N) scans, and what has no index

**There is no index.** Every search here is a walk, and this change did not add one. What follows
from that:

- **Quick Open visits every eligible entry.** Ranking cannot know which ten entries score highest
  without scoring all of them. What is bounded is what is *kept* — the page plus one — and what is
  charged is every visit. On a workspace larger than the entry budget the answer is honestly partial.
- **Content search opens every eligible file until a budget stops it.** Streaming rather than
  two-pass, so the memory is one file, but the work is still linear in the tree.
- **Document discovery walks the project.** Same shape, its own profile.
- **A directory page enumerates the whole directory before cutting it.** The order is a total order
  and a page is a window onto it, so the entries after the cut still have to be seen to know they
  come after. This is unchanged from before and is why `directory_listing` charges entries.
- **Directory fingerprints are a stat each, never a listing.** The polling path deliberately does not
  enumerate; that is the one place where the cheap answer is the correct one.

**Snapshot limitations.** A cursor names a position in an ordering, and the ordering is recomputed
on every page. Cursor v2 detects that the directory changed — version, workspace identity, order
mode, policy identity, and a directory fingerprint — and refuses rather than paging into a list that
no longer exists. What it cannot do is show a reader the directory as it was: there is no snapshot,
so a stale cursor's only recovery is to start again. That is stated in the UI as a restart rather
than presented as an error.

**Remote coverage is the helper's, not this side's.** The counts a remote answer carries are the
ones the helper reported. Where the helper cannot express a bound this side has — ripgrep bounds a
content search by result count and has no byte budget — the difference is reported as coverage
rather than pretended away, and no local counter is invented to fill the gap.
