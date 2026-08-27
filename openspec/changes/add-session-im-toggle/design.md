## Context

See `proposal.md` for motivation and the delta specs for observable behavior. The repository already has a native `communications` bounded context, a Feishu long-connection transport, durable event deduplication, short-lived pairing, session bindings, final-response delivery, unified redacted logging, runtime-neutral `ImService` adapters, and WebdriverIO/Tauri desktop verification. The missing boundary is an authoritative per-session opt-in checked by native inbound routing; the current information-panel IM tab immediately loads pairing controls and the current binding view carries no enablement state.

The first delivery target is Feishu direct messages. “Multi-Agent” means a direct Feishu chat bound to an existing VaneHub session containing multiple seats; it does not expand the connector's existing direct-message-only platform scope to Feishu group chats.

The design follows Feishu's event subscription and message APIs for transport behavior, and OWASP deny-by-default, least-privilege, server-side authorization, and replay-protection guidance for the access boundary. Relevant upstream references are listed at the end of this document.

## Goals / Non-Goals

**Goals:**

- Make Feishu access deny-by-default for newly eligible sessions and enforce it in native code for pairing and every inbound turn.
- Preserve existing bindings and distinguish a manual binding pause from a session-level IM opt-out.
- Route Feishu text into the existing single-Agent or multi-Agent session execution path without duplicating mention or handoff logic in the connector.
- Keep desktop and Web/mock contracts behaviorally aligned while keeping `invoke()` out of React components.
- Add deterministic WebdriverIO coverage through the real Tauri IPC and SQLite boundaries, plus a separately gated live-Feishu qualification path.
- Leave a data model that can support another connector later without changing the meaning of the current switch.

**Non-Goals:**

- Feishu group chats, attachments, voice, cards, reactions, streaming response edits, or user-directory synchronization.
- A new connector protocol implementation, frontend state library, browser-side secret store, or alternative desktop automation framework.
- Replacing the existing pairing command, durable deduplication, outbound chunking, Agent execution, or multi-Agent handoff models.
- Treating fixture success as proof that a real Feishu tenant, permissions, or network path works.

## Decisions

### 1. Persist connector-scoped session access in the communications context

Add an append-only SQLite migration for `im_session_connector_access` with `(session_id, connector)` as the primary key, an `enabled` boolean, and `updated_at`. A missing row means disabled. The table belongs to `communications`, references `sessions(id)` with cascade deletion, and stores no external identity or secret.

The first UI reads and mutates only the `feishu` row, but connector scope avoids a future global boolean whose meaning changes when another platform is exposed. During migration, existing live bindings receive an enabled access row so an upgrade does not silently stop an already-authorized chat; unbound and newly created sessions remain off.

Alternative considered: add `im_enabled` directly to `sessions`. Rejected because it gives the sessions context ownership of connector policy and cannot represent staged rollout by connector. Alternative considered: infer enablement from the presence of a binding. Rejected because opt-in must precede pairing and disabling must retain the binding.

### 2. Keep access state separate from binding pause state

`SessionBinding.state` continues to represent the user's manual pause/resume choice. Session connector access is a second gate. Effective delivery eligibility is:

```text
session connector access enabled
AND binding state active
AND connector lifecycle connected
```

Disabling access does not rewrite an active binding to manually paused. The UI derives an “opted out” effective state, while a manually paused binding remains paused after access is re-enabled. This makes re-enable deterministic and prevents the switch from erasing user intent.

Alternative considered: set every binding to `paused` when the switch is turned off and automatically set it back to `active` when turned on. Rejected because it cannot distinguish a binding the user had already paused.

### 3. Extend the typed service boundary additively

Add a strict `ImSessionAccess` contract containing `sessionId`, `connector`, `enabled`, and `updatedAt`. Extend `ImSessionBindingView` with `access`, and add `setSessionAccess(sessionId, connector, enabled)` to `ImService`.

- `SessionImPane` and `useSessionImState` call only `ImService`.
- `tauri-im-client.ts` owns `invoke("set_im_session_access", ...)` and schema parsing.
- `web-im-client.ts` keeps deterministic per-session/per-connector access state in memory and applies the same default and error semantics.
- The Rust command maps the request into the communications application service; SQLite is never accessed from React.

The information panel renders a proper accessible switch before connector or binding controls. Turning off a bound session requires confirmation; pending state disables repeated mutations, and success replaces local state with the normalized service result. Only Feishu is offered in this first UI slice even if other connectors are configured globally.

Alternative considered: keep the toggle only in component state. Rejected because browser state is neither an authorization boundary nor valid desktop persistence evidence.

### 4. Enforce opt-in at every native entry point

The communications application service checks the Feishu access row:

1. before creating a pairing intent;
2. again before consuming a pairing intent, covering disable/pair races;
3. after resolving a binding and before submitting ordinary inbound text to Agent execution;
4. before optional completion notifications for that session.

The native check is authoritative; hiding UI controls is only presentation. Unknown sessions, missing access rows, repository failures, and unsupported connector values fail closed with stable safe codes. Access mutations and routing checks use the repository transaction/serialization boundary needed to ensure a completed disable prevents subsequently admitted work. An Agent turn already admitted before disable is allowed to finish, but its optional desktop completion notification is suppressed after a fresh access check; the final reply for the already accepted IM-originated turn remains tied to that accepted turn.

Pairing remains short-lived, connector-scoped, single-use, hashed at rest, and external identifiers remain hashed or held behind secure delivery references. Existing Feishu `event_id` claim-before-execution deduplication and early acknowledgement remain unchanged.

Alternative considered: check access only when the binding is created. Rejected because authorization can be revoked and must be evaluated for every later external message.

### 5. Delegate multi-Agent routing to the existing session runtime

The communications layer passes normalized message text, the bound session id, and IM origin metadata to the existing Agent execution port. It does not parse seat labels or choose Agents. The sessions/agent-runtime path already owns stable seat identities, line-leading human mentions, current-turn fallback, bounded serial handoffs, and terminal completion.

For an invalid or unavailable seat mention, the shared routing layer returns a typed safe outcome that communications localizes for Feishu. Only one terminal external response is dispatched for the admitted inbound turn; intermediate seat messages remain in the VaneHub thread and are not mirrored individually.

Alternative considered: implement `@seat` parsing in the Feishu adapter. Rejected because it would create divergent desktop and IM routing semantics and couple a vendor transport to Agent orchestration.

### 6. Preserve the existing Feishu transport and narrow its permissions

The existing native long-connection adapter remains the ingress mechanism and the existing `im/v1/messages` API remains the egress mechanism. The implementation requests only the Feishu bot/message scopes needed to receive direct text and send replies. Tenant tokens continue to be cached with expiry skew and invalidated on authentication rejection; credentials stay in the operating-system credential store.

Protocol frames normalize to the shared inbound type. The stable Feishu event id is the idempotency key. Acknowledgement remains decoupled from Agent completion, outbound text is split after terminal completion using the connector's Unicode-safe limit, and raw frames, message text, external ids, tokens, and credentials never enter persisted diagnostics.

Alternative considered: replace the current transport with a new SDK while adding the toggle. Rejected because the access feature does not require a protocol rewrite and doing both would enlarge the regression surface.

### 7. Add a production-inert deterministic Feishu desktop fixture

Create a dedicated `desktop-feishu-im` WebdriverIO layer using the existing shared Tauri service, isolated application-data directory, deterministic CLI Agent fixture, screenshots, result JSON, process ownership, and unified-log evidence collection.

The `desktop-e2e` build may expose narrowly scoped fixture commands or a fixture transport for:

- configuring a connected fake Feishu transport without credentials;
- injecting recorded direct-text, duplicate, malformed, and disconnect events;
- reading a sanitized outbound fixture ledger containing only sequence and safe status metadata;
- forcing connector recovery and oversized deterministic Agent output.

All fixture assembly and commands are compiled and registered only under `desktop-e2e`, activated by a layer-specific environment flag, and covered by boundary tests proving they are absent from production builds. The WebdriverIO spec operates the switch through the rendered WebView, uses Tauri IPC only for fixture setup/evidence, and verifies state after a real app relaunch. The npm entry point is `test:desktop:feishu-im`; the default `test:desktop` composition can include it once runtime and cross-platform stability are demonstrated.

Alternative considered: verify the feature only with React or Playwright mocks. Rejected because those tests cannot prove native persistence, IPC registration, routing, or Tauri lifecycle. Unit and Playwright coverage still remain useful lower layers and continue to run.

### 8. Gate live Feishu qualification and isolate credentials

Live qualification is opt-in and never part of the deterministic CI result. It requires an explicit flag, a dedicated Feishu test application/tenant, a permitted direct chat, and credentials supplied at runtime. WebdriverIO enters credentials through the normal settings UI into an isolated desktop run, avoids screenshots and command logging during secret entry, and clears the connector plus its run-scoped credential reference during cleanup.

The live matrix covers authentication, long-connection health, direct-message receipt, retry/deduplication, single-Agent response, multi-Agent mentioned/default routing, outbound chunking, disable/re-enable, restart, and credential rejection. Results use `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`; retained artifacts contain safe codes and timestamps only. Missing credentials yield `BLOCKED`/`NOT RUN`, never fixture-based `PASSED`.

Alternative considered: commit a tenant fixture or reuse a developer's configured connector. Rejected because either leaks credentials or mutates personal application state.

## Risks / Trade-offs

- [Two independent gates can confuse users] → Render one effective status with a clear reason (`session-disabled`, `binding-paused`, or connector lifecycle) and keep manual pause controls hidden while session access is off.
- [Upgrade policy conflicts with deny-by-default] → Backfill enabled rows only for existing bindings, which represent prior explicit authorization; every unbound/new session remains disabled.
- [Disable races with inbound admission] → Re-check native access at pairing consumption and execution admission, serialize the relevant mutation/admission boundary, and add race tests.
- [Feishu retries can duplicate expensive Agent work] → Preserve durable event-id claiming before admission and test retry before, during, and after execution.
- [Multi-Agent replies can produce excessive external traffic] → Deliver only the terminal response externally and keep bounded internal handoffs unchanged.
- [Fixture-only commands increase attack surface if mispackaged] → Feature-gate implementation and registration, require a layer flag, and add source/production-artifact boundary tests.
- [Live automation can leak credentials in screenshots or logs] → Suppress capture during secret entry, use the normal write-only credential path, redact environment-name/value patterns, and clear run-owned credentials during cleanup.
- [Feishu API or permission behavior changes] → Keep wire behavior behind the existing adapter, pin qualification evidence to the observed date/application configuration, and report the live layer independently.

## Migration Plan

1. Add the connector-scoped access table and backfill enabled Feishu access only for sessions with existing Feishu bindings; verify empty/current/legacy database fixtures.
2. Add native domain/repository/service/command contracts and enforce access at pairing creation, pairing consumption, inbound admission, and completion notification.
3. Extend strict frontend contracts, Tauri and Web/mock adapters, hook state, and localized accessible information-panel UI.
4. Route fixture Feishu messages through the existing single- and multi-Agent execution path and add focused Rust, service, hook, and component tests.
5. Add the feature-gated fixture transport/commands, WebdriverIO layer, npm/orchestrator entry point, relaunch coverage, and production-boundary tests.
6. Run the full repository verification matrix and record deterministic desktop evidence. Run live qualification only after the user provides a dedicated Feishu test environment.

Rollback removes UI exposure and command use while leaving the additive access table harmless. A code downgrade ignores the table; it does not delete bindings or credentials. If migration fails, startup fails safely without partially enabling previously unbound sessions.

## Open Questions

- Live qualification needs the test application's App ID/App Secret, confirmation that bot/message event permissions and long-connection subscription are enabled, and a dedicated direct chat. These inputs affect only whether the separately gated live matrix can run, not the implementation design or deterministic task plan.

## Upstream References

- Feishu Open Platform, event subscription overview: https://open.feishu.cn/document/server-docs/event-subscription-guide/overview
- Feishu Open Platform, receive-message event: https://open.feishu.cn/document/server-docs/im-v1/message/events/receive
- Feishu Open Platform, create-message API: https://open.feishu.cn/document/server-docs/im-v1/message/create
- OWASP Authorization Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html
- OWASP Business Logic Security Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Business_Logic_Security_Cheat_Sheet.html
