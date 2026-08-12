## Context

See `proposal.md` for motivation. Session chat configuration is serialized as JSON in `sessions.chat_preferences`; its `permission_mode` value currently reaches CLI chat mapping directly, while Agent Terminal separately looks up the principal policy and applies a second mapping. Native OnePiece uses the same session field only to select a reduced plan-mode tool catalog. CLI profile definitions also expose security controls that overlap both paths.

The implementation must preserve the React service boundary, keep SQLite and process construction in Rust, and provide deterministic Web/mock behavior without claiming to enforce a local process.

## Goals / Non-Goals

**Goals:**

- Make the Agent policy template the single durable safety ceiling for all execution paths.
- Represent session-local intent independently as `inherit`, `plan`, or `execute`.
- Resolve one effective `readonly`, `ask`, or `allow` posture before native tool selection or CLI argument construction.
- Remove editable CLI-profile fields that can change execution permission, approval, or sandbox posture.
- Expose the resolved posture through the shared desktop/Web service contract.
- Reset old persisted chat snapshots instead of maintaining compatibility code.

**Non-Goals:**

- Translating arbitrary native CLI prompts into VaneHub `ApprovalCard` requests; that requires a separate provider-protocol change.
- Changing the four policy-template meanings, remembered grants, MCP Ask floor, or Claude hook mappings.
- Mutating already-running generations or terminal processes when policy changes.

## Decisions

### 1. Resolve a provider-neutral effective policy before provider mapping

The Rust runtime will introduce provider-neutral `SessionExecutionMode` and `EffectiveExecutionPolicy` types. Resolution is a complete matrix:

| Agent template | inherit | plan | execute |
|---|---|---|---|
| readonly | readonly | readonly | readonly |
| standard | ask | readonly | ask |
| trusted | allow | readonly | allow |
| yolo | allow | readonly | allow |

Provider adapters receive only the effective policy and translate it into catalog-legal CLI selections/environment. This avoids ordering bugs where either a message override or saved profile can undo a policy override.

Alternative considered: apply session overrides and policy overrides sequentially. Rejected because either ordering allows one axis to accidentally relax the other and duplicates the composition rule across launch paths.

### 2. Use one resolver but preserve enforcement mechanisms

Managed CLI launches enforce the effective posture through provider-native flags and environment. Claude Code additionally retains its authenticated action-level hook. Native OnePiece continues to use the permissions decision pipeline; `plan` selects the reduced tool catalog, while `inherit` and `execute` expose the normal catalog and let the Agent template resolve each gated action.

Alternative considered: force every CLI action through `ApprovalBroker`. Rejected for this change because most CLI chat protocols do not expose a stable structured approval callback. Unsupported Ask behavior must fail closed or use the provider's supported noninteractive posture rather than simulate an approval channel.

### 3. Security selections are runtime-owned

Execution, approval, automatic-approval, and sandbox definitions will be removed from editable CLI profile catalogs. Their provider mappings remain internal to the runtime policy adapter. Ordinary selections such as model, effort, agent/persona, and browser integration keep their current precedence.

Alternative considered: keep security fields visible but ignore them. Rejected because editable controls with no launch effect are misleading and leave two apparent sources of truth.

### 4. Make the request contract intentionally incompatible

TypeScript, Tauri DTOs, Rust application/domain models, serialized JSON, and event payloads will rename `permissionMode` / `permission_mode` to `executionMode` / `execution_mode`. Only `inherit`, `plan`, and `execute` are valid. Deserialization of old snapshots fails and the repository treats them as absent; a database migration also sets existing `chat_preferences` values to `NULL`, so all sessions derive fresh defaults with `inherit`.

No parser aliases, serde aliases, union members, or value translations will be added. Requests using the old field fail normal contract deserialization or validation.

Alternative considered: reuse the old field name with new values. Rejected because it would retain the conceptual ambiguity the change is intended to remove.

### 5. Backend owns effective-behavior reporting

The session chat-configuration response will include a projection containing the Agent policy template and effective behavior. Tauri obtains it from the Rust resolver. Web/mock maintains mock principal assignments and applies the same explicit matrix in its adapter. React renders the response and never derives policy precedence itself.

The selector is renamed to “Execution mode” / “运行模式” and presents Inherit, Plan, and Execute. A compact hint states the Agent policy and resolved behavior. Agent Terminal surfaces continue to state that policy changes apply to the next launch.

### 6. Existing processes retain launch-time policy

Policy is resolved once per generation/process spawn and copied into that execution request. No attempt is made to signal or rewrite an active process. UI text describes this boundary, and tests verify the next launch sees a reassignment.

## Risks / Trade-offs

- [Existing session preferences are discarded] → Reset only the `chat_preferences` JSON column; preserve sessions, history, worktrees, and identities, and state the breaking behavior in release notes.
- [Provider `ask` semantics differ in noninteractive chat] → Define and test a safe mapping for each supported CLI; if the installed provider cannot enforce the posture, fail before launch with an actionable message.
- [Removing catalog entries breaks saved profile JSON] → Catalog validation ignores no legacy values: reset governed saved selections during migration and reject future submissions containing them.
- [Claude launch flags and hook decisions can overlap] → Treat the hook as authoritative for mapped actions and retain the existing MCP floor and offline fail-closed behavior.
- [Desktop and Web implementations drift] → Put the complete policy matrix in table-driven Rust and TypeScript contract tests and add adapter-conformance assertions.
- [Large mechanical rename overlaps active work] → Restrict edits to configuration, policy mapping, selectors, adapters, and directly affected tests; preserve unrelated changes already present in the worktree.

## Migration Plan

1. Add a SQLite migration that clears every existing `sessions.chat_preferences` value and removes persisted governed CLI selections.
2. Deploy the new request/response types and Rust domain validation in the same application version; mixed old/new clients are unsupported.
3. On first load, derive `executionMode: "inherit"` with existing model discovery and non-security defaults.
4. Resolve Agent policy for every future native generation, CLI chat process, and Agent Terminal spawn.
5. Rollback requires restoring the previous application binary and database backup; the removed per-session preference snapshots cannot be reconstructed.
