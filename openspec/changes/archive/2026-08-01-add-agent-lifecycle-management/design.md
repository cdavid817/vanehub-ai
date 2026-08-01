## Context

`agents(id, display_name, provider, launch_kind, launch_command, launch_url, executable_name, managed_sdk_dependency_id, model_id, interface_format, base_url)` (base columns in `platform/database/migrations.rs`'s `apply_initial_schema`, `model_id`/`interface_format`/`base_url` added later by `agent_runtime::infrastructure::schema.rs`). `ApiAgentGateway::register` (`infrastructure/sqlite_repository.rs`) is the only thing that ever writes a `launch_kind = 'api'` row today; nothing updates or deletes one.

`PRAGMA foreign_keys = ON` is enforced on every connection (`platform/database/mod.rs`, guarded by a regression test). Five tables reference `agents(id)` without `ON DELETE CASCADE`: `sessions.agent_id` (NOT NULL), `agent_memories.agent_id`, `usage_records.agent_id` (NOT NULL), `loop_definitions.worker_agent_id`, `loop_definitions.verifier_agent_id`. A sixth, `skill_api_agent_bindings.agent_id`, has no database-enforced FK at all — deleting a referenced agent would silently orphan its rows rather than error.

`ApiCredentialPort` (`store`/`fetch`/`remove`, OS-keychain-backed) already has everything key rotation and deletion need; only `remove` needs a second call site (it currently fires exactly once, as registration-failure rollback).

## Goals / Non-Goals

**Goals:**
- Let a user correct or rotate a registered API agent's config without re-registering under a new id.
- Let a user remove a registered API agent, with the delete failing loudly and specifically (not silently, not partially) when doing so would orphan other data.
- Match `register_api_agent`'s existing validation and error conventions exactly, so the three operations feel like one coherent facade rather than a bolt-on.

**Non-Goals:**
- Editing CLI-launch-kind agents — they're auto-detected, not user-configured this way.
- Cascading delete of a referenced agent's history — explicitly rejected (see Decision 2).
- Any change to how a *new* agent is registered.

## Decisions

### 1. `provider` and `interfaceFormat` are immutable after registration; `displayName`, `modelId`, `baseUrl`, and the API key are editable

**Why:** `interfaceFormat` (`anthropic` vs `openai-compatible`) selects which wire-format module (`anthropic_provider.rs` vs `openai_compatible_provider.rs`) a generation uses — it's a structural choice, not configuration, and every stored session/message for this agent implicitly assumes it hasn't changed. `provider` is a free-text label with no behavioral effect, but changing it after the fact reads as "this became a different agent," which conflicts with keeping the same id and history. Both are cheap to work around by registering a new agent instead, which is the correct move when the *kind* of agent is really changing. `displayName`/`modelId`/`baseUrl` are pure configuration with no structural coupling to stored history — safe to edit in place.

**Alternative considered:** allow editing everything, including `interfaceFormat`. Rejected — a mid-history wire-format switch would make every `history_to_turns`/`build_request_body` call for that agent's *existing* sessions silently reinterpret old turns under the new format's assumptions, with no migration path for what's already stored.

### 2. Delete rejects with an itemized reference count rather than cascading

**Why:** user decision this session — delete is irreversible, and an agent that's actually been used typically has session/message/memory/usage history a silent cascade would destroy without a distinct confirmation step this phase doesn't build. Rejecting with a specific, actionable count (e.g. "3 sessions, 12 memories still reference this agent") tells the user exactly what stands in the way, matching `CommandError::validation`'s existing role for user-actionable rejections (e.g. `register_api_agent`'s own field-validation errors).

**Query shape:** one `SELECT` per referencing table (`sessions`, `agent_memories`, `usage_records`, and `loop_definitions` checked against both `worker_agent_id` and `verifier_agent_id`) inside the same transaction as the eventual delete, each contributing a `(label, count)` pair; if any count is non-zero, return `CommandError::validation` listing every non-zero label before anything is deleted. `skill_api_agent_bindings` (no DB-enforced FK) is deleted unconditionally as part of the same transaction — it's a many-to-many binding row, not history, so unbinding a to-be-deleted agent has no data-loss implication distinct from the delete itself.

**Alternative considered:** cascade-delete everything transactionally. Rejected per the standing user decision (Decision made this session, not re-litigated).

### 3. Key rotation is a dedicated `newApiKey: Option<String>` field on the same `update_api_agent` call, not a separate command

**Why:** rotating a key and editing other fields are both "change this agent's config" from the user's perspective, and sharing one command avoids two near-identical validation/error paths. `newApiKey: None` leaves the stored credential untouched; `Some(key)` calls `credentials.store()` with the new value, overwriting the old one in place (the OS keychain adapter already treats `store` as an upsert — confirmed by `register_api_agent` never checking for an existing entry before its own first `store` call).

**Alternative considered:** a separate `rotate_api_agent_key` command. Rejected as needless surface-area duplication — the frontend form already shows the API key as one field among several; splitting it into a separate round trip has no user-facing benefit.

### 4. `update_api_agent` re-validates exactly like `register_api_agent`, minus identity fields

**Why:** consistency — a `baseUrl`-required-for-`openai-compatible` rule that only applies at creation and not at edit time would let a user edit their way into an invalid config that registration itself would have refused. Validates: non-empty `displayName`/`modelId`; `baseUrl` required when the *agent's existing* `interfaceFormat` is `openai-compatible` (read from the stored row, since `interfaceFormat` isn't part of the update payload per Decision 1).

## Risks / Trade-offs

- **[Risk]** The reference-check-then-delete isn't fully race-free against a concurrent write creating a new session for this agent between the check and the delete. **Mitigation:** both run inside one SQLite transaction with the existing busy-timeout/serialized-writer behavior this app already relies on elsewhere; the window is a single connection's transaction lifetime, and a concurrent create racing a delete of the same agent is an extreme edge case with no existing handling precedent elsewhere in this codebase either.
- **[Trade-off]** No UI path to force a cascade later if a user genuinely wants to purge an agent and its entire history. **Mitigation:** out of scope for this phase per the standing decision; revisit only if this friction proves real in practice (existing sessions can still be deleted individually first, via `delete_session`, to unblock the agent delete).

## Migration Plan

Purely additive: two new facade methods, two new Tauri commands, no schema changes (both operations use `agents`' and `skill_api_agent_bindings`' existing columns). No changes to `register_api_agent` or any other existing call path.
