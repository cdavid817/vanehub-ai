## Context

See proposal.md — Why for the measurements and for why the ticket's "lost symmetry" diagnosis does not hold.

Four properties of the file determine how it can be cut:

- **Three composition styles already coexist** in the `webAgentClient` object literal: inline `async` methods (the 218 to be extracted), per-method delegation (`getDesktopUpdateSnapshot: webDesktopUpdateClient.getSnapshot`), and whole-module spread (`...webBuiltinToolClient` and three others at lines 2845-2848).
- **The established extraction pattern is a narrow interface plus a spread.** `web-builtin-tool-client.ts` types itself `: BuiltinToolService`, `web-lsp-client.ts` types itself `: WebLspClient`. Neither uses `Pick<AgentService, ...>`.
- **15 module-level `let` bindings hold mock state** — `webSkills`, `webAgentRuns`, `webEvaluationArenas`, `webAgentMemories`, `webRetrievalConfiguration` and others, at lines 220-2801. Methods read and reassign them.
- **10 `this.` call sites** exist, where one method calls another through the composed object.

## Goals / Non-Goals

**Goals:**

- Move the remaining inline methods into bounded-context modules, ending with `web-agent-client.ts` as a composition root.
- Keep every intermediate step compiling and green, so the work can be paused or split across sessions.
- Leave the observable surface — signatures, return shapes, mock data, ordering, timing — untouched.

**Non-Goals:**

- Changing what the Web/mock adapter simulates. A method that returns three fake sessions today returns the same three afterwards.
- Consolidating `localStorage` in the nine other `src/services` files that use it. This change consolidates only this file's 12 accesses.
- Touching any React component, or the Tauri client.
- Reaching a specific line count. The target is "composition root plus what genuinely belongs there", and the budget records whatever that turns out to be.

## Decisions

### Follow the module-plus-spread pattern already in this directory

Each extracted group becomes `web-<context>-client.ts` exporting one object typed against a narrow service interface, spread into `webAgentClient`. This is what `webBuiltinToolClient`, `webSessionWorkspaceClient`, `webCodeReviewClient`, and `webLspClient` already do in this same file, and what the Tauri counterpart does with 19 modules.

*Alternative rejected — per-method delegation properties*: the file already has six of these for desktop updates. They cost one line per method forever and do not shrink the composition root proportionally. Spread costs one line per group.

### Mock state moves with the methods that own it, and stays module-private

This is the hazard that decides whether the refactor is safe. A `let webEvaluationArenas` read by an extracted method but left behind in `web-agent-client.ts` would need exporting, and an exported mutable binding read in two modules is one accidental re-import away from two divergent copies of the mock world — a bug that presents as the UI showing stale data in one panel and fresh data in another.

So the cut lines follow the state, not the method names: a group is extractable only when every `let` it touches moves with it and stays unexported. A method touching state that two groups share is left in the composition root until a later step separates it.

### `this.` call sites survive spread, but the extracted module must say so

Spread copies function references onto the final object, so a spread method's `this` is `webAgentClient` at call time and the 10 existing `this.` calls keep resolving. TypeScript does not infer that on its own: an extracted method using `this` needs an explicit `this: AgentService` parameter, or its callee must move into the same group. Rewriting `this.x()` into a direct import is not equivalent — it would bypass any later override in the composition root.

### The subtree budget must not rise, and that is the point

`freeze-large-file-line-budgets` records `src/services` at 18,149 aggregate lines. Extraction moves code *within* that subtree, so the aggregate is neutral apart from per-module boilerplate. Unlike the two native lanes, this lane has no legitimate reason for a large raise: a materially higher aggregate means methods were rewritten or duplicated rather than moved, and the budget failing is the correct outcome.

### Verify each step against the type checker and the contract test, not by reading

`webAgentClient` is annotated `: AgentService`. Any method dropped, renamed, or given a different signature during extraction is a `tsc --noEmit` error, and `contract-conformance.test.ts` covers the surface besides. That makes each extraction step independently verifiable, which is what allows the work to proceed group by group instead of as one unreviewable diff.

## Risks / Trade-offs

- **Shared mutable mock state is split across modules** → The cut rule above is the control: state moves with its methods and stays unexported. If a group cannot satisfy that, it is not extracted in this change.
- **An extracted method loses its `this` binding** → Caught by `tsc --noEmit` when the method declares `this: AgentService`; the 10 call sites are few enough to enumerate and check individually.
- **A method is silently dropped** → `webAgentClient: AgentService` makes it a compile error, since `AgentService` declares all 252 methods.
- **Mock behavior drifts during a move** → The existing `web-agent-client.test.ts` and `contract-conformance.test.ts` run at every step; a changed return shape or ordering fails them.
- **The file keeps growing while the extraction is in flight** → The per-file ESLint budget freezes it at 6,013, so concurrent work cannot enlarge the target mid-refactor.
- **Merge conflicts with concurrent work** → This lane owns `src/services/`. The other two lanes in this batch are Rust-only and do not overlap.

## Migration Plan

No deployment step. Each extraction group is an independently revertable commit that leaves `tsc --noEmit`, `npm run test`, and `npm run contracts:check` green, so the work can stop at any group boundary without leaving the tree in a half-migrated state.
