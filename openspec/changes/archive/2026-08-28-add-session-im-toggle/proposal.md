## Why

Users need an explicit, low-risk way to connect an existing VaneHub session to Feishu from the session information panel. The current IM surface exposes binding management whenever an eligible session is selected, which makes IM activity appear enabled before the user has deliberately opted in and does not define how inbound Feishu messages enter multi-Agent sessions.

## What Changes

- Add a session-scoped IM enablement switch to the information panel. It is off by default, reveals Feishu binding controls only after explicit opt-in, and preserves the user's choice through the native session state.
- Limit the first release to the existing Feishu connector; other built-in connector behavior remains unchanged.
- Define safe Feishu direct-message routing for both single-Agent sessions and multi-Agent sessions, reusing the session's existing routing and handoff semantics rather than creating an IM-specific dispatch control.
- Add deterministic WebdriverIO desktop coverage for IM opt-in, Feishu pairing/binding lifecycle, single-Agent delivery, multi-Agent routing, and disabled/error paths. External Feishu platform qualification remains separately reported until test credentials and a test tenant are supplied.

## Capabilities

### New Capabilities

- `feishu-session-im-integration`: Session-level Feishu opt-in and inbound delivery behavior for single-Agent and multi-Agent sessions.

### Modified Capabilities

- `im-session-binding-ui`: Require a default-off information-panel control before exposing session IM binding operations.
- `im-connector-management`: Constrain first-release session opt-in and Feishu delivery to explicit, enabled session bindings.
- `multi-agent-group-chat`: Define how IM-originated input enters the existing multi-Agent mention and handoff routing model.
- `desktop-runtime-verification`: Add a deterministic WebdriverIO desktop verification layer for Feishu session IM integration.

## Impact

This affects both desktop and Web/mock runtimes. React UI will consume typed IM operations through `AgentService`; Tauri calls, Feishu transport, secure credentials, session persistence, and unified redacted logs remain behind the native communications boundary. The Tauri and Web/mock runtime adapters must expose matching contracts. Existing WebdriverIO/Tauri test infrastructure will be extended without adding a second automation framework.
