## Why

IM connector credentials and transport lifecycle are application-level concerns, but the current design also requires one global Agent and project path before any connector can run. This makes multi-project use confusing because new external chats depend on mutable global defaults while existing bindings silently retain their original session configuration.

## What Changes

- Separate global IM connector configuration and access control from session routing.
- Allow a configured connector to run without a global default Agent or project path.
- Add session-level IM management for pairing one external direct chat with an existing VaneHub session, pausing reception, enabling completion notifications, and removing the binding.
- Use a short-lived, single-use pairing code so the desktop session and external direct chat explicitly confirm the binding without exposing external identity values.
- Route bound inbound messages through the existing session's persisted Agent, workspace or worktree, model, permissions, history, and provider continuity.
- Stop automatically creating an Agent session for an unbound direct message; return a safe pairing instruction without executing the message.
- Restrict the first version to one active external-chat binding per session and one session per external chat, with explicit confirmation before rebinding.
- Preserve existing IM-created sessions and bindings during migration while removing global routing defaults from the primary setup flow.
- Keep automatic reply delivery for IM-originated turns, and make session completion notifications opt-in without mirroring all desktop conversation content.

## Capabilities

### New Capabilities

- `im-session-binding-ui`: Session information-panel behavior for pairing, inspecting, pausing, notifying, rebinding, and removing an IM attachment.

### Modified Capabilities

- `im-connector-management`: Decouple connector lifecycle from global routing, replace implicit first-message session creation with explicit pairing to an existing session, and define binding cardinality, migration, and notification behavior.
- `settings-im-management-ui`: Remove Agent and project routing as connector enablement prerequisites and focus the settings page on connector credentials, authorization, access, and lifecycle.

## Impact

- Desktop runtime: communications domain, SQLite schema and migration, connector lifecycle, inbound routing, pairing state, binding APIs, safe delivery metadata, and unified logging.
- Frontend: IM contracts and service interface, Tauri and Web/mock adapters, settings IM page, session information panel, responsive session actions, localization, and tests.
- Runtime boundaries remain intact: React uses service interfaces only; Tauri commands own native persistence and connector behavior; the Web/mock adapter exposes equivalent deterministic behavior.
- Existing `im_routing_settings` data becomes legacy migration input rather than a connector startup dependency. Existing `im_session_bindings` records remain valid.
- No new third-party runtime or UI dependency is required.
