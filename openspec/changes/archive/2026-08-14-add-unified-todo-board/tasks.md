## 1. Contracts and persistence

- [x] 1.1 Add shared TypeScript work-item, source-link, filter, mutation, and projection contracts.
- [x] 1.2 Add additive SQLite migrations for work items, source links, Session lineage, and Scheduled Task run history with indexes and migration tests.
- [x] 1.3 Implement the native work-board domain validation, repository CRUD, sparse ordering, archive lifecycle, and unavailable-source projection.
- [x] 1.4 Implement transactional automatic reconciliation for manual Sessions, aggregated Plans, and Scheduled Tasks with child-Session suppression and idempotency tests.
- [x] 1.5 Persist Session lineage for direct, Plan-attempt, and Scheduled Task run creation paths and expose it through Session contracts.
- [x] 1.6 Persist and query bounded Scheduled Task run history for success, failure, skip, and backfill outcomes.
- [x] 1.7 Add global Plan summary discovery that aggregates drafts, versions, and latest runs by stable Plan id.

## 2. Service boundaries and adapters

- [x] 2.1 Define a runtime-neutral WorkBoardService covering list/reconcile, create, update, move, link, archive, restore, and permanent delete operations.
- [x] 2.2 Add Rust/Tauri work-board commands and a Tauri frontend adapter without exposing invoke calls to React components.
- [x] 2.3 Add a contract-compatible Web/mock adapter with automatic reconciliation and deterministic in-memory ordering.
- [x] 2.4 Add adapter conformance, contract drift, native repository, and error-path tests.

## 3. Todo Board user interface

- [x] 3.1 Add a lazy-loaded full-screen Todo Board destination and active activity-bar navigation state.
- [x] 3.2 Implement responsive board columns and cards showing board metadata separately from live multi-source projections.
- [x] 3.3 Implement manual work creation and editing, stage movement, ordering, priority, project, and due-date controls through WorkBoardService.
- [x] 3.4 Implement source, stage, priority, project, archive, and text filters that keep multi-source work as one card.
- [x] 3.5 Implement archive, restore, and permanent-delete flows without mutating linked source records.
- [x] 3.6 Add pointer movement plus equivalent keyboard and explicit non-drag card movement controls.
- [x] 3.7 Add complete localized strings for all supported locales and update localization guardrail coverage.

## 4. Verification

- [x] 4.1 Add unit and component coverage for reconciliation, filtering, navigation, compact layout, accessibility, card movement, and archive behavior.
- [x] 4.2 Add Playwright coverage for the Todo Board workflow in Web mode.
- [x] 4.3 Run `npm run lint:ci`, `npm run test`, `npm run build`, and all applicable coverage, policy, version, contract, and Playwright checks.
- [x] 4.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, strict Clippy, Rust tests, and Cargo check.
- [x] 4.5 Run `openspec validate add-unified-todo-board --strict` and `openspec validate --specs --strict`.
