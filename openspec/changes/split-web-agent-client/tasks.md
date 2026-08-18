## 1. Map the cut lines

- [x] 1.1 Record the baseline: `web-agent-client.ts` physical lines, its inline `async` method count, and the `src/services` aggregate
  - `web-agent-client.ts`: 6,013 physical lines. `webAgentClient` holds 228 properties — 218 inline `async` methods, 6 per-method delegations, 4 whole-module spreads.
  - `src/services` aggregate: 18,149 physical lines across 143 production files — exactly the `[ARCH-FE-004]` budget, so the subtree starts with zero headroom.
- [x] 1.2 Build the state-ownership map: for each of the 15 module-level `let` bindings, list every method that reads or reassigns it
  - The file has **43** module-level `let` bindings, not 15. The map was built from the TypeScript AST, taking the transitive closure through the 141 top-level helpers, so a method that reaches `sessions` only via `updateSession()` is counted as touching `sessions`.
- [x] 1.3 Group the 218 inline methods into candidate bounded contexts, marking each group extractable only if every `let` it touches is touched by no other group
  - Union-find over shared `let` bindings yields 89 components. One hub component holds 101 methods over 29 bindings (`sessions`, `activeSessionId`, `workflowState`, `webSkills`, `loopRuns`, `webAgentRuns`, …); the other 88 are already disjoint and extractable as-is.
- [x] 1.4 List the methods that share state across candidate groups; these stay in the composition root for this change, and the reason is recorded
  - The hub component is held together by a small set of cascade methods, each of which mutates state belonging to two or more otherwise-disjoint contexts: `deleteApiAgent` (sessions + loops + memories + skills + skill bindings), `sendMessage` (sessions + runs + memories + skills), `createSession` (sessions + projects + remote workspaces + expert roles + retrieval config), `applyCliConfigProfile` (CLI config + workflow), `deleteSession` / `archiveSession` / `resolveAgentQuestion` / `resolvePlanExit` / `stopGeneration` (sessions + runs), `startLoop` / `resumeLoop` / `continueLoop` (loops + sessions + runs), `deleteSessionCategory` / `assignSessionCategory` (categories + sessions), `getSessionChatConfig` / `saveSessionChatConfig` (chat configs + sessions).
  - These stay in the composition root. Extracting one of them would force its co-owned `let` to be exported from the module that keeps it, and an exported mutable binding read from two modules is the divergent-mock-world failure design.md rules out.
- [x] 1.5 Enumerate the 10 `this.` call sites and note, for each, whether caller and callee land in the same group
  - `cancelEvaluation` → `getEvaluationArena` and `exportEvaluation` → `getEvaluationArena` (same group, evaluation); `performMissionControlAction` → `cancelAgentRun` and → `resumeAgentRun` (same group, agent runs); `getContextEvidenceManifest` → `listContextEvidenceManifests` (same group, context evidence); `getSkillEvolutionSeedLineage` → and `purgeSkillEvolutionEvidence` → `querySkillEvolutionEvidence` (same group, skill evolution evidence); `getSkillOverview` → `listSkills` and → `detectSkillDrift`; `syncSkillDrift` → `detectSkillDrift`.
  - Caller and callee land in the same group in every case, so no call is rewritten; each moved caller declares `this: AgentService` so the binding stays late-bound through the composed object.

## 2. Extract group by group

- [x] 2.1 For each extractable group: create `src/services/web-<context>-client.ts`, define its narrow service interface, move the methods and their `let` state verbatim, and keep the state unexported
  - Seven groups shipped as seven commits, moving **102 of the 218** inline methods: evaluation (7), prompt hooks (14), API agent + OnePiece provider + profiles + Hybrid Routing (21), retrieval and code index (16), CLI tools + parameters + config (17), scheduled tasks + context quality (11), skill governance + evidence + agent registry (16).
  - The narrow interface follows `BuiltinToolService`, not `Pick<AgentService, …>`: each group's signatures **move** out of `AgentService` into a `<context>-service.ts` that `AgentService` now extends. No signature is duplicated, and both adapters keep checking against one surface.
  - A group whose module would exceed the 300-line rule is split further (state module + client module, or two client modules over one interface file). Mutable state lives in the state module, unexported, reached through accessors.
- [x] 2.2 Give any moved method that uses `this` an explicit `this: AgentService` parameter rather than rewriting the call into a direct import
  - Six moved methods declare it: `cancelEvaluation`, `exportEvaluation`, `getContextEvidenceManifest`, `syncSkillDrift`, `getSkillEvolutionSeedLineage`, `purgeSkillEvolutionEvidence`. No `this.x()` was rewritten into an import.
- [x] 2.3 Replace the moved methods in `webAgentClient` with a single spread, matching the existing `...webBuiltinToolClient` style
- [x] 2.4 After each group: `tsc --noEmit`, `npm run contracts:check`, and `npm run test` pass before starting the next group
  - Run at every one of the seven boundaries. `npm run test` reported 286 files / 1301 tests passing each time — the same totals as the pre-change baseline.
- [x] 2.5 Lower the `web-agent-client.ts` budget in `eslint.config.js` after each group, so the ratchet tracks the work
  - 6,013 → 5,972 → 5,492 → 4,967 → 4,661 → 4,190 → 4,064 → 3,861. `agent-service.ts` ratcheted down alongside it, 665 → 496.

### Groups deferred to a follow-up

The 101-method hub component described in 1.3 is not extracted here. Every method in it
reaches `sessions`, `activeSessionId`, `workflowState`, `webSkills`, `loopRuns` or
`webAgentRuns`, and the cascade methods listed in 1.4 tie those bindings together. Splitting
it needs a session-state module and a skills-state module first — a larger, separately
reviewable step, which is why this change stops at a clean boundary instead of forcing it.
The skill overlay delegations are blocked by the same edge: `webSkillOverlayRuntime` is
constructed with a resolver that closes over `webSkills`.

## 3. Consolidate this file's localStorage access

- [x] 3.1 Route the 12 `localStorage` accesses in `web-agent-client.ts` through a single storage module, preserving the exact keys, values, and write timing
  - `web-mock-storage.ts` exposes `readWebMockStorage(key, fallback)` and `writeWebMockStorage(key, value)`. All four read/write pairs route through it — CLI parameter selections, session chat configs, prompt hooks, prompt hook traces — keeping the same keys, the same in-memory fallback on a missing or unparseable entry, and the same write ordering (memory first, then storage). The prompt hook trace cap of 50 is unchanged.
  - `grep -c localStorage src/services/web-agent-client.ts` is now 0.
- [x] 3.2 Confirm no plaintext secret reaches browser storage, as the `frontend-runtime-architecture` requirement "Honest Web/mock behavior" requires
  - The four persisted payloads are catalog flag selections, chat model/policy selections, prompt hook templates, and trace hashes. None declares a key, token, credential or password field; `tokenEstimate` on a trace is a count.
  - The credential-bearing mock state is never persisted: `webCliConfigProfiles` keeps `credentialConfigured` and `webOnePieceProviderConfig` keeps `credentialPresent` as booleans in memory only, and neither module imports the storage helper.
- [x] 3.3 Leave the nine other `src/services` files that use `localStorage` untouched, and record them as follow-up scope
  - Measured, the count is three, not nine: `web-floating-assistant-client.ts`, `web-prompt-hook-versions.ts`, `web-settings-client.ts`. Four files outside `src/services` also touch it — `main-layout/{create-session-dialog.tsx,main-layout.tsx,session-sidebar.tsx,workspace-route.ts}`. All seven are untouched here and are the follow-up scope.

## 4. Prove the surface did not move

- [x] 4.1 `webAgentClient` is still annotated `: AgentService` and `tsc --noEmit` passes
  - The annotation is also machine-checked: `[ARCH-FE-003]` in `scripts/architecture/frontend-rules.mjs` fails the build if either adapter loses it.
- [x] 4.2 `npm run contracts:check` passes unchanged — no edit to `contract-conformance.test.ts` was needed
- [x] 4.3 `npm run test` passes with an unchanged total test count
  - 286 files / 1301 tests, before and after. No test file was edited: the two exported test seams that moved (`resetWebRetrievalForTest`/`searchWebCodeIndex`, `resetWebEvidenceForTest`) are re-exported from `web-agent-client.ts`.
- [x] 4.4 Confirm no React component or `src/components` file appears in the diff
  - The diff is `eslint.config.js`, `scripts/architecture/frontend-rules.mjs`, this tasks file, and `src/services/*.ts`. No `.tsx` file and no Rust file.
- [x] 4.5 Confirm `src/services/tauri-agent-client.ts` is untouched
  - `git diff --stat -- src/services/tauri-agent-client.ts` is empty. Splitting `AgentService` into extended interfaces is structural, so the Tauri adapter keeps checking against the same surface without a text change.

## 5. Budgets and verification

- [x] 5.1 Confirm the `src/services` subtree aggregate did not rise beyond per-module boilerplate; a material rise means code was duplicated rather than moved, and must be traced rather than absorbed by raising the budget
  - 18,149 → 18,513, +364 over 32 new files — about 11 lines each. That is the fixed cost per module: the import block, `export const x: XService = {`, the closing brace, and the spread plus import added to the composition root. State modules add a few accessor pairs on top.
  - It is not duplication. Method bodies were moved verbatim, and the narrow interfaces' signature lines moved out of `AgentService` rather than being copied — that is most of why `agent-service.ts` fell 665 → 496. The raise is recorded with its reason in `scripts/architecture/frontend-rules.mjs`.
- [x] 5.2 Set the final `web-agent-client.ts` budget to the measured value, or remove its `eslint.config.js` entry entirely if the file reached 300 lines
  - Set to 3,861. The entry stays: 116 methods are still inline — the 101-method hub component, plus a handful of singletons whose only helpers (`inspectMockProject`, `representativeUsageRecords`, `tr`) are still owned by the root.
- [x] 5.3 `npm run lint:ci`, `npm run build`, and `npm run architecture:check` pass
- [x] 5.4 `npx playwright test` passes, since the Web/mock adapter backs the browser-mode UI these specs drive
- [x] 5.5 `openspec validate split-web-agent-client --strict` and `openspec validate --specs --strict` pass
