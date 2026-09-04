## 1. Implementation

- [ ] 1.1 Create `src/command-center/goal-search-provider.ts` exporting `goalSearchProvider:
      WorkbenchSearchProvider`, mirroring `run-search-provider.ts`'s shape: `id: "goals"`,
      `supports: (scope) => scope === "goal"`, `search()` calling `goalService.listGoals()`
      (`../services/runtime-goal-client`), case-insensitive substring-matching `title`, mapping
      each match to a `WorkbenchSearchResult` with `key: goal.id`, `kind: "goal"`,
      `title: goal.title`, `status` via a `toStatus(derivedStatus)` mapping (`draft`/`abandoned`
      -> `"neutral"`, `active` -> `"active"`, `awaiting_acceptance` -> `"attention"`, `achieved`
      -> `"success"` -- mirrors `goal-presentation.ts`'s existing `TONES` color intent rather than
      inventing a new one), `route: { destination: "plan", section: "goals", goalId: goal.id }`,
      and `updatedAt: goal.updatedAt`. Slice to `request.limit`, `nextCursor: null`, leave
      `request.signal` unused with the same documented reasoning `run-search-provider.ts` already
      states (not abortable, a shared orchestrator discards stale results).
- [ ] 1.2 Register `goalSearchProvider` in `command-center-registry.ts`'s `SEARCH_PROVIDERS`
      array. Rewrite the file's own doc comment (currently documents 6.6's full deferral) to
      state Goal is now live and cite the real, current, independently-blocking reasons Work
      Item/Evaluation remain deferred (no injectable initial-selection prop on `WorkBoard`; no
      design decision yet for how `EvaluationCenter`'s run-attempt selection maps onto
      "experiment") rather than repeating the old "route adapters don't exist" line verbatim.

## 2. Tests

- [ ] 2.1 Add `src/command-center/goal-search-provider.test.ts`, mirroring
      `run-search-provider.test.ts`'s coverage shape: matches by case-insensitive title substring;
      an empty/no-match query returns no items and no error; a result never carries `description`,
      `acceptanceNotes`, or `links`; every `DerivedGoalStatus` value maps to the intended
      `SemanticStatus`; results respect `request.limit`.
- [ ] 2.2 Extend `command-center-registry.test.ts` to assert the `goal` scope is now covered by
      `SEARCH_PROVIDERS` (matching however that file already asserts session/project/run
      coverage).
- [ ] 2.3 Extend `tests/e2e/command-center.spec.ts` with a live-browser case: open the Command
      Center, search for an existing seeded goal by title, select the result, and assert the app
      lands on the Plan destination's Goals tab with that goal selected -- the same
      real-browser-not-jsdom verification 6.2's own evidence already established for this file.

## 3. Verification

- [ ] 3.1 Run `npx eslint` on every changed/added file, `npx tsc --noEmit` (full project), and
      `npx vitest run src/command-center` -- record pass counts.
- [ ] 3.2 Run `npx playwright test tests/e2e/command-center.spec.ts` (proxy unset) -- record pass
      count.
- [ ] 3.3 Run `openspec validate add-goal-command-center-provider --strict` and
      `openspec validate --specs --strict`.
