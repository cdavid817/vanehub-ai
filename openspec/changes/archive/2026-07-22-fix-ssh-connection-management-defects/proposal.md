## Why

The first SSH connection management implementation leaves existing desktop databases without the new remote workspace port columns and has several failure paths where persisted state and visible UI state diverge. These defects must be corrected before the change is committed so upgrades, credential handling, and session creation remain reliable.

## What Changes

- Move remote workspace port schema upgrades into the current migration so databases that already applied the original remote workspace migration receive the new columns.
- Add a realistic pre-SSH migration fixture that verifies upgrade behavior and preserves existing remote workspace records.
- Refresh SSH connection query state after both successful and failed connection tests.
- Include save-as-connection authentication fields in create-session submission eligibility and preserve actionable validation feedback.
- Compensate secure-storage writes and deletes when the corresponding SQLite mutation fails, preventing orphaned credentials or misleading partial failures.
- Add focused frontend and Rust regression tests for each corrected path.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-management`: Require an existing desktop database to gain remote workspace port columns during upgrade without losing historical session snapshots.
- `ssh-connection-management`: Require failed profile mutations and deletions to avoid leaving unmanaged secure-storage credentials.
- `settings-ssh-connection-ui`: Require failed connection tests to refresh persisted failure status and save-as-connection submission to validate required authentication input.

## Impact

The change affects the Rust SQLite migration and SSH connection application service, React Query cache handling on the SSH settings page, and create-session validation in both desktop and Web UI paths. Existing frontend service boundaries and Tauri command contracts remain unchanged; no new dependency is introduced.
