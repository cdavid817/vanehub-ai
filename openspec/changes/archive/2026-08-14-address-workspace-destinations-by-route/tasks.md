## 1. Route structure

- [x] 1.1 Add `src/main-layout/workspace-route.ts`: `parseWorkspaceLocation`, `workspacePath`, and the launch-location memory, covering `sessions`, `sessions/new`, `sessions/:sessionId`, `plans`, `loops`, and `work-board`.
- [x] 1.2 Serve every workspace URL from a single `<Route path="/workspace/*">`. Destinations are parsed inside the layout, so React Router never swaps route elements and never unmounts a destination.
- [x] 1.3 Replace the `destination` `useState` with the route-derived value, keeping the hidden-but-mounted rendering and the `loopCenterVisited` / `planCenterVisited` / `workBoardVisited` gates intact.
- [x] 1.4 Derive the visited gates from the active destination. They were set in click handlers, so a deep link to `/workspace/loops` rendered an empty region.
- [x] 1.5 Fall back to sessions for an unknown destination segment.

## 2. Active session

- [x] 2.1 Add `src/main-layout/use-workspace-session-route.ts` to reconcile the route's session with the backend-owned active session in both directions.
- [x] 2.2 Report and fall back when the route names a session that no longer exists, but only once a session list has arrived so a deep link does not bounce on first paint.
- [x] 2.3 Keep `loopInspection` local; only the destination switch it performs navigates, and the `inspectionRequestRef` ordering guard is retained.
- [x] 2.4 Bound a rejected switch to one attempt per route change: `useSessionSwitch` restores the previous active session on failure, which would otherwise retry forever.

## 3. External entry points

- [x] 3.1 Replace `?createSession=1` with `/workspace/sessions/new` and update the floating assistant handler.
- [x] 3.2 Audit every former `setDestination` call site. Async ones (`inspectLoopSession`, the floating assistant subscription) keep their staleness guards.
- [x] 3.3 Restore the previous workspace location on launch.

## 4. Retention verification

- [x] 4.1 `tests/e2e/workspace-routing.spec.ts` asserts that leaving and returning to the Loop Center keeps it mounted and does not replay its loading state.
- [x] 4.2 Assert Back returns through the destinations it came from.
- [x] 4.3 Assert relaunch resumes the previous destination.
- [x] 4.4 `tests/e2e/workspace-activity-bar.spec.ts` "preserve mounted session workspace state for later return" passes unmodified. Its only edit is the URL assertion after opening the scheduled-tasks dialog, which is a dialog rather than a destination and must not change the route.

## 5. Regression surface

- [x] 5.1 `npx playwright test`: 104 pass. Two specs needed repair — the scheduled-tasks URL assertion above, and a retention assertion of mine that raced a cold Vite compile.
- [x] 5.2 `npm run docs:screenshots:check` passes with no baseline regeneration; routing changes no rendering.
- [x] 5.3 `npm run lint:ci`, `npm run test` (994), `npm run build`, `openspec validate --specs --strict` (107), `cargo fmt --check` all pass. `cargo clippy --all-targets -- -D warnings` and `cargo test` were not run: this change touches no Rust and both require a full rebuild in a fresh worktree.
- [x] 5.4 `npm run contracts:check` passes.
- [x] 5.5 `src/main-layout/main-layout.tsx` is 443 lines. It is on the `eslint.config.js` technical-debt exemption list and was 444 before this work began, so the change does not worsen it; the session reconciliation was extracted to its own hook rather than inlined.
