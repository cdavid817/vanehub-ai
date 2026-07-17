## Context

VaneHub already persists chat messages in SQLite and stores optional per-message usage as `token_input` and `token_output`. The frontend model mirrors this as `ChatMessage.tokenUsage`, and the Web/mock adapter produces compatible mock values.

The requested feature is a settings-center usage statistics page. The first implementation should provide useful totals without importing the larger `cc-switch` usage dashboard architecture. That reference app tracks request logs, providers, models, pricing, cache tokens, and protocol-specific accounting; VaneHub does not yet persist those dimensions.

## Goals / Non-Goals

**Goals:**
- Add a localized Usage Statistics settings page.
- Summarize total, input, and output usage from persisted message usage fields.
- Show bounded first-version metrics: counted sessions, counted messages, and supported time ranges.
- Keep React components behind the existing Agent service boundary.
- Keep SQLite aggregation in the Rust/Tauri layer.
- Preserve Web/mock runtime behavior through a matching adapter implementation.
- Document known accounting constraints for future work.

**Non-Goals:**
- No billing-grade tokenizer integration.
- No cost estimation or pricing table.
- No provider, model, app, cache-read, cache-write, or request-log breakdowns.
- No trend chart, request detail drawer, or live usage event bridge.
- No schema migration for new usage tables.
- No attempt to reinterpret historical rows beyond summing existing `token_input` and `token_output`.

## Decisions

### Decision: Use existing message usage fields for first-version aggregation

The first version will aggregate the existing `messages.token_input` and `messages.token_output` columns. This avoids a new schema and exposes current persisted data immediately.

Alternative considered: import a `cc-switch`-style request log and rollup model. That would create a broader backend subsystem before VaneHub has persisted provider/model/pricing dimensions, so it is deferred.

### Decision: Add a service method to `AgentService`

Usage statistics will be exposed as `getUsageStatistics(input)` on the frontend service boundary. The Tauri adapter calls a declared command, and the Web adapter aggregates mock in-memory messages. React pages will not call Tauri APIs directly.

Alternative considered: create a standalone usage service. That may become appropriate when usage accounting grows into request logs and billing data. For the first version, usage is derived from chat sessions/messages already owned by the Agent service boundary.

### Decision: Keep native aggregation bounded and synchronous

The native command will run a small read-only aggregate SQL query over indexed message/session data. It returns directly as a bounded request/response command. If future usage reporting adds heavy rollups, filesystem scans, or remote pricing refreshes, it should move to backend-managed asynchronous operations.

### Decision: Represent documented limitations in the UI and docs

The page will display a localized note that values come from VaneHub's stored message usage fields and may not match provider billing. This same limitation is captured here so future work can replace the data source intentionally.

## Risks / Trade-offs

- [Risk] Existing native `send_message` currently records character-count approximations rather than tokenizer output. -> Mitigation: label the first version as based on stored usage fields and document true tokenizer accounting as future work.
- [Risk] Historical rows without token usage will be excluded from token totals. -> Mitigation: show counted message/session totals and allow zero totals without treating them as errors.
- [Risk] Users may expect `cc-switch`-level costs and provider/model breakdowns. -> Mitigation: keep the first page compact and list unsupported dimensions in localized explanatory copy.
- [Risk] Aggregate queries can grow with large message history. -> Mitigation: use SQL aggregates with time filtering and avoid loading message bodies into the frontend.

## Migration Plan

No database migration is required. Existing databases already have the `messages` table and usage columns from the chat messages migration.

Rollback is removing the settings page registration, service method, adapter implementation, and native command registration. No persisted data must be reverted.

## Future Work

- Replace approximate stored values with model/provider tokenizer output or upstream-reported token usage.
- Persist provider id, model id, request status, cache-read/cache-write tokens, and cost inputs per assistant response.
- Add request logs, trend charts, provider/model breakdowns, export, and configurable retention.
- Add pricing metadata and billing-grade cost estimates once usage rows carry reliable model identity.
- Add incremental rollups if aggregate queries become expensive on large histories.
