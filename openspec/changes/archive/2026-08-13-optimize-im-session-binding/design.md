## Context

See `proposal.md` for motivation. Today the communications context stores connector configuration globally, requires a singleton `im_routing_settings` record before connector startup, and creates a non-activating IM-owned session when a new external chat first sends text. `im_session_bindings` hashes the external chat id and maps it to the created session. Existing bindings already execute through the session's persisted chat configuration, which is the behavior this design promotes to the primary model.

The frontend has a runtime-neutral `ImService`, Tauri and Web implementations, an IM settings page, and a session information panel. React components must remain unaware of Tauri commands. Native connector credentials and platform delivery state must remain behind the Rust communications boundary and follow unified logging and redaction requirements.

## Goals / Non-Goals

**Goals:**

- Make connector availability independent from Agent and workspace routing.
- Attach an authenticated external direct chat to an existing session without changing the active desktop session.
- Preserve session-owned Agent, effective workspace/worktree, model, permission, history, and provider continuity.
- Provide a safe, expiring pairing handshake and session-level lifecycle controls.
- Preserve legacy bindings and keep desktop and Web/mock service contracts aligned.
- Support safe completion notifications without creating a general conversation-mirroring subsystem.

**Non-Goals:**

- Group chats, shared-channel collaboration, attachments, voice, cards, or non-text Agent input.
- More than one active external-chat binding per session in the first version.
- Arbitrary proactive IM composition or mirroring desktop prompts and responses.
- Selecting or creating projects, worktrees, Agents, or models from IM.
- Replacing connector-specific authentication with the session pairing flow.

## Decisions

### 1. Treat connector configuration, access, and session attachment as separate concepts

Connector configuration remains application-scoped and owns credentials, authorization, transport lifecycle, and health. Pairing authorizes one external chat to attach to one existing session. The attachment owns pause state and completion-notification preference.

This follows the established service boundary while removing the singleton route from the hot path. It also avoids overloading session `source` metadata: a desktop-created session with an IM attachment remains desktop-created, while legacy IM-created sessions retain their source metadata.

Alternative considered: keep global routing and add a session override. This leaves two competing routes and makes it unclear which project receives a new message, so it is rejected.

### 2. Use a desktop-initiated, IM-completed pairing intent

Starting pairing creates a connector- and session-scoped intent with a short expiry. The native service returns the plaintext code once and stores only a salted code hash plus non-secret intent metadata. Connector ingress intercepts the reserved pairing command before normal deduplication and Agent routing, validates it with constant-time comparison, and consumes it transactionally when creating the binding.

The plaintext code must not enter persistent frontend state, logs, analytics, diagnostics, or error payloads. Pending intents are bounded per session and connector; creating a replacement intent invalidates the previous one. Expired and consumed intents are removed through bounded maintenance.

Alternative considered: ask the user to paste a platform chat id into the desktop UI. This is error-prone, leaks identifiers, and cannot prove control of the target chat, so it is rejected.

### 3. Evolve bindings into managed session attachments

The native persistence model extends binding records with lifecycle state, completion-notification preference, safe timestamps, and a credential-store reference for any opaque delivery handle needed after the inbound request ends. SQLite keeps connector kind, hashed external identity, session id, state, safe metadata, and credential reference; raw delivery identifiers or reply contexts are stored through the existing operating-system credential adapter when required by a platform.

Uniqueness rules are enforced transactionally:

- one `(connector, external chat)` may target only one session;
- one session may have only one active binding in the first version;
- rebinding requires a short-lived confirmation token returned only to the desktop caller;
- deletion of a session cascades or invalidates its attachment and removes binding-owned secure delivery state.

Existing `im_session_bindings` rows are migrated as active attachments. If legacy data contains multiple bindings for one session, the oldest remains active and additional records are retained as paused migration records so no identity association is silently discarded.

Alternative considered: use session source columns as the attachment. Those columns cannot represent pause state, pairing, notification preference, or desktop-created sessions with an attachment, so they remain origin metadata only.

### 4. Route only bound ordinary messages to Agent execution

Inbound processing keeps durable event deduplication and per-chat serialization. After connector admission:

1. Reserved pairing commands are handled by the pairing service and never become chat messages.
2. Ordinary text for an active binding executes through the existing session configuration.
3. Ordinary text for a paused binding receives a concise paused response without Agent execution.
4. Ordinary text without a binding receives localized pairing guidance without session creation or message persistence.

The connector runtime can therefore start without routing settings. Existing automatic final-response delivery remains unchanged for IM-originated turns.

Alternative considered: automatically create an inbox session for unbound traffic. A session without an explicit project or worktree would either be unusable or recreate the same global-routing ambiguity, so it is rejected for the first version.

### 5. Keep outbound scope narrow and origin-aware

Each binding has an opt-in completion-notification flag. A qualifying desktop-started execution may emit only a localized status containing safe session identity and terminal outcome. It does not include prompt content, assistant content, file paths, diagnostics, or secrets. IM-originated executions continue sending their final response to the originating chat and do not also emit a duplicate completion notification.

The execution source already distinguishes instant-message work; the notification dispatcher uses source metadata and terminal events to prevent loops and duplicate delivery. Delivery failure is logged with redacted safe codes and never reruns the Agent.

Alternative considered: mirror every desktop message and response. This creates privacy, duplication, formatting, and loop risks and is deferred to a separately specified capability.

### 6. Add runtime-neutral session-binding APIs

The frontend `ImService` gains typed operations equivalent to:

- list the selected session's safe binding view;
- begin, observe, cancel, and retry pairing;
- pause or resume the attachment;
- enable or disable completion notifications;
- request and confirm replacement;
- remove the attachment;
- subscribe to generation-aware pairing, binding, and connector-health changes.

The Tauri adapter alone invokes new communications commands. Rust validates session eligibility, connector health, pairing state, concurrency, persistence, and secure delivery storage. The Web/mock adapter implements deterministic in-memory transitions and synthetic pairing completion for component and browser tests; it does not claim to connect a real IM network.

Alternative considered: have the session information panel call the existing Tauri bridge directly. This violates the runtime boundary and leaves browser mode unusable, so it is rejected.

### 7. Place binding UX in a dedicated session IM pane with a responsive fallback

The information panel adds an `im` tab rather than expanding the basic-details pane. It shows configured connectors when unbound and safe attachment state when bound. Pairing code, expiry, instructions, cancellation, and retry occupy a focused transient surface. Destructive removal or replacement requires confirmation.

Because the current information panel is hidden at narrow widths, the session action menu opens the same binding surface in the responsive layout. Both surfaces use shared state and service hooks rather than duplicating business behavior.

Settings retains connector credentials, authorization, tests, access posture, lifecycle, and health. The routing form is replaced with concise guidance and an action that leads to an eligible session.

## Risks / Trade-offs

- [A pairing code is disclosed before expiry] → Scope it to one connector and session, use short expiry and single use, bound pending counts, never persist plaintext, and require desktop confirmation for replacement.
- [A platform requires a raw target for later notification] → Store only the minimum opaque delivery handle in the operating-system credential store and keep a reference plus hash in SQLite.
- [Legacy data violates new one-binding-per-session cardinality] → Migrate deterministically, keep one active, retain extras paused, and surface a safe remediation state.
- [Connector receives unbound message floods] → Apply bounded per-sender responses, existing transport pacing, and no Agent/session creation before pairing.
- [Notification events create duplicate outbound messages] → Gate on execution source and terminal message id and persist delivery idempotency before dispatch.
- [Web/mock behavior is mistaken for a real connection] → Label simulated pairing clearly while preserving the same typed transitions and error semantics.
- [Information panel gains another tab] → Keep the pane focused, use the responsive session-action entry, and validate both visual styles and narrow layouts.

## Migration Plan

1. Add pairing-intent and managed-attachment storage additively, including secure delivery references and bounded maintenance indexes.
2. Migrate existing `im_session_bindings` into active attachments without changing their session ids or session source metadata; retain unexpected duplicates as paused records.
3. Deploy native commands and both frontend adapters before switching the UI to the new service operations.
4. Remove `im_routing_settings` from connector startup validation and route unbound ordinary messages to pairing guidance.
5. Replace the settings routing form with connector-focused guidance and enable the session IM surfaces.
6. Keep legacy routing rows during the compatibility window so rollback can restore the previous runtime without reconstructing defaults.
7. On rollback, disable new pairing entry points, read migrated active bindings through the compatibility repository path, and leave additive tables and columns intact.
