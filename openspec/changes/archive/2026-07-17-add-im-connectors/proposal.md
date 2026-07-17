## Why

VaneHub currently requires users to remain in the desktop chat UI to interact with coding Agents, which prevents developers from delegating work and receiving final results through the IM tools they already monitor. Adding a desktop-owned IM gateway makes VaneHub reachable from five common messaging platforms while keeping Agent execution, session history, credentials, and diagnostics under the existing local runtime boundary.

## What Changes

- Add independently configurable connectors for Feishu, Telegram, DingTalk, WeCom, and personal WeChat.
- Support a first-version text-only direct-message flow: receive one external message, route it to a dedicated VaneHub session, and send the completed Agent response back once as a final-only reply.
- Add global IM routing settings for the default Agent and default project, with one automatically created session per external chat.
- Add connector lifecycle management, connection health, reconnection, inbound deduplication, per-chat serialization, and explicit enable/disable and connection-test actions.
- Store connector secrets in the operating-system credential store and persist only non-secret configuration, credential references, bindings, offsets, and status metadata in SQLite.
- Add an IM settings page with synchronized zh-CN and en resources and equivalent behavior in the `futuristic` and `minimal` visual styles.
- Keep the desktop process available after the main window is closed by hiding it to the system tray; an explicit tray quit action stops connectors and exits the process.
- Mark personal WeChat support as experimental and provide QR authorization, session-expiry status, and reauthorization behavior.
- Keep the browser runtime usable through a complete Web/mock adapter without claiming that browser-only mode establishes real platform connections.
- Route all connector diagnostics through unified logging with credential and message-content redaction.

## Capabilities

### New Capabilities
- `im-connector-management`: Connector configuration, secure credentials, platform lifecycle, direct-message routing, session binding, deduplication, serialization, health, and outbound final delivery for the five supported platforms.
- `desktop-background-lifecycle`: Close-to-tray, window restoration, explicit quit, and graceful connector shutdown behavior for the Tauri desktop runtime.

### Modified Capabilities
- `settings-center-ui`: Add the localized, service-backed IM settings page with connector status, routing, credential, test, and authorization controls in both registered visual styles.
- `session-runtime-management`: Allow external chats to create and reuse dedicated sessions and route messages through the same Agent execution and persistence behavior as desktop chat.
- `frontend-runtime-architecture`: Add matching Tauri and Web/mock IM service adapters and keep React isolated from direct Tauri calls.
- `native-runtime-architecture`: Host long-running connector tasks, shared chat-runtime entry points, credential-store integration, and non-blocking lifecycle commands in the Rust runtime.
- `unified-log-management`: Define connector-specific diagnostic context and mandatory redaction for credentials, authorization artifacts, external identifiers, and message content.
- `main-layout-ui`: Identify IM-created sessions and their source platform in the existing session navigation without creating a separate conversation system.

## Impact

- Desktop runtime: new Rust IM domain modules, connector background tasks, SQLite migrations, operating-system credential-store dependency, tray menu and window lifecycle handling, and refactoring of chat execution into a reusable internal service.
- Frontend: new IM contracts, service interface, Tauri adapter, Web/mock adapter, settings page modules, session source metadata, translations, and tests.
- Platform APIs: Feishu WebSocket, Telegram Bot API long polling, DingTalk Stream, WeCom intelligent-bot WebSocket, and personal WeChat iLink Bot long polling.
- Security and operations: secrets leave the general settings store, connector logs use the unified logging service, and connector shutdown/restart behavior becomes part of application lifecycle.
- No breaking change is intended for existing desktop chat, session storage, or Web/mock workflows.
