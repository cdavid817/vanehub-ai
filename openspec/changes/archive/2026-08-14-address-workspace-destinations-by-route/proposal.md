## Why

`frontend-runtime-architecture` already requires the frontend to "expose top-level application surfaces through a routing layer that can address workspace, settings, and future detail views without relying on a single component-local view flag". The workspace does not meet that requirement today.

`src/App.tsx` registers exactly two routes, `/workspace` and `/settings`. Everything inside the workspace — the Sessions, Plans, Loops, and Todo Board destinations, the selected session, the nine session tabs, and the information panel tabs — lives in `useState` inside `MainLayout`. `destination` is literally the component-local view flag the requirement names.

The consequences are user-visible. A UX audit (`docs/ux-audit-report.md`, P2-12) recorded them:

- Browser and keyboard Back do nothing inside the workspace. A user who opens a session from the Loop Center can only return through the one back button that surface happens to render; the same jump from Plan Center has no return affordance at all.
- No workspace state survives a reload. Reopening the app always lands on Sessions with no session selected, discarding the destination, session, and tab the user was on.
- Nothing inside the workspace can be linked to. The floating assistant, notifications, and IM connectors can only route to `/workspace` wholesale, which is why `App.tsx` carries a `?createSession=1` query parameter as a one-off substitute for addressability.

## What Changes

- Make the four workspace destinations addressable as routes under `/workspace`, so the active destination is derived from the URL rather than from `useState`.
- Make the active session addressable, so a session can be linked to and restored.
- Restore the last workspace location on launch, and make Back and Forward traverse destination and session changes.
- Replace the `?createSession=1` parameter with a route that expresses the same intent.
- Preserve mounted state across destination changes. The current implementation keeps visited destinations mounted and hidden so their state survives switching; routing MUST NOT regress that into unmount-on-navigate.

Explicitly out of scope: the nine session tabs and the information panel tabs stay component-local in this change. They are a second addressability layer with their own retention semantics (`session-workspace-tabs`), and folding them in would make one change responsible for two independent state models.

Also out of scope: `main-layout.tsx` remains over the 300-line limit under its existing `eslint.config.js` exemption. Splitting its destination sections out is worth doing but is unrelated to addressability, and doing both at once would make the retention risk below much harder to review.

## Capabilities

### Modified Capabilities

- `frontend-runtime-architecture`: State that workspace destinations and the active session are addressable, and that navigation is restorable and reversible.
- `main-layout-ui`: State that activating a destination changes the route, that Back returns to the previous destination or session, and that mounted destination state survives navigation.

## Impact

- Frontend only. No Tauri command, service interface, DTO, or persistence change.
- `src/App.tsx` gains nested routes; `src/main-layout/main-layout.tsx` reads destination and session from route params instead of `useState`.
- The floating assistant handler in `App.tsx` and every `onOpenSettings` / inspection callback that currently mutates local state becomes a navigation.
- Deep links and Back/Forward become part of the tested surface, which they are not today.
- Behavioral risk is concentrated in state retention: `loopCenterVisited` / `planCenterVisited` / `workBoardVisited` and the hidden-but-mounted sections exist to keep destination state alive. A naive route split would unmount them and silently regress the "preserve mounted session workspace state for later return" scenarios already asserted in `main-layout-ui` and `tests/e2e/workspace-activity-bar.spec.ts`.
