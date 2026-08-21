## Context

See proposal.md — Why for the re-measurement and for where `split-web-agent-client`'s numbers needed correcting.

Four properties of the post-#168 file determine how the hub can be cut:

- **The hub is 99 of 116 inline methods over 45 mutable module-level bindings** — 26 `let` plus 19 `const` Map/Set/array containers mutated in place. The 17 non-hub methods are the 14 skill-overlay delegations plus `inspectProject`, `selectProjectDirectory`, `selectWorkspaceDirectory`.
- **The bindings that fuse it are few.** Removing `sessions`, `activeSessionId` and `workflowState` from the shared set, and separately `webSkills`, `webSkillMountPaths`, `webSkillDocuments`, `webSkillApiAgentBindings`, `webSkillResourceDocuments`, `deletedBuiltinSkillIds` and `nextWebSkillRevision`, is what breaks the component. Everything else — loops, agent runs, memories, terminals, categories, expert roles, known projects, chat configs — is already a small cluster that only reaches its neighbours through those two sets.
- **Reads dominate.** Most hub methods reach `sessions` read-only through `findSession()`, and reach `webSkills` read-only through `findWebSkill()` / `hydrateSkillBindings()`. Those helpers are already the accessor; they simply live in the same file as the binding today.
- **Six `this.` call sites remain** — `getSkillOverview` → `listSkills` and → `detectSkillDrift`, `performMissionControlAction` → `cancelAgentRun` and → `resumeAgentRun`, `syncSkillDrift` → `detectSkillDrift` (already extracted), and the skill-overlay group.

## Goals / Non-Goals

**Goals:**

- Break the hub by relocating its two fusing binding sets into state modules that own them and expose accessors, so later cuts follow real context boundaries.
- Extract every context the new seams unblock, keeping each intermediate step compiling and green so the work is resumable at a group boundary.
- Leave the observable surface — signatures, return shapes, mock data, ordering, timing — untouched.

**Non-Goals:**

- Changing what the Web/mock adapter simulates.
- Extracting the cascade methods themselves as a goal in their own right. They land wherever their primary context lands; the point of the state modules is that no cut is *forced* by them any more.
- Touching any React component, the Tauri client, or Rust.
- Reaching a specific line count.

## Decisions

### A state module owns bindings and exports behaviour; it never exports a binding

This is the constraint `split-web-agent-client` set and it carries over unchanged. `web-session-state.ts` declares `let sessions` and never exports it; callers get `listWebSessions()`, `findWebSession(id)`, `updateWebSession(id, updates)`, `replaceWebSessions(next)`. `web-skill-state.ts` declares `let webSkills` and never exports it; callers get `listWebSkills()`, `findWebSkill(...)`, `upsertWebSkill(...)`.

The failure this prevents is concrete: an exported `let` is copied into the importing module's binding at import time under some bundler and interop configurations, so a reassignment in the owner is invisible to the importer, and the mock world silently forks. An exported function reads the live binding on every call, so it cannot fork.

*Alternative rejected — a single `web-mock-world.ts` holding all 45 bindings*: it would break the union-find edge just as well, and it would be one 600-line god module reproducing the problem one directory level up. State modules are per context, with the same bounded-context discipline as the client modules.

### Accessors are the narrowest set the call sites actually need

Every accessor pair costs lines that a direct read/write did not, and the `src/services` aggregate budget is the check on that. So the accessor set is derived from the call sites, not designed up front: `activeSessionId` gets a getter and a setter because it is read in 24 methods and assigned in 9; `nextSessionId` gets only `nextWebSessionSequence()` because every use is `id-${nextSessionId++}`.

### The cascade methods stay whole and move with their primary context

`sendMessage` touches sessions, streams, messages, runs, memories and skills. It is not split. It moves to the chat context and reaches the other five through their state modules' accessors. This is a deliberate reversal of #168's rule "a method touching state that two groups share stays in the composition root": that rule was correct while the state was file-local, and is unnecessary once the state is behind accessors.

### Seed data moves to its own module when the state module would exceed 300 lines

`webSkills`' initial value is 145 lines of fixture. A state module carrying it would break the 300-line ESLint rule that applies to all production `ts`. Fixture data moves to a `web-*-seeds.ts` exporting factory functions; the state module imports them to initialise its bindings. `web-session-workspace-fixtures.ts` and `web-prompt-hook-seeds.ts` already do this.

### `this.` call sites survive spread, but the extracted method must say so

Unchanged from #168: a moved method that uses `this` declares an explicit `this: AgentService` parameter. Rewriting `this.x()` into a direct import is not equivalent — it would bypass any later override in the composition root.

### Exported test seams keep their current import path

`resetWebAgentMemoriesForTest`, `resetWebLoopsForTest`, `simulateWebLoopRestartForTest`, `setWebLoopPhaseDelayForTest`, `seedWebMissionControlRunsForTest`, `resetWebMissionControlRunsForTest`, `seedWebRecoverySessionForTest`, `resetWebRecoverySessionsForTest`, `seedWebImSessionForTest`, `resolveWebMockToolApproval`, `WEB_MOCK_QUESTION_TRIGGER`, `WEB_MOCK_PLAN_EXIT_TRIGGER` are imported from `web-agent-client` by tests and by other modules. When their implementation moves, `web-agent-client.ts` re-exports them, exactly as #168 did for `resetWebRetrievalForTest` and `resetWebEvidenceForTest`. No test file is edited.

### Verify each step against the type checker and the contract test, not by reading

`webAgentClient` is annotated `: AgentService`, and `[ARCH-FE-003]` fails the build if the annotation goes missing. Any method dropped, renamed or given a different signature is a `tsc --noEmit` error, and `contract-conformance.test.ts` covers the surface besides.

## Risks / Trade-offs

- **An accessor changes semantics by accident** — e.g. returning a copy where the original returned the live array, so a caller's in-place mutation stops taking effect. Control: accessors return exactly what the previous expression returned; `listWebSessions()` returns the live array because `sessions` was read live. Where a caller mutated a binding in place, it gets a mutator rather than a getter.
- **The accessor layer inflates the aggregate.** Control: the `[ARCH-FE-004]` subtree budget. A per-module fixed cost plus a bounded accessor set is the expected shape; anything materially larger means a method was rewritten.
- **A moved method loses its `this` binding** → `tsc --noEmit` catches it once the method declares `this: AgentService`.
- **A method is silently dropped** → `webAgentClient: AgentService` makes it a compile error.
- **Mock behavior drifts during a move** → `web-agent-client.test.ts`, the adapter-parity suites, and `contract-conformance.test.ts` run at every group boundary.
- **The hub proves genuinely irreducible.** If a context cannot be cut without exporting a binding, it is left in the composition root and the reason is recorded, rather than papered over by exporting the binding.

## Migration Plan

No deployment step. Each extraction group is an independently revertable commit that leaves `tsc --noEmit`, `npm run test` and `npm run contracts:check` green, so the work can stop at any group boundary without leaving the tree half-migrated.
