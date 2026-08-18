## 1. Re-measure the hub before building on it

- [x] 1.1 Rebuild the state-ownership map from the TypeScript AST with the transitive closure taken through the top-level helpers, and record where `split-web-agent-client`'s numbers need correcting
  - The hub is **99 of the 116 remaining inline methods**, not 101. `split-web-agent-client` measured it against the pre-#168 file; on the post-#168 file the count is 99, and the 17 non-hub methods are the 14 skill-overlay delegations plus `inspectProject`, `selectProjectDirectory` and `selectWorkspaceDirectory`.
  - The map was rebuilt with union-find over shared mutable module-level bindings, taking the transitive closure through the 90 top-level helpers, so a method that reaches `sessions` only via `updateSession()` counts as touching `sessions`.
- [x] 1.2 Separate `let` bindings from `const` containers that are mutated in place, and confirm both classes are treated as shared mutable state by the cut rule
  - The hub spans **45 bindings, not 29**: 26 module-level `let` plus 19 module-level `const` Map/Set/array containers mutated in place. `split-web-agent-client` counted only the `let`s. The `const` containers are as much shared mutable state — `messagesBySession`, `webAgentRunEvents`, `loopTimers`, `sessionEventSubscribers` — and a cut line that ignores them is not safe either, so both classes are treated identically here.
- [x] 1.3 Identify the minimal binding set whose relocation breaks the hub component, and confirm by re-running union-find with those bindings removed
  - The fusing set is 14: `sessions`, `activeSessionId`, `workflowState`, `nextSessionId`, `nextSeatId`, `sessionEventSubscribers` on the session side, and `webSkills`, `webSkillMountPaths`, `webSkillApiAgentBindings`, `webSkillDocuments`, `webSkillResourceDocuments`, `deletedBuiltinSkillIds`, `nextWebSkillRevision`, `builtinSkillSeeds` on the skills side.
  - Re-running union-find with exactly those removed splits the 99-method component into **59 components**: a 44-method residual over loops, runs, memories, chat and recovery, plus 55 smaller ones and 47 singletons. That is the measurement the extraction plan is built on, taken before any code moved.
- [x] 1.4 Enumerate the remaining `this.` call sites and note whether caller and callee land in the same group
  - Six remain. `getSkillOverview` → `listSkills` (same group, catalogue) and → `detectSkillDrift` (a different group, already extracted as `webSkillGovernanceClient`); `performMissionControlAction` → `cancelAgentRun` and → `resumeAgentRun` (same group, mission control); `syncSkillDrift` → `detectSkillDrift` and `getSkillEvolutionSeedLineage` → `querySkillEvolutionEvidence` were already handled by `split-web-agent-client`.
  - Both moved callers declare `this: AgentService`, so the calls stay late-bound through the composed object. No `this.x()` was rewritten into an import.

## 2. Extract the two state modules that fuse the hub

- [x] 2.1 Create the session-state module owning `sessions`, `activeSessionId`, `workflowState`, `nextSessionId`, `nextSeatId` and `sessionEventSubscribers`, exporting accessors only
  - `src/services/web-session-state.ts`, 106 lines. It also takes the four helpers that are the write path for those bindings — `findSession`, `updateSession`, `createWebSeatId`, `sortSessions` — since leaving them behind would have meant exporting the bindings to reach them.
- [x] 2.2 Create the skills-state module owning `webSkills`, `webSkillMountPaths`, `webSkillApiAgentBindings`, `webSkillDocuments`, `webSkillResourceDocuments`, `deletedBuiltinSkillIds` and `nextWebSkillRevision`, exporting accessors only; move the fixture seeds to a seeds module so neither file breaks the 300-line rule
  - `src/services/web-skill-state.ts` (274) plus `src/services/web-skill-seeds.ts` (183). The seeds module exports factory functions and one frozen `readonly` seed list; the state module calls them to initialise its bindings.
- [x] 2.3 Confirm no state module exports a mutable binding, and that the accessor set matches what the call sites need rather than exposing the binding shape
  - Every `let` and every mutated `const` container in the new modules is unexported. `grep -n "^export let"` over `src/services` is empty.
  - The accessor set was derived from the call sites, then audited: every exported accessor has at least one caller outside its own module. The one that did not — `webSkillRefusal` — was un-exported rather than left as dead surface. `activeSessionId` gets a getter and a setter because it is read in 24 places and assigned in 9; `nextSessionId` gets only `nextWebSessionSequence()` because every use was `id-${nextSessionId++}`.
- [x] 2.4 `tsc --noEmit`, `npm run contracts:check` and `npm run test` pass before starting the next group

## 3. Extract the contexts the new seams unblock

- [x] 3.1 Skills: the skill catalogue, mount-path and binding methods, plus the 14 skill-overlay delegations that were blocked by the resolver closing over `webSkills`
  - 33 methods into `web-skill-catalog-client.ts` (287), `web-skill-binding-client.ts` (195) and `web-skill-overlay-client.ts` (86), over one `skill-service.ts`. The overlay runtime construction moved with the delegations: its resolver now reads the catalogue through `listWebSkills()`.
- [x] 3.2 Loops and agent runs / mission control, each with its own state module
  - Agent runs: 8 methods into `web-mission-control-client.ts` (127) over `mission-control-service.ts`, with the fixtures and write path in `web-agent-run-state.ts` (146). The loop, chat and session contexts still project run state; they reach it through `projectWebOwnerRun`, `findWebAgentRun`, `prependWebAgentRun`, `setWebAgentRunEvents` and `isTerminalWebRunState`.
  - Loops: 14 methods into `web-loop-client.ts` (261), with `web-loop-state.ts` (188) and `web-loop-scheduler.ts` (186) over `loop-service.ts`. The scheduler keeps its timer semantics, reading the phase delay and the timer map through accessors.
- [x] 3.3 The smaller session-adjacent contexts the session-state module unblocks — memories, session categories, expert roles, known projects, usage statistics, terminals, recovery, workflow, chat configs
  - 22 methods into `web-session-category-client.ts` (66), `web-expert-role-client.ts` (55), `web-known-workspace-client.ts` (134), `web-agent-terminal-client.ts` (129) and `web-usage-statistics-client.ts` (65), over `session-organization-service.ts`, `agent-terminal-service.ts` and `usage-statistics-service.ts`.
  - Memories, recovery, workflow and chat configs are **not** extracted — see 3.9.
- [ ] 3.4 Chat and messaging, including the cascade method `sendMessage`
- [ ] 3.5 Session lifecycle, including the cascade methods `createSession`, `deleteSession` and `archiveSession`
- [x] 3.6 Each group: a narrow interface whose signatures **move** out of `agent-service.ts`, methods moved verbatim, a single spread in the composition root, and `this: AgentService` on any method that uses `this`
  - Six new interface files; `AgentService` now extends 11 more interfaces and `agent-service.ts` fell 496 → 364. No signature is duplicated.
  - Method bodies moved verbatim apart from three mechanical substitutions, each behaviour-identical: a direct binding read became the accessor call; the file-local `tr(key)` shim became `i18n.t(key)`, which is what it forwarded to and what the other extracted modules already use; and `resetWebLoopsForTest` was split across its two owners (see 3.9).
- [x] 3.7 After each group: `tsc --noEmit`, `npm run contracts:check` and `npm run test` pass before the next group starts
  - Run at all five boundaries. `npm run test` reported 286 files / 1301 tests passing each time — the same totals as the pre-change baseline, with no test file edited.
- [x] 3.8 Lower the `web-agent-client.ts` budget in `eslint.config.js` after each group so the ratchet tracks the work
  - 3,861 → 3,502 → 2,973 → 2,580 → 2,373 → 1,877. `agent-service.ts` ratcheted alongside, 496 → 422 → 392 → 384 → 364.
- [x] 3.9 Record any context that could not be cut without exporting a binding, with the reason, rather than exporting the binding
  - **No context required a binding export.** Nothing in this change is blocked by the rule; what remains is unextracted for scope, not for safety.
  - One seam was split rather than moved whole: `resetWebLoopsForTest` clears loop state *and* `messagesBySession`, which the composition root still owns. Rather than export either binding, the loop state module exposes `clearWebLoopTimersAndSubscribers()` and `clearWebLoopStateForTest()`, and the root calls them around the message cleanup so the original ordering holds. When chat state moves, this returns to a single move.

### Groups deferred to a follow-up

The composition root ends at 1,877 lines with 39 inline methods over 8 remaining bindings —
`messagesBySession`, `subscribersBySession`, `activeStreams`, `nextMessageId`,
`webAgentMemories`, `nextAgentMemoryId`, `memoryChatConfigs`, `recoveryReportsBySession`.
Re-running the AST analysis on the result: the 99-method hub is now a **20-method** component,
and 19 of the methods that were in it are fully disjoint singletons. The remaining work is
three contexts — chat and messaging (including the 470-line `sendMessage`), memories, and
session lifecycle plus recovery — and it is bounded by review size, not by the binding rule.
Each needs its own state module first, exactly as the session and skills sides did here.

## 4. Prove the surface did not move

- [x] 4.1 `webAgentClient` is still annotated `: AgentService` and `tsc --noEmit` passes
  - `[ARCH-FE-003]` in `scripts/architecture/frontend-rules.mjs` also fails the build if either adapter loses the annotation; `npm run architecture:check` passes.
- [x] 4.2 `npm run contracts:check` passes with no edit to `contract-conformance.test.ts`
- [x] 4.3 `npm run test` passes with an unchanged total test count and no test file edited
  - 286 files / 1301 tests, before and after. The moved test seams — `setWebLoopPhaseDelayForTest`, `simulateWebLoopRestartForTest`, `seedWebMissionControlRunsForTest`, `resetWebMissionControlRunsForTest` — are re-exported from `web-agent-client.ts`, so no importer changed.
- [x] 4.4 No React component, `.tsx` file, or Rust file appears in the diff
  - The diff is `eslint.config.js`, `scripts/architecture/frontend-rules.mjs`, this change's artifacts, and `src/services/*.ts`.
- [x] 4.5 `src/services/tauri-agent-client.ts` is byte-identical
  - `git diff --stat` against the branch point is empty for that path. Splitting `AgentService` into extended interfaces is structural, so the Tauri adapter keeps checking against the same surface without a text change.

## 5. Budgets and verification

- [x] 5.1 Confirm the `src/services` subtree aggregate rose only by per-module boilerplate plus the bounded accessor set; trace any material rise rather than absorbing it into the budget
  - 18,513 → 19,087, +574 over 23 new files. About 11 lines each is the fixed per-module cost `split-web-agent-client` measured (imports, `export const x: XService = {`, the closing brace, the spread and import in the root); the remaining ~320 is the accessor layer, which is the price of the "never export a mutable binding" rule — a direct `sessions.filter(...)` read is now `listWebSessions().filter(...)`, and each pair of `list*`/`replace*` functions costs six lines that a bare binding did not.
  - It is not duplication. Method bodies moved verbatim, and the narrow interfaces' signatures moved out of `AgentService` rather than being copied, which is why `agent-service.ts` fell 496 → 364. The accessor set was audited: every exported accessor has a caller outside its owning module.
- [x] 5.2 Set the final `web-agent-client.ts` and `agent-service.ts` budgets to the measured values, or remove the `web-agent-client.ts` entry if the file reached 300 lines
  - Set to 1,877 and 364. Both entries stay: 39 methods are still inline.
- [x] 5.3 `npm run lint:ci`, `npm run build` and `npm run architecture:check` pass
- [x] 5.4 `npx playwright test` passes, since the Web/mock adapter backs the browser-mode UI these specs drive
- [x] 5.5 `openspec validate extract-web-client-state-modules --strict` and `openspec validate --specs --strict` pass
