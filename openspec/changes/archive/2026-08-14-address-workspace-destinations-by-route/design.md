## Context

The workspace shell was built before it had four destinations. A single `destination` flag was reasonable when there was one; it now carries Sessions, Plans, Loops, and Todo Board, plus an inspection mode that jumps from the Loop Center into a session. Each addition made the flag carry more meaning and made the absence of a URL more visible.

The existing routing requirement anticipated this. This change makes the implementation catch up rather than introducing a new idea.

## Goals / Non-Goals

Goals:

- Destination and active session derived from the URL.
- Back, Forward, reload, and deep links behave the way they do in `/settings` today.
- Mounted destination state survives navigation exactly as it survives the current flag flip.

Non-Goals:

- Session tabs and information panel tabs. Separate retention model, separate change.
- Any change to what the destinations render.
- Persisting workspace location across machines. Restoring on launch is local.

## Decisions

### One catch-all `/workspace/*` route, not one route element per destination

`/workspace/sessions`, `/workspace/plans`, `/workspace/loops`, `/workspace/work-board`, with the active session as `/workspace/sessions/:sessionId` and creation as `/workspace/sessions/new`. The URL segment matches the destination identifier used in code, so the board is `work-board` rather than a shorter third name for the same thing.

All of them resolve through a single `<Route path="/workspace/*">`. The segment is parsed inside the layout by `parseWorkspaceLocation`; React Router never sees a route change between destinations and therefore never unmounts anything.

### Keep destinations mounted; let the route pick which is visible

This is the decision that determines whether the change is safe. Today `main-layout.tsx` renders every visited destination and hides the inactive ones with `hidden`, gated by `loopCenterVisited` / `planCenterVisited` / `workBoardVisited`. That is what makes returning to the Loop Center cheap and what `main-layout-ui` means by "preserve the existing Session, Plan, and Loop destination state for later return".

React Router's default is to unmount the previous route element. Four sibling route elements — the obvious way to express four destinations — would therefore destroy exactly the state two specs and one e2e suite assert. The catch-all route above avoids the problem structurally rather than by remembering to avoid it.

The visited flags also had to move: they were set in click handlers, which meant a deep link to `/workspace/loops` would render an empty region because nothing had "visited" it. They are now derived from the active destination.

The alternative — accept unmounting and rebuild state on return — was rejected. It would regress asserted behavior and trade a navigation improvement for a responsiveness regression on every destination switch.

### The route owns the session id; the backend still owns the active session

`activeSession` is not local state — it comes from a `["sessions", "active"]` query and is written by `agentService.switchSession`. Making the session addressable therefore creates two claims about one fact.

They are reconciled in one direction each: when the route names a session the backend does not have active, the layout switches to it; when the backend's active session changes and the route does not name it, the layout replaces the URL. A route naming a session that no longer exists reports and falls back, but only once the session list has loaded — otherwise a deep link bounces on first paint before the data arrives.

`onNavigate` takes a whole location rather than a patch, so it depends only on `navigate` and stays referentially stable. A patch-shaped callback would close over the current location, change every render, and re-fire the reconciliation effect continuously.

### Loop inspection stays local state

`loopInspection` overlays a fetched session and messages over the Sessions destination and is cleared on any other selection. Encoding it in the URL would require the target session's message window to be addressable, which it is not. It stays local; only the destination switch it triggers becomes a navigation.

### `?createSession=1` becomes `/workspace/sessions/new`

The parameter exists because the floating assistant needed to express "open the workspace and start creating". A route says the same thing without a boolean query flag, and it gives the create-session dialog the addressability the audit found missing everywhere else.

## Risks / Trade-offs

The retention decision above is the whole risk. If routing is wired the conventional way — one `element` per destination — the change looks correct, passes a superficial reading, and silently breaks state retention that two specs and one e2e suite depend on. Implementation must verify retention explicitly, not assume it.

Second, every current `setDestination` call site becomes a navigation, including ones inside async callbacks (`inspectLoopSession`) and inside the floating assistant event subscription. Navigating from a stale closure after the user has moved elsewhere would yank them back. Those call sites need the same request-ordering guard `inspectionRequestRef` already applies.

Third, `useMainLayoutModel` currently owns session selection. Moving the active session into the URL means the model must accept the session id as input rather than owning it, or the two become competing sources of truth.

## Open Questions

- Should reload restore the exact previous location, or always land on Sessions with the previous session selected? Restoring a mid-inspection Loop Center view may be more confusing than helpful, and the audit produced no evidence either way.
- Should `/workspace` without a destination segment redirect to `/workspace/sessions` or render the last visited destination? A redirect is simpler to reason about; last-visited is friendlier but makes the URL lie briefly on first paint.
