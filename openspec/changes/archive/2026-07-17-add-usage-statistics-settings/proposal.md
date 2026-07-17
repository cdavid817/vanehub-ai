## Why

Users need a lightweight way to understand how much AI assistant usage has accumulated inside VaneHub sessions without leaving the settings center. The app already persists per-message input and output usage fields, so a first version can expose useful totals now while documenting the limits before deeper billing-grade accounting is added.

## What Changes

- Add a Usage Statistics page to the settings center for token usage summaries.
- Aggregate existing persisted message usage fields into total, input, output, session, and message counts across common time ranges.
- Keep the page available in both Tauri desktop and Web/mock runtimes through the frontend service boundary.
- Localize all user-visible copy in Simplified Chinese and English.
- Keep visual styling aligned with existing settings pages, shared page primitives, and both registered visual styles.
- Document first-version constraints: current native usage values are based on existing stored fields, real tokenizer accounting, provider/model breakdowns, cache tokens, request logs, and cost estimation are out of scope.

## Capabilities

### New Capabilities
- `usage-statistics`: Settings-center usage statistics for persisted session token usage, including first-version constraints and future extension points.

### Modified Capabilities
- `settings-center-ui`: Add Usage Statistics as a settings page with localized navigation, search placeholder, and token-first visual styling.
- `frontend-runtime-architecture`: Expose usage statistics through the frontend service interface and matching Tauri/Web adapters rather than direct runtime calls from React components.
- `native-runtime-architecture`: Add a read-only native command that aggregates token usage from SQLite message storage without moving database access into the frontend.

## Impact

- Frontend: settings page registry, new usage statistics page, i18n resources, service interface, Tauri adapter, Web/mock adapter, and focused tests.
- Backend: Rust command and query logic for SQLite usage aggregation.
- Documentation: design notes capturing first-version constraints and deferred billing-grade usage features.
- Runtime scope: both desktop runtime and Web/mock runtime.
- No new dependencies are expected.
