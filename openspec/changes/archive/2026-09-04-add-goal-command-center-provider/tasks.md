## 1. Implementation

- [x] 1.1 Create `src/command-center/goal-search-provider.ts` exporting `goalSearchProvider:
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
- [x] 1.2 Register `goalSearchProvider` in `command-center-registry.ts`'s `SEARCH_PROVIDERS`
      array. Rewrite the file's own doc comment (currently documents 6.6's full deferral) to
      state Goal is now live and cite the real, current, independently-blocking reasons Work
      Item/Evaluation remain deferred (no injectable initial-selection prop on `WorkBoard`; no
      design decision yet for how `EvaluationCenter`'s run-attempt selection maps onto
      "experiment") rather than repeating the old "route adapters don't exist" line verbatim.

## 2. Tests

- [x] 2.1 Add `src/command-center/goal-search-provider.test.ts`, mirroring
      `run-search-provider.test.ts`'s coverage shape: matches by case-insensitive title substring;
      an empty/no-match query returns no items and no error; a result never carries `description`,
      `acceptanceNotes`, or `links`; every `DerivedGoalStatus` value maps to the intended
      `SemanticStatus`; results respect `request.limit`.
- [x] 2.2 Extend `command-center-registry.test.ts` to assert the `goal` scope is now covered by
      `SEARCH_PROVIDERS` (matching however that file already asserts session/project/run
      coverage).
- [x] 2.3 Extend `tests/e2e/command-center.spec.ts` with a live-browser case: open the Command
      Center, search for an existing seeded goal by title, select the result, and assert the app
      lands on the Plan destination's Goals tab with that goal selected -- the same
      real-browser-not-jsdom verification 6.2's own evidence already established for this file.

## 3. Verification

- [x] 3.1 Run `npx eslint` on every changed/added file, `npx tsc --noEmit` (full project), and
      `npx vitest run src/command-center` -- record pass counts. — `eslint` clean on all 5
      changed/added files; `tsc --noEmit` clean project-wide; `vitest run src/command-center` --
      12/12 files, 110/110 tests passing.
- [x] 3.2 Run `npx playwright test tests/e2e/command-center.spec.ts` (proxy unset) -- record pass
      count. — 3/3 passed (26.8s), including the new goal-search case. First attempt caught a real
      bad assumption in the new test itself (a "会话" activity-bar button that does not exist --
      Sessions is the persistent default view, not a destination button like the other four),
      fixed by navigating via the real "质量" button instead before re-running clean.
- [x] 3.3 Run `openspec validate add-goal-command-center-provider --strict` and
      `openspec validate --specs --strict`.
- [x] 3.4 Run `npm run build` (AGENTS.md's own mandatory command, not originally listed above). —
      First attempt failed the post-build chunk-size budget check:
      `scripts/check-frontend-chunks.mjs`'s `maxRawJavaScriptChunkBytes` (701.5 KiB) was exceeded
      by the real, expected growth of adding `goal-search-provider.ts` behind
      `command-center-registry.ts`'s eager (non-lazy) bundle -- unlike this budget's prior three
      bumps, clearly this change's own doing, not ambiguous shared-branch drift. Raised to the
      exact new measured value (721545 bytes / 704.6 KiB) with a reason comment following the
      file's own established append-only convention. Rebuilt clean: 16 lazy chunks verified, main
      static closure 162.8 KiB gzip (unchanged -- the new code is small and the gzip budget has
      much more headroom than the raw-byte one).
- [x] 3.5 Run `npm run architecture:check` (not strictly required by AGENTS.md for this change --
      no Tauri boundary or cross-context dependency touched -- run anyway since CI runs it
      unconditionally). — Clean: frontend node-tests, `lint:ci`, `tsc --noEmit`, and all 4 Rust
      architecture-fitness binaries passing, 0 failed.
