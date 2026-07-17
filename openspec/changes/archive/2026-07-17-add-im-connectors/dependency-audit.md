# IM Dependency Audit

## Selected baseline

| Concern | Selection | Compatibility rationale |
|---|---|---|
| OS credential storage | `keyring` 4.1.5, default `v1` feature | Uses Apple Keychain, Windows Credential Store, and the Secret Service backend on Unix; MIT OR Apache-2.0; MSRV 1.88, below the repository Rust 1.97 toolchain. |
| Async WebSocket | `tokio-tungstenite` 0.30 with rustls web PKI roots | Async `Stream`/`Sink` API, cross-platform TLS without an OpenSSL packaging dependency; MSRV 1.85. |
| Async adapter contracts | `async-trait` 0.1 and `futures-util` 0.3 | Keeps platform transports object-safe and testable while using the existing Tokio runtime. |
| Binary protocol frames | `prost` 0.14 | Supports the framed protobuf payload used by Feishu long connections without a build-time `protoc` dependency when message structs are declared in Rust. |
| QR rendering | `qrcode` 0.14.1 SVG renderer plus `base64` 0.22 | Generates a short-lived in-memory data URL without filesystem writes or a heavyweight image codec. |
| Retry jitter and secret cleanup | `rand` 0.9 and `zeroize` 1 | Bounded retry jitter and explicit cleanup for temporary secret buffers. |
| System tray | Tauri 2 `tray-icon` feature | Uses the existing desktop runtime and packaged application icon on Windows, macOS, and Linux. |

## Platform SDK decision

- Feishu has a new third-party Rust crate, but it is young and its websocket feature adds a separate HTTP/TLS stack. The connector will keep the protocol behind VaneHub's transport trait instead of making that crate part of the public architecture.
- Telegram uses the documented Bot API directly through the existing `reqwest` stack.
- DingTalk Stream, WeCom intelligent-bot WebSocket, and personal WeChat iLink do not have a suitable first-party Rust SDK in the audited dependency set. Their minimum protocol clients remain private adapters with recorded-fixture tests.
- The Clowder Node SDK implementations remain behavior references only. VaneHub does not launch or package a Node connector process.

## Packaging constraints

- No selected dependency requires a separately installed Node runtime, Redis service, OpenSSL runtime, or public webhook server.
- Linux secure storage requires a working desktop Secret Service. An unavailable store is a surfaced configuration error; plaintext fallback is prohibited.
- WebSocket proxy support must use the existing VaneHub proxy policy and a connector-owned tunnel path; it cannot assume `connect_async` automatically consumes `reqwest` proxy settings.
- Personal WeChat remains experimental and disabled until explicit QR authorization and the release access review are complete.

