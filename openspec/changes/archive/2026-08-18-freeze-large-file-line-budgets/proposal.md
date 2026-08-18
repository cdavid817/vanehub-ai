## Why

The repository's file-size constraint has no ratchet on its largest files, so the worst offenders grow unobserved. The frontend technical-debt list in `eslint.config.js` disables `max-lines` entirely (`"max-lines": "off"`) for nine paths, and the line counts recorded in its trailing comments — captured in a 2026-08 inventory — have since drifted by 38% in aggregate:

| Exempt path | Annotated | Measured | Drift |
|---|---:|---:|---|
| `src/services/web-agent-client.ts` | 4,137 | 6,013 | +45% |
| `src/services/tauri-agent-client.ts` | 763 | 1,213 | +59% |
| `src/types/agent.ts` | 538 | 702 | +30% |
| `src/contracts/agent.ts` | 364 | 504 | +38% |
| `src/main-layout/main-layout.tsx` | 341 | 528 | +55% |
| `src/services/agent-service.ts` | 307 | 665 | +117% |
| `src/main-layout/create-session-dialog.tsx` | 306 | 318 | +4% |
| `src/settings/pages/sdk-page.tsx` | 393 | 396 | +1% |
| `src/services/coordination-runtime.ts` | 330 | *(file deleted)* | dead entry |
| **Total** | **7,479** | **10,339** | **+38%** |

The list has also decayed structurally: one entry names a file that no longer exists, so nothing is checking whether the list still describes reality.

On the native side there is no file-size gate at all — `src-tauri/tests/architecture.rs` enforces layering and dependency direction but nothing about size — and `api_process_adapter.rs` grew from 13,325 to 13,927 lines in the two days between the audit that produced the optimization ticket and this change.

This matters now because the planned decomposition work (splitting `api_process_adapter.rs`, `web-agent-client.ts`, and `migrations.rs`) runs in parallel branches. Without a measured baseline recorded first, those branches have nothing to prove reduction against, and any file they do not touch keeps growing during the work.

## What Changes

- Add a native line-budget architecture test to `src-tauri/tests/architecture.rs` that fails when a registered path or subtree exceeds its recorded budget.
- Replace the frontend `"max-lines": "off"` blanket exemption with an explicit per-file budget for each currently-exempt file, so the global 300-line rule keeps applying to everything else and the exempt files can only shrink.
- Express every budget as both a **path budget** (one file or glob) and a **subtree budget** (aggregate physical lines under a directory), so that converting a single file into a directory module satisfies the gate instead of breaking it, and so that a "split" that duplicates rather than moves code is still rejected.
- Treat a missing registered path as satisfied, since its subtree budget continues to bound the code that replaced it.
- Require budget increases to be an explicit, reviewed edit with a recorded justification; decreases need no ceremony.
- Correct the stale line annotations in the `eslint.config.js` technical-debt list to the measured values, and drop the entry for the deleted `src/services/coordination-runtime.ts`.
- **No runtime behavior changes.** This change touches test and lint configuration only; no Tauri command, SQLite schema, React component, or adapter behavior is affected in either the desktop or Web runtime.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `repository-governance`: The existing "Existing source constraints remain enforced" requirement bans new blanket exemptions but the frontend list is exactly that. Add a line-budget ratchet requirement covering both the native and frontend source trees, define budgets as path-plus-subtree pairs so directory-module refactors remain compliant, and require that raising a budget be an explicit reviewed edit.

## Impact

- `src-tauri/tests/architecture.rs` — new budget test and its accepting/rejecting fixtures.
- `eslint.config.js` — technical-debt block changes from `"max-lines": "off"` to per-file numeric budgets; stale annotations corrected.
- Frontend budget enforcement runs inside `npm run lint:ci`; native budget enforcement runs inside `cargo test --manifest-path src-tauri/Cargo.toml`. No new CI job is introduced.
- Downstream decomposition changes for `api_process_adapter.rs`, `web-agent-client.ts`, and `migrations.rs` will lower these budgets as they land; this change only records the baseline and the mechanism.
- No frontend/backend isolation or runtime adapter boundary is affected — the change adds no production code.
