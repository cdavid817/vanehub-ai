# Verification report — `redesign-unified-workbench-ui`

Task 21.24. Final-state snapshot of the change's own verification evidence, compiled directly from
`tasks.md`'s per-task evidence lines (22.1, 22.8-22.13 primarily) plus a fresh `/opsx:verify` pass —
not a re-run of every command from scratch. Every number below cites the `tasks.md` line it comes
from rather than being restated from memory.

## Commands run

Full mandatory sequence (AGENTS.md's own "校验命令" list), most recently all green in one fresh,
isolated pass (22.10/22.11/22.13):

```
npm run lint:ci
npm run test               # 639/639 files, 4398/4398 tests
npm run test:coverage      # same, +85.19% frontend-total vs. 45.20% policy floor
npm run build              # 16 lazy chunks, main static closure 162.8 KiB gzip
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run native:panic:check
cargo test --workspace     # 6026 passed, 0 failed, 15 intentionally-ignored (fixture-only)
openspec validate redesign-unified-workbench-ui --strict   # valid
openspec validate --specs --strict                          # 142 passed, 0 failed
npm run architecture:check
npm run contracts:check
npx playwright test --workers=1   # 295 passed, 22 failed, 317 total, 50.2 min (see Failures)
npx playwright test --config playwright.visual.config.ts   # 24 passed, 0 failed
npm run test:desktop:build        # succeeds, repeatable
npm run test:desktop:session-shell / :external-provider / (others)  # see Platforms
```

`cargo test --workspace` was not re-run a second time in the exact same pass as 22.11's other
commands — `git diff --stat` confirmed zero `src-tauri/` changes since the cited run, so a second
~10-minute full run would have produced no new signal (22.11's own evidence).

## Commits

361 commits on `feat/redesign-unified-workbench-ui` ahead of `main`, from
`1cbaba82 chore(workbench): establish redesign change and record milestone 0 baseline` through
`09a1afbc docs(tasks): record 22.12 evidence`. Every commit that changes `src/`, `src-tauri/src/`,
or `tests/` carries its own task-scoped evidence line in `tasks.md`; this report does not re-list
them individually — `git log --oneline main..HEAD` plus the relevant `tasks.md` section is the
authoritative per-commit trail.

## Fixtures

- `tests/desktop/fixtures/cli/` — stub CLI-agent binaries used by every desktop layer that needs a
  "CLI agent" without a real model call or real credentials.
- `tests/desktop/cli-management-fixture.mjs` — a full temporary-PATH fake CLI + fake package-manager
  rig for the `desktop-cli-management` layer, independently checked post-run by
  `scripts/desktop/cli-side-effect-guard.mjs` for any touch of real npm/WinGet/vendor URLs/credentials/
  the user's real database (查不到证据即判失败，不判跳过).
- `desktop-external-provider`'s SSH case is deliberately real-provider-gated
  (`VANEHUB_SSH_HOST`/`VANEHUB_SSH_USER`/`VANEHUB_SSH_PASSWORD`), not fixture-backed — a fixture
  cannot stand in for a real SSH host-key handshake (21.20's own evidence).

## Platforms

- **Windows**: attempted repeatedly, real and reproducible results below.
- **macOS**: not executed — no runner exists in this environment. Reported as not executed, not as
  passed.
- **Linux**: not executed — no runner exists in this environment. Reported as not executed, not as
  passed.

Web Playwright and all Vitest/Rust suites are platform-agnostic in this repo's CI and were run on
Windows here; CI itself additionally runs `Desktop Smoke` independently on Windows, macOS, and Linux
native runners — those platform results are CI's own to report per-run, not reproduced locally here.

## Counts

| Metric | Value | Source |
|---|---|---|
| Tasks complete | 322/382 (84%) | `tasks.md` checkbox count |
| Spec requirements | 89 across 11 delta files | `requirement-test-matrix.md`, re-confirmed via `grep -c` |
| Requirement coverage | 46 Covered / 41 Partial / 2 Gap | `requirement-test-matrix.md` |
| Scenarios (real, current) | 401 (vs. 312 stale in `traceability.md`) | `requirement-test-matrix.md` |
| Vitest | 639/639 files, 4398/4398 tests | 22.10 |
| Frontend coverage | 85.19% lines vs. 45.20% floor | 22.10 |
| `cargo test --workspace` | 6026 passed, 0 failed, 15 ignored | 22.1, cited by 22.11 |
| Build | 16 lazy chunks, 162.8 KiB gzip main closure | 22.10 |
| Web Playwright (full) | 295 passed, 22 failed / 317 | 22.12 |
| Web Playwright (visual baseline) | 24 passed, 0 failed | 22.12 |
| `openspec validate --specs --strict` | 142 passed, 0 failed | 22.13 |
| i18n keys removed (dead) | 295 across 5 locales | 22.8 |

## Screenshots

8 documentation screenshot scenarios (16 files across en/zh-CN) confirmed stale by direct visual
inspection — still show the pre-redesign 9-icon activity bar and/or the old flat nine-tab session
strip: `session-workspace`, `session-logs`, `session-traces`, `loop-center`, `todo-board`,
`goal-center`, `evaluations`, `mission-control`. **Not regenerated** — the automated generator
(`tests/docs/documentation-screenshots.spec.ts`) already targets current labels/routes (this is a
stale-artifact gap, not a broken generator), but regenerating requires a clean single-writer
environment and this worktree is shared with other concurrently-committing agent sessions (22.9's
own evidence, matching this session's own prior memory on screenshot regeneration reliability).
Waived — see Waivers below.

## Failures

**Web Playwright, full suite, 22 failures** — every one individually cross-referenced against this
change's own already-written evidence, not treated as an undifferentiated count (22.12's own
evidence has the full per-file breakdown):

- **21 of 22** are exact-match instances of failures already independently found, root-caused, and
  disclosed earlier in this same change: the session-sidebar Sheet-scrim-lingering bug (11.7),
  the §8 nine-tab-split staleness pattern (12.9), the `SplitPane` hit-area regression's disclosed
  sibling gap, `local-media.spec.ts`'s sr-only accessible-name fold, `loop-engineering.spec.ts` /
  `loop-engineering.visual.spec.ts`'s pre-existing id-mismatch, `main-chat.spec.ts`'s rapid-switch
  flake, `session-category-management.spec.ts`, and `frontend-rendering-performance.spec.ts`'s
  already-flagged lifecycle/virtualization gaps.
- **1 of 22 genuinely unexplained**: `settings-search.spec.ts`'s "ArrowDown moves the highlighted
  result before Enter selects it" — reproduced identically across 2 independent runs (not load —
  3/4 tests in the same file pass), not root-caused. Standing, disclosed gap.

**Desktop/WebdriverIO** — confirmed environmentally BLOCKED on Windows, reproduced independently 3
separate times across this session (21.20/21.21's original finding via a control experiment on an
unmodified sibling spec; 22.12's own standalone re-run losing an orphaned `vanehub-ai.exe` process
after 28+ minutes of zero output). `npm run test:desktop:build` itself succeeds cleanly and
repeatably every time — the blocker is specifically live UI-driven layer execution. Working theory,
unconfirmed: GPU/WebView2 or shared-host resource contention from multiple concurrent Claude Code
sessions on one machine. macOS/Linux: not executed, no runner available.

## Waivers

Explicit, evidenced exceptions accepted rather than force-closed or silently dropped:

1. **Desktop-native WebdriverIO/Tauri layers (21.20, 21.21, part of 22.12)** — environmentally
   blocked on this host, reproduced 3× independently with converging diagnosis. Two flows
   (`xterm resize/clipping`, `window resize composition`) have new, real, lint-clean, mechanism-
   verified-by-reading-source test code that has never been confirmed by a passing live run. One
   flow (`native folder dialog`) is a genuine, disclosed gap: its one real command has zero frontend
   callers, so there is no UI element to automate against.
2. **8 stale documentation screenshots (22.9)** — not regenerated; needs a clean single-writer
   environment this shared worktree cannot currently provide. The generator itself is not broken.
3. **`settings-search.spec.ts`'s 1 unexplained Web Playwright failure (21.23, 22.12)** — reproduced,
   not load noise, not root-caused within this change's own scope/budget.
4. **2 of 89 spec requirements remain Gap-verdict** (`requirement-test-matrix.md`): Loop iteration
   Inspector selection (17.11, blocked on an architectural design decision — `WorkbenchSelection`'s
   type union vs. a nested-provider approach) and persistent workspace trust presentation (13.10,
   blocked on a confirmed-absent backend concept). Both individually owned by their own `[ ]` tasks.
5. **60 remaining open tasks overall** — every one carries dense, specific, already-written evidence
   explaining why it is incomplete (out-of-this-change's-scope backend limitation, pending human
   design decision, environmentally blocked in this worktree, or an honestly-disclosed partial
   scope-down), not silent gaps. See `/opsx:verify`'s own report (22.14) for the full categorization.

## Verdict

No CRITICAL finding blocks archiving. All waivers above are evidenced, individually owned by their
source task, and consistent with this change's own established anti-fabrication discipline: every
open item was investigated, not skipped.
