## Why

Remote workspace creation currently requires users to re-enter SSH host, user, and path details in the create-session dialog. As remote workflows become a first-class session target, users need reusable SSH connection profiles with safe credential handling and a simple way to verify that a profile can connect before using it for a session.

## What Changes

- Add a service-backed SSH Connection settings page for creating, editing, deleting, searching, and testing SSH connection profiles.
- Store SSH profile metadata in SQLite while storing password credentials through the native secure credential store; key authentication stores only a key path, not key contents.
- Extend the create-session remote workspace section so users can choose an existing SSH connection, override the remote path for the new session, or manually enter a temporary remote target and optionally save it as a connection.
- Preserve historical session behavior by continuing to store a snapshot of remote workspace metadata on each session; deleting or editing an SSH connection does not rewrite existing sessions.
- Include a simple first-version SSH connection test that performs bounded TCP reachability to the configured SSH host and port with redacted diagnostics; full SSH authentication and no-side-effect remote probes are documented as follow-up work.
- Document follow-up remote capabilities that are intentionally out of scope for the first version.

## Capabilities

### New Capabilities

- `ssh-connection-management`: Covers durable SSH connection profiles, safe credential storage, CRUD operations, and simple connection testing.
- `settings-ssh-connection-ui`: Covers the settings-center SSH connection management page and localized form behavior.

### Modified Capabilities

- `session-management`: Remote session creation can use a selected SSH connection while still persisting session-local remote workspace snapshots.
- `settings-center-ui`: The settings navigation includes SSH connection management as a first-class settings page.
- `unified-log-management`: SSH connection tests and credential handling redact passwords, key paths where required, and sensitive command diagnostics before persistence.

## Impact

- Affects both desktop runtime and Web/mock runtime service contracts.
- Extends `src/services/agent-service.ts`, `src/services/tauri-agent-client.ts`, and `src/services/web-agent-client.ts` or introduces a parallel frontend SSH service boundary with matching Tauri/Web adapters.
- Adds Rust/Tauri commands and application/domain/infrastructure modules for SSH connection persistence, credential store access, and connection testing.
- Adds an additive SQLite migration for SSH connection metadata; password plaintext must never be stored in SQLite or returned to React.
- Adds settings navigation, page components, validation, and synchronized zh-CN/en locale keys.
- Adds focused frontend, Web adapter, Rust domain/application/infrastructure, migration, redaction, and OpenSpec validation coverage.
