## Context

See `proposal.md` for motivation. The completed `add-session-im-toggle` change introduced a connector-keyed access table and typed access contract, but the application service authorizes only Feishu, the binding view always reads the Feishu row, and the session hook and panel hard-code Feishu. All five connector transports and global lifecycle controls already exist.

## Goals / Non-Goals

**Goals:**

- Make the existing connector-keyed access record authoritative for every built-in connector.
- Keep connector selection, access mutation, pairing, and bound-state rendering on one stable connector id across asynchronous UI operations.
- Preserve desktop/Web adapter parity and native deny-by-default enforcement.
- Reuse existing transports, pairing, binding, deduplication, routing, and final-response delivery.

**Non-Goals:**

- Supporting group chats, attachments, cards, reactions, voice, or streaming edits.
- Allowing more than one external chat binding per VaneHub session.
- Adding new connector credentials, protocol implementations, or live external-platform qualification.
- Changing global connector configuration or lifecycle semantics.

## Decisions

### 1. Query access for an explicit connector

Extend the session binding query with a connector argument. The response continues to contain one binding and one access record, but the access record is for the requested connector unless a binding exists, in which case the bound connector is authoritative. This avoids returning an unbounded access collection and makes stale frontend selections detectable.

Alternative considered: return all five access rows. Rejected because missing rows already normalize to disabled and the session surface needs only the selected connector.

### 2. Enforce access uniformly in the communications application service

Remove the Feishu bypass from the shared authorization function. Pairing creation, pairing consumption, inbound admission, and completion notification already call this function, so one change establishes the same deny-by-default boundary for every transport. The existing read/write gate continues to serialize access mutation against admission.

Alternative considered: add checks inside each transport. Rejected because it would duplicate policy and let protocol-specific paths drift.

### 3. Keep one selected connector in the session hook

The hook owns a selected connector id. It initializes from the binding when present, otherwise from the first healthy configured connector using descriptor order. Selection triggers a request-versioned binding/access reload; access mutation and pairing capture the selected id at operation start. The panel disables selection while bound, pairing, or mutating.

Alternative considered: one toggle per connector. Rejected because the current one-binding-per-session model would make simultaneous enabled toggles misleading and increase accidental authorization.

### 4. Preserve runtime adapter symmetry

`ImService.getSessionBinding` accepts the connector id in both Tauri and Web implementations. The Tauri adapter passes it through the registered command and strictly parses the response; the native command retains Feishu as the compatibility default for older callers that omit the new argument. The Web adapter keys access by session and connector and mirrors bound-connector precedence. React components still call only `ImService`.

### 5. Generalize deterministic verification without live credentials

Unit, component, and Playwright tests cover all stable connector ids. Native tests exercise the shared authorization function for every connector. The desktop fixture gains connector selection and safe event injection using existing normalized inbound fixtures; no production-only fixture command or credential path is added.

## Risks / Trade-offs

- [Existing non-Feishu bindings have no access row after upgrade] → Backfill enabled access for every existing binding, preserving previously authorized chats while new and unbound sessions remain disabled.
- [Selection changes race with reload or pairing] → Version reloads and capture the selected connector per mutation; ignore stale responses.
- [A bound connector differs from the requested connector] → Return and render the bound connector as authoritative until replacement/removal.
- [Personal WeChat is experimental] → Keep its experimental label and authorization flow; uniform session authorization does not promote its maturity status.

## Migration Plan

1. Add an append-only migration that backfills enabled access for existing non-Feishu bindings; make it repeat-safe.
2. Generalize native access lookup and authorization while keeping missing rows disabled.
3. Extend service/adapters and add selected-connector hook state and UI.
4. Add focused native, frontend, browser, and deterministic desktop coverage.

Rollback can hide multi-connector selection and restore the Feishu-only UI while leaving connector-scoped access rows harmless. A downgrade continues to ignore non-Feishu access rows without deleting bindings or credentials.
