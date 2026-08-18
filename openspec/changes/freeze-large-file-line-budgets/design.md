## Context

See proposal.md — Why for the motivation and the measured drift.

Three facts about the existing code shape this design:

- The repository's architecture fitness gate is `npm run architecture:check` (`package.json:38`), which CI runs at `.github/workflows/ci.yml:234`. It fans out to three enforcement layers: a JavaScript rule engine for frontend rules (`scripts/architecture/`), ESLint plus `tsc --noEmit`, and `cargo test --test architecture` for native rules.
- The JavaScript engine already owns frontend traversal. `productionFiles()` at `scripts/architecture/frontend-rules.mjs:81-89` walks a root recursively and excludes `.test.`, `.spec.`, and `.d.ts` files — exactly the walk a frontend subtree budget needs — and `architectureDiagnostic()` at `scripts/architecture/rules.mjs:20-22` already formats `[RULE-ID] file:line: message Repair: ...`.
- The project registers a PostToolUse hook (`scripts/hooks/post-edit-quality.mjs`) that runs `eslint --fix` after every `.ts`/`.tsx` write. Anything expressed as an ESLint rule fails inside the author's edit loop; anything expressed elsewhere fails only when the author runs the fuller gate.

## Goals / Non-Goals

**Goals:**

- Record a measured baseline for every path that is currently exempt from the 300-line rule, before the three decomposition lanes branch from `main`.
- Keep the gate valid when a registered file becomes a directory module.
- Fail in the fastest loop that can express each check.

**Non-Goals:**

- Reducing any file. This change records and enforces; the decomposition changes reduce.
- Budgeting the whole frontend tree. Only subtrees containing a registered path get a subtree budget; a blanket `src/**` budget would block ordinary feature work, which is not this gate's job.
- Replacing the global 300-line rule, the ESLint `max-lines` rule itself, or any existing architecture rule.

## Decisions

### Each budget lives in the layer that already owns that traversal

Three homes, all reachable from the one `npm run architecture:check` entry point:

| Budget | Home | Why there |
|---|---|---|
| Frontend per-file | `eslint.config.js` | The PostToolUse hook runs ESLint on every `.ts`/`.tsx` write, so the budget fails in the author's edit loop |
| Frontend subtree | `scripts/architecture/` | `productionFiles()` already implements the exact recursive walk with `.test.`/`.spec.`/`.d.ts` exclusion |
| Native per-file and subtree | `src-tauri/tests/architecture.rs` | `rust_files()` and the `[ARCH-NATIVE-*]` diagnostic convention already live there |

ESLint cannot express a directory aggregate, which is why the frontend needs two of the three.

*Alternative rejected — a separate Vitest check or a `wc -l` npm script* (what the optimization ticket proposed): adds a fourth gate and new CI wiring for work the existing layers already do.

*Alternative rejected — put the frontend subtree budget in `architecture.rs`*: it would reimplement `productionFiles()`' test-and-`.d.ts` exclusion in a second language, and the two copies would drift the first time the exclusion rule changes. The repository already splits frontend rules into JavaScript and native rules into Rust; a frontend line budget belongs on the frontend side of that split.

**Duplication guard:** no number is recorded in two places. ESLint records only per-file budgets, `scripts/architecture/` only frontend subtree budgets, `architecture.rs` only native budgets.

### Budgets are exact measured baselines, with no headroom

A tolerance band is a silent allowance to grow, which is the failure this change exists to stop. When a decomposition lane needs a handful more lines for `mod` declarations and imports, it raises the number explicitly with the measured delta — the reviewed-edit path the spec requires. This also makes the useful negative signal visible: if a "pure move" grows a subtree by more than module boilerplate, it was not a pure move.

### A missing registered path is satisfied, not a failure

Converting `migrations.rs` into `migrations/` must not fail the gate for the refactor itself. The subtree budget bounds whatever replaced the file, so treating the absent path as satisfied does not open a rename hole.

### Physical lines, matching the existing rule

Counting is physical lines with no blank- or comment-skipping, matching the repository's existing `"max-lines": ["error", { max: 300, skipBlankLines: false, skipComments: false }]`.

Subtree counters count newline-terminated lines, so they agree with `wc -l`: Rust uses `str::lines().count()`, JavaScript uses `source.split("\n").length` minus one when the source ends in a newline. ESLint's own `max-lines` counting can differ by one on a file with no trailing newline, so each per-file budget is set from the tool that enforces it rather than assumed equal across tools.

### Baseline

Native per-file budgets (`architecture.rs`):

| Path | Budget |
|---|---:|
| `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs` | 13,927 |
| `src-tauri/src/contexts/sessions/infrastructure/tests.rs` | 5,110 |
| `src-tauri/src/contexts/agent_runtime/application/tests.rs` | 4,628 |
| `src-tauri/src/contexts/tooling/skills/application/tests.rs` | 4,049 |
| `src-tauri/src/platform/database/migrations.rs` | 2,301 |

Native subtree budgets (`architecture.rs`), registered only where a decomposition lane will convert a file into a directory module:

| Subtree | Budget | Counts |
|---|---:|---|
| `src-tauri/src/contexts/agent_runtime/infrastructure/` | 58,116 | `*.rs` |
| `src-tauri/src/platform/database/` | 2,914 | `*.rs` |

Frontend subtree budget (`scripts/architecture/`), measured with the existing `productionFiles()` walk (143 files):

| Subtree | Budget | Counts |
|---|---:|---|
| `src/services/` | 18,149 | `*.ts`, `*.tsx` excluding `.test.`, `.spec.`, `.d.ts` |

Frontend per-file budgets (`eslint.config.js`), replacing `"max-lines": "off"`:

| Path | Budget |
|---|---:|
| `src/services/web-agent-client.ts` | 6,013 |
| `src/services/tauri-agent-client.ts` | 1,213 |
| `src/types/agent.ts` | 702 |
| `src/services/agent-service.ts` | 665 |
| `src/main-layout/main-layout.tsx` | 528 |
| `src/contracts/agent.ts` | 504 |
| `src/settings/pages/sdk-page.tsx` | 396 |
| `src/main-layout/create-session-dialog.tsx` | 318 |

`src/services/coordination-runtime.ts` is dropped — the file no longer exists.

## Risks / Trade-offs

- **A pure test-extraction move adds `mod` and `use` boilerplate, tripping the subtree budget by a few lines** → Expected and accepted. The lane raises the subtree budget by the measured delta with a one-line reason. Small, visible friction is the intended behavior; a tolerance band would hide the same signal permanently.
- **The three lanes branch from `main` and all three will edit budgets** → They edit disjoint rows (`agent_runtime/infrastructure`, `platform/database`, `src/services`), so the merges do not overlap in content, only in file. This change must land before the lanes branch so they share one baseline.
- **Budgets could be quietly raised instead of the code being reduced** → Not preventable by tooling, and out of scope for it. The gate makes the raise appear as a reviewed diff line, which is the enforceable part.
- **A subtree budget can be satisfied by moving code out of the subtree** → Accepted. That is exactly what the planned relocation of heavyweight inline tests to `src-tauri/tests/` does, and the existing layering rules in `architecture.rs` already constrain where code may legally live.
- **Recording a budget for a file today makes it look sanctioned** → Mitigated by the diagnostic requirement: a failure names the decomposition work that owns the path, so the budget reads as a debt marker rather than an allowance.

## Migration Plan

Land this change on `main` before the `api_process_adapter.rs`, `web-agent-client.ts`, and `migrations.rs` decomposition changes branch, so all three measure against one recorded baseline. No runtime deployment step and no rollback concern — reverting the commit removes the gate and restores the previous exemption behavior.
