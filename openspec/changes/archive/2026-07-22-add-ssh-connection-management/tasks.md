## 1. Native Data Model And Storage

- [x] 1.1 Add SSH connection domain/application models for profile metadata, auth mode, credential presence, validation, test status, and timestamps.
- [x] 1.2 Add an additive SQLite migration for `ssh_connections` and repository methods for list, create, update, delete, and test-status persistence.
- [x] 1.3 Add migration and repository tests proving SSH profile metadata persists correctly and password plaintext never appears in SQLite rows.
- [x] 1.4 Reuse or extend the existing native credential-store abstraction with SSH-specific credential references, replacement, preservation, and deletion behavior.

## 2. SSH Connection Test Runtime

- [x] 2.1 Select and isolate the first-version TCP reachability backend for host and port without starting an interactive shell.
- [x] 2.2 Implement bounded connection testing with timeout handling and normalized success/failure results.
- [x] 2.3 Update last test status, last connected timestamp, and redacted last error after each connection test.
- [x] 2.4 Add native tests for domain validation, repository persistence, password absence from SQLite, and redaction of SSH credential diagnostics.

## 3. Commands And Service Boundaries

- [x] 3.1 Add Tauri commands for list, create, update, delete, and test SSH connection operations behind Rust application services.
- [x] 3.2 Add frontend SSH connection types and a service boundary with Tauri and Web/mock adapters.
- [x] 3.3 Ensure React components consume only the frontend service boundary and never call Tauri `invoke()` directly.
- [x] 3.4 Add Web/mock SSH connection behavior that simulates credential presence without persisting submitted password plaintext.

## 4. Settings SSH Connection UI

- [x] 4.1 Register the SSH connection settings page in settings navigation with a stable icon while keeping About as the final entry.
- [x] 4.2 Build the SSH connection settings page with search, empty state, profile list, add/edit/delete controls, and test action feedback.
- [x] 4.3 Build localized form validation for name, host, port, user, default path, auth mode, key path, and password replacement.
- [x] 4.4 Add write-only password form behavior that shows configured state without rendering or resubmitting stored passwords.
- [x] 4.5 Add synchronized zh-CN and en locale keys and settings i18n coverage for the new page and dialogs.

## 5. Create Session Integration

- [x] 5.1 Extend remote workspace/session contracts with port-aware remote URI/display behavior while keeping SSH connection ids out of session snapshots.
- [x] 5.2 Update the create-session remote section to select an SSH connection, prefill host/user/port/default path, and allow path override.
- [x] 5.3 Support manually entered remote details with an optional save-as-connection path through the service boundary.
- [x] 5.4 Preserve existing manual remote session creation and known remote workspace behavior for users who do not save a connection.
- [x] 5.5 Verify through contracts and persistence tests that sessions keep remote workspace snapshots rather than live SSH connection references.

## 6. Logging And Security

- [x] 6.1 Extend unified log redaction for SSH password fields, credential references, private key paths, and bounded SSH test diagnostics.
- [x] 6.2 Add tests proving SSH password plaintext, private key paths, and credential references are not persisted to logs or SQLite rows.
- [x] 6.3 Ensure connection test errors shown in UI are concise and do not include raw credentials or full private key paths.

## 7. Verification

- [x] 7.1 Run `openspec validate add-ssh-connection-management --strict`.
- [x] 7.2 Run `npm run test`.
- [x] 7.3 Run `npm run build`.
- [x] 7.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 7.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.6 Run `openspec validate --specs --strict`.
