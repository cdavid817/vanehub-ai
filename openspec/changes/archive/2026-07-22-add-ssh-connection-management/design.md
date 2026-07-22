## Context

VaneHub AI already supports remote workspace session metadata. Users can create a remote session by typing host, optional user, path, and display name, and the runtime persists a remote workspace snapshot plus a recent remote workspace history entry. That model is useful for durable session history, but it is not a good long-term place for reusable SSH connection configuration because it has no port, authentication, credential lifecycle, or test status.

The project also already has a native credential-store pattern for IM connectors. SSH password handling should reuse that shape: SQLite stores non-secret metadata and a stable credential reference, while the native credential store owns password plaintext. React must never receive stored passwords, and all native diagnostics must pass through unified log redaction.

## Goals / Non-Goals

**Goals:**

- Add durable SSH connection profiles with name, host, port, user, default path, authentication mode, key path, password credential presence, timestamps, and last test state.
- Add a Settings page where users can add, edit, delete, search, and test SSH connections.
- Let the create-session remote section select an SSH connection, prefill host/user/default path/port, override the session path, and optionally save manually entered connection details.
- Keep each session's remote workspace metadata as a snapshot so historical sessions survive connection edits or deletion.
- Store password credentials through the native credential store and never in SQLite, logs, operation results, Web mock persistence, or React state after submission.
- Implement a simple bounded connection test that validates TCP reachability to the configured SSH host and port without launching an interactive session.

**Non-Goals:**

- Do not run Agent CLIs over SSH in this version.
- Do not add an SSH-backed interactive Shell tab in this version.
- Do not add remote Files, Documents, Changes, or Git diff/status support in this version.
- Do not add SFTP browsing, remote file reads, remote project inspection, or remote worktree creation in this version.
- Do not persist private key contents or passphrases in this version; key authentication stores only the key path.
- Do not implement SSH config import, jump hosts, agent forwarding, host key management UI, or multi-hop connection routing in this version.
- Do not rewrite existing remote session snapshots when SSH connections are edited or deleted.

## Decisions

### Decision: Model SSH connections separately from known remote workspaces

Create an `ssh_connections` data model rather than extending `known_remote_workspaces` into a configuration table. Known remote workspaces are recency history keyed by URI; SSH connections are user-managed profiles with auth, port, test state, and lifecycle semantics.

Alternative considered: add auth fields to `known_remote_workspaces`. That would mix passive history with editable credentials and make deletion semantics ambiguous.

### Decision: Store passwords in native credential storage

SQLite stores only a `credential_ref` and `has_password`/auth metadata. Password plaintext is accepted only as a write-only mutation input, immediately handed to the Rust credential-store adapter, and zeroized where practical after use. Existing passwords are preserved when the edit form submits no replacement password.

Alternative considered: encrypt passwords into SQLite. That introduces key-management problems and weakens the existing project convention that OS credential storage owns secrets.

### Decision: Keep session remote workspace snapshots independent

Session creation derives a `remoteWorkspace` snapshot from the selected connection plus the effective session path. The session stores host, user, path, display name, URI, and port if the contract is extended to include it. It should not rely on a live connection id for historical display.

Alternative considered: store only `sshConnectionId` on sessions. That makes old sessions fragile when profiles are renamed or deleted.

### Decision: Implement a first-version TCP reachability test

The first-version test opens a bounded TCP connection to the configured SSH host and port after validating that the selected authentication mode has the required local credential metadata. It does not authenticate, run remote commands, or launch a shell. The test updates last status, last connected timestamp on success, and a concise redacted error on failure.

Alternative considered: test by starting a remote shell or listing files. That crosses into remote execution and file browsing, which are explicitly future work.

### Decision: Preserve Web/mock parity without real credentials

The Web adapter should support equivalent CRUD and session-selection behavior using in-memory or browser-local mock state, but it must not pretend to store secrets securely. Password submissions in Web mode should set credential presence only in mock state and must not persist plaintext.

Alternative considered: disable the page in Web mode. That would weaken UI coverage and runtime contract parity.

### Future Technical Extensions

The first-version non-goals should be preserved as explicit extension points:

- Remote Agent CLI launch: add a Rust-owned remote runtime adapter that maps existing Agent Terminal lifecycle events to SSH process execution.
- Remote Shell tab: implement an SSH-backed shell service behind the existing shell contract with resize/input/state events.
- Remote Files/Documents tabs: add bounded SFTP/SSH file listing and reads with the same limits as local workspace tabs.
- Remote Git status/diff: run normalized Git commands remotely and map output to existing Git status/diff frontend contracts.
- Full SSH authentication tests: add password/key authentication, timeout classification, and a no-side-effect probe command once a Rust SSH client or system-ssh strategy is selected.
- Host key management: introduce explicit known-host verification, trust prompts, and audit logging before accepting unknown host keys.
- Advanced SSH topology: support SSH config import, jump hosts, agent forwarding, passphrases, and per-profile environment setup in later changes.

## Risks / Trade-offs

- Password leakage through DTOs, logs, or tests -> Keep passwords write-only, use credential references, extend redaction tests, and assert persisted SQLite rows never contain submitted secrets.
- Connection tests hang or prompt interactively -> Run tests with bounded timeout, non-interactive options, and clear failure mapping.
- Key paths reveal sensitive local filesystem structure -> Treat key paths as sensitive diagnostic context and redact or abbreviate them in logs.
- Users expect remote sessions to execute remotely after selecting SSH -> Settings copy and unsupported runtime errors must make first-version limits clear without blocking future support.
- SSH tooling behavior differs by OS -> Keep the first test narrow, isolate command construction in Rust infrastructure, and cover Windows path/argument handling.
- Web/mock credential behavior can mislead users -> Web mode should show that secure password persistence is unavailable or simulated.

## Migration Plan

- Add an additive SQLite migration for `ssh_connections` with no seed data.
- Reuse the existing native credential-store abstraction for password storage where possible; add only SSH-specific account naming and cleanup behavior.
- Existing sessions and `known_remote_workspaces` remain valid and unchanged.
- Rollback is data-compatible: disabling the feature leaves the new table and credential refs unused, while existing remote sessions continue to display from their snapshots.

## Open Questions

- Should full SSH authentication tests use the system `ssh` binary first, or introduce a Rust SSH client dependency for password authentication?
- Should key paths be fully redacted in all persisted logs, or shown as a basename-only diagnostic?
- Should the create-session remote form save manually entered password credentials when "save as connection" is checked, or require saving credentials from the settings page only?
