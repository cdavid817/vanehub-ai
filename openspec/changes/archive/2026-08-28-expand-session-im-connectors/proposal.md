## Why

Session-level IM authorization currently applies only to Feishu even though VaneHub already ships five independently managed connector transports. This leaves non-Feishu chats able to pair and submit work without the same explicit per-session opt-in, and the information panel cannot select those configured connectors.

## What Changes

- Apply deny-by-default session authorization to Telegram, DingTalk, WeCom, and personal WeChat as well as Feishu.
- Make the session information panel show connector-scoped access and let the user choose one healthy configured connector before pairing.
- Preserve the existing one-binding-per-session rule while keeping access state independent per connector and preventing stale access for one connector from authorizing another.
- Keep connector configuration, health, restart, and credential management independent from session access.
- Extend Web/mock, native, component, Playwright, and deterministic desktop coverage across the five stable connector ids without requiring live external credentials.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `im-connector-management`: Require connector-scoped session authorization for pairing and inbound delivery across every built-in connector.
- `im-session-binding-ui`: Add connector selection and connector-scoped access state to the session information panel.
- `desktop-runtime-verification`: Verify connector isolation and non-Feishu session opt-in through the native desktop boundary.

## Impact

Both desktop and Web/mock runtimes are affected. The typed `ImService` contract, Tauri and Web adapters, communications application service, session information panel, and their tests will change. React remains isolated from direct Tauri calls, transport credentials remain in the native secure store, and no connector protocol or external dependency is replaced.
