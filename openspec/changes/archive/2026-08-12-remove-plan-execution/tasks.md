## 1. Frontend Product Surface

- [x] 1.1 Remove the Plan activity-bar destination, lazy Plan Center mounting, and Plan-specific navigation tests while preserving all retained destinations.
- [x] 1.2 Remove Plan Center components, Plan frontend types, polling, Tauri/Web adapters, service contracts, and their tests.
- [x] 1.3 Remove Plan-specific localization keys and verify no frontend source or test imports the retired modules.

## 2. Native Runtime

- [x] 2.1 Remove Plan Tauri commands, command registry entries, runtime bootstrap assembly, managed state, and command-contract fixtures.
- [x] 2.2 Remove the `task_orchestration` bounded context and Plan-only OnePiece, session, operation, workspace, logging, and observability APIs after checking retained consumers.
- [x] 2.3 Preserve the shipped Plan migration identifier and inert legacy schema application, and update migration tests to document compatibility without live Plan runtime code.

## 3. Contract and Repository Cleanup

- [x] 3.1 Remove Plan-specific generated/schema contracts, test fixtures, and documentation references outside immutable archives while retaining chat and CLI Plan Mode behavior.
- [x] 3.2 Run repository-wide searches proving no live Plan execution UI, command, adapter, or task-orchestration references remain.
- [x] 3.3 Validate `remove-plan-execution` strictly and validate all main specifications strictly.

## 4. Full Verification

- [x] 4.1 Run `npm run lint:ci`, `npm run test`, `npm run build`, and UI coverage/E2E checks required by the navigation change.
- [x] 4.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, strict Clippy, Rust tests, and Rust check.
- [x] 4.3 Run the additional repository policy, coverage, version, contract, documentation, and OpenSpec validation commands required by the affected files, then record final results.
