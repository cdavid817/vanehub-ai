## Why

A registered native/API agent (`launch_kind = "api"`) can only ever be created — `register_api_agent` has no counterpart to edit its config, rotate a leaked or expiring API key, or remove one entirely. Once registered, a mistake (wrong model id, wrong base URL) or a routine key rotation currently requires re-registering under a new id, orphaning the old one and its history rather than correcting it in place.

## What Changes

- Add `update_api_agent`: edit an existing API agent's `displayName`, `modelId`, `baseUrl`, and/or stored API key. `provider` and `interfaceFormat` are immutable after registration (see design.md).
- Add `delete_api_agent`: remove a registered API agent and its stored credential, **rejecting** the delete with a clear, itemized error if any session, message, memory, usage record, or Loop worker/verifier assignment still references it. No cascading deletes.
- Extend the Agents settings page with edit and delete affordances per registered API agent, alongside the existing registration form.
- Full Web/mock parity for both new operations.

## Capabilities

### New Capabilities
- `agent-lifecycle-management`: editing, deleting, and key-rotation for registered native/API agents, and the referenced-entity guard that protects a delete from silently orphaning history.

### Modified Capabilities
(none — no existing `openspec/specs/` capability currently documents API agent registration to extend; the 5 prior native-agent phases were implemented and merged but never archived, so there is no base spec to layer requirements onto yet)

## Impact

- **Desktop runtime**: `contexts::agent_runtime` (application service, `ApiAgentGateway`/`ApiCredentialPort` ports, SQLite repository), two new Tauri commands, `commands/registry.rs`.
- **Web runtime**: `web-agent-client.ts` mock parity for both operations.
- **Frontend**: `agent-service.ts`, `tauri-agent-client.ts`, `agents-page.tsx` (or a small sibling component) gain edit/delete UI.
- **No breaking changes**: purely additive facade methods and commands; `register_api_agent` and its existing callers are untouched.
