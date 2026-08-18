## Why

`src/services/web-agent-client.ts` is 6,013 lines with 218 inline `async` methods. It is the largest frontend file by a wide margin, and it grew 696 lines between the audit that produced the optimization ticket and the branch this change starts from.

The optimization ticket diagnosed this as lost adapter symmetry: "web has 187 async methods, tauri has 38 — a 5× gap showing the web/mock client absorbed mock data construction and state simulation; interface symmetry is gone." **That diagnosis does not survive measurement.** The `AgentService` interface declares 252 Promise-returning methods. Both clients are explicitly annotated `: AgentService` — `webAgentClient` at `web-agent-client.ts:2825`, `tauriAgentClient` at `tauri-agent-client.ts:226` — so TypeScript already rejects an asymmetric implementation at compile time, and `src/contracts/contract-conformance.test.ts` covers it besides. Symmetry is enforced, not lost.

The real difference is decomposition, and it runs the other way from what the ticket implies. `tauri-agent-client.ts` keeps only 39 methods inline and composes the rest by spreading sibling modules; there are **19** `tauri-*` service modules. The web side already has **36** `web-*` modules — more than the Tauri side — but `web-agent-client.ts` kept 218 methods inline and spreads only four (`web-agent-client.ts:2845-2848`). So the pattern this file needs is not novel: it is the pattern its own directory and its own counterpart already use, applied to the methods that never got extracted.

## What Changes

- Extract the remaining inline method groups from `web-agent-client.ts` into bounded-context `web-*` modules, composed into `webAgentClient` by spread — the structure `tauri-agent-client.ts` already uses.
- Keep `webAgentClient` typed `: AgentService` throughout, so the type checker holds the surface fixed at every intermediate step.
- Consolidate the 12 `localStorage` accesses in this file behind a single storage module. Nine other `src/services` files also touch `localStorage`; this change consolidates only its own, without changing what is stored or when.
- Lower the recorded per-file budget for `web-agent-client.ts` as the extraction proceeds.
- **No UI change and no behavior change.** No React component is touched. Every method keeps its current signature, return shape, mock data, and timing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure code-organization refactor of one adapter's internals. The `frontend-runtime-architecture` requirements "Mechanically enforced runtime adapter parity" and "Honest Web/mock behavior" describe behavior this change deliberately preserves; the latter's prohibition on persisting plaintext secrets to browser storage constrains the `localStorage` consolidation but is not altered by it. The change sets `skip_specs: true`.

## Impact

- `src/services/web-agent-client.ts` — shrinks to a composition root plus whatever genuinely belongs there.
- `src/services/web-*.ts` — new modules join the 36 that already exist, following their naming and export conventions.
- `eslint.config.js` — the `web-agent-client.ts` per-file budget drops; the entry is removed entirely if the file reaches 300 lines.
- `scripts/architecture/frontend-rules.mjs` — the `src/services` subtree budget of 18,149 continues to bind and must not rise; an extraction that moves code within the subtree is aggregate-neutral.
- `src/contracts/contract-conformance.test.ts` — must keep passing unchanged; it is the evidence the surface did not move.
- No Rust file is touched. No Tauri command, SQLite schema, or native behavior is affected.
