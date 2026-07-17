## Why

VaneHub's current Usage Statistics page sums character-count approximations stored on assistant messages, so it cannot distinguish provider-reported tokens from estimates or show how usage changes over time and across Agents. Upgrading the accounting model now gives users trustworthy monitoring inside VaneHub while preserving an explicit boundary from provider billing records.

## What Changes

- Capture provider-reported input, output, cache-read, and cache-creation token usage from supported CLI runtime events when available.
- Persist one normalized usage record per VaneHub assistant response, including Agent attribution, optional provider/model identity, accounting quality (`reported` or `estimated`), source, and occurrence time.
- Migrate existing message usage into explicitly estimated historical records without presenting those values as reported tokens.
- Replace the single mixed total with separate reported and estimated summaries, data-coverage counts, daily trend points, and per-Agent breakdowns.
- Make supported time ranges use consistent user-local calendar boundaries in both desktop and Web runtimes.
- Refresh usage data without blocking settings navigation and retain clear localized explanations that statistics cover VaneHub-managed sessions rather than provider billing or external-terminal history.
- Refine the Usage Statistics settings page using the existing semantic visual system for both `futuristic` and `minimal` styles, with synchronized Simplified Chinese and English copy.
- Add parser, migration, aggregation, adapter, UI, i18n, responsive, and dual-style regression coverage.
- Keep cost estimation, pricing configuration, request-detail logs, external CLI history scanning, and provider/model filtering outside this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `usage-statistics`: Separate reported tokens from estimates and add coverage, local-time ranges, daily trends, cache usage, and per-Agent aggregation for VaneHub-managed sessions.
- `settings-center-ui`: Upgrade the localized Usage Statistics page with trend and Agent breakdown surfaces that remain coherent in both registered visual styles.
- `frontend-runtime-architecture`: Expand the usage statistics service contract and keep matching Tauri and Web adapter behavior for richer monitoring data.
- `native-runtime-architecture`: Persist normalized usage records and expose bounded read-only aggregation through declared native commands.
- `session-runtime-management`: Extract and normalize usage from supported CLI runtime events, with explicit estimated fallback behavior when reported usage is unavailable.

## Impact

- Desktop runtime: Rust CLI output parsing, session completion accounting, SQLite migration and indexes, usage aggregation, command contracts, and unified diagnostic logging for non-sensitive parser failures.
- Web runtime: Mock usage records and aggregation matching the desktop service contract, including accounting quality and local-time semantics.
- Frontend: Usage statistics types, Agent service interface, Tauri/Web adapters, React Query data flow, settings page composition, semantic styling, and zh-CN/en resources.
- Data: Existing persisted message usage is retained as estimated history; new usage records avoid storing prompts, responses, credentials, or other message content.
- Boundaries: React continues to depend only on the frontend Agent service interface; SQLite and CLI-specific parsing remain in the Rust/native layer.
- Dependencies: No new runtime, state-management, charting, or UI-library dependency is expected; the trend visualization will use repository-native React/SVG and semantic design tokens.
