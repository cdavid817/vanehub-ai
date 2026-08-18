## Context

See `proposal.md` for motivation. Canonical lifecycle and Mission Control already belong to `operations`; provider translation and generation lifecycle belong to `agent_runtime`; Session remote-workspace snapshots and current SSH bindings belong to `sessions`; authenticated SSH profiles, credentials, host trust, transport pooling, and independent channels belong to `ssh_connections`. The current CLI process adapter combines provider invocation preparation/parsing with local `std::process` ownership, and canonical Runs do not retain runner metadata. Mission Control accepts a runner filter but treats all persisted Runs as Local and returns no remote results.

The completed `harden-runtime-lifecycle-and-boundaries` change is a prerequisite baseline for process reaping, bounded output, short lock scopes, and architecture ports. The active Skill Evolution orchestration change touches `desktop-background-lifecycle` for its own internal scheduler; this change does not extend that scheduler or create a competing lifecycle service.

## Goals / Non-Goals

**Goals:**

- Separate provider protocol behavior from execution-location behavior at an application-owned port.
- Preserve byte-compatible Local CLI behavior when no runner is selected.
- Execute eligible CLI providers through the existing SSH pool without duplicating transport, trust, credential, or Session workspace models.
- Persist enough safe Runner identity and evidence for Mission Control filtering, cancellation ownership, and conservative restart reconciliation.
- Make renderer/page lifetime irrelevant to accepted native work while keeping explicit desktop quit honest and bounded.
- Enforce permission, secret, command-construction, output, reconnect, concurrency, and cleanup bounds with deterministic tests.

**Non-Goals:**

- A Docker daemon adapter, container image management, privileged execution, cloud Runner, external daemon, or application-exit persistence guarantee.
- Running direct HTTP/API providers on SSH; those providers continue to declare Local-only execution because their transport is the native API adapter rather than a CLI process.
- A second canonical lifecycle, SSH stack, permission evaluator, log store, terminal transcript store, Session model, or Mission Control detail implementation.
- Replaying prompts, stdin, tool calls, approvals, questions, or destructive actions during reconnect or restart recovery.
- Work from roadmap item 13 or later.

## Decisions

### 1. Agent Runtime owns the Runner application contract

`agent_runtime` already owns provider invocation and generation lifecycle, so it will define the consuming-side Runner models and ports. The contract uses stable `RunnerKind`, `RunnerSelection`, `RunnerCapabilities`, `RunnerLaunchSpec`, `RunnerHandle`, `RunnerEvent`, `RunnerInspection`, and classified `RunnerError` values. The effective operations are capability discovery, preparation, spawn, input, event polling/streaming, cancel, inspect, cleanup, and recover.

`LocalRunner` and `SshRunner` are infrastructure adapters assembled by bootstrap into a registry. `SshRunner` consumes only the deliberately published `SshConnectionsApi`; it does not import SSH repositories or russh infrastructure. `operations` receives immutable Runner metadata through its published Run API and never invokes a Runner directly.

Alternative considered: add a `runners` bounded context. Rejected because Runner is a replaceable execution port inside the existing Agent generation lifecycle and has no independent aggregate or transaction ownership in this stage.

### 2. Split provider execution from transport without changing the public generation port

The existing `AgentProcessGateway` remains the application-facing generation port to avoid spreading a compatibility refactor through Sessions and commands. Internally, its CLI implementation becomes a provider execution coordinator:

1. resolve the provider by stable id and negotiate provider capabilities;
2. build the provider-owned invocation and output parser contract;
3. resolve and validate the selected Runner;
4. ask the Runner to execute an opaque bounded launch specification;
5. feed raw Runner stdout/stderr/exit events into the provider parser and existing normalized event sink;
6. preserve Runner errors separately when no provider terminal event exists.

The Runner never sees provider ids for branching and never parses provider protocol. Provider adapters never create a local process, SSH channel, or transport. The current API-process adapter remains a Local-only `AgentProcessGateway` implementation because it is not a CLI execution-location concern.

Alternative considered: wrap the current process adapter as `LocalRunner`. Rejected because that would leave provider parsing inside the Runner and make an SSH implementation duplicate provider logic.

### 3. Runner selection is explicit but backward compatible

The shared contract adds `RunnerSelection` to message/generation submission. It is optional at transport boundaries; absence normalizes to Local before canonical Run creation. A selection contains only kind and stable target id/revision. Remote cwd, display label, endpoint, and current binding are resolved from the authoritative Session and SSH APIs, never trusted from React.

The service exposes a bounded `listAgentRunners(sessionId, agentId)` capability query. It returns Local, eligible current SSH binding, and unavailable Docker/Sandbox descriptors with safe reasons. Availability is side-effect free: it does not authenticate, launch, retrieve a credential, or open a channel. A stable selection is revalidated during preparation to close time-of-check/time-of-use gaps.

The chat composer adds a compact localized Runner selector for CLI Agents. Local is default. Direct API Agents show Local as fixed. The floating assistant keeps Local because it has no remote-workspace selection surface. Existing clients and tests that omit selection retain their current behavior.

Alternative considered: infer SSH solely because a Session has a remote workspace. Rejected because users need an explicit execution-location choice and silent remote execution would be unsafe.

### 4. Local Runner extracts current process ownership with compatibility fixtures

Local process creation, stdin, stdout/stderr readers, cancellation, child-tree reaping, cwd normalization, output caps, and handle registry move behind `LocalRunner`. Provider preparation, MCP arguments, Codex final-output handling, output parsing, usage normalization, evidence, and telemetry remain in the provider coordinator.

Golden invocation and parser fixtures assert unchanged executable, args, cwd, prompt delivery, event order, opaque resume id, usage, error mapping, cancellation, and operation/log correlation for every built-in CLI. Local Runner uses the existing managed-process/platform containment facilities and preserves Windows Job Object and Unix process-group semantics.

Alternative considered: rewrite Local execution to a new async process library at the same time. Rejected because it increases compatibility risk and is unnecessary for the abstraction.

### 5. SSH Runner reuses the current profile binding and pool

Runner preparation requests the Session's published current remote SSH binding and snapshot, then resolves the matching profile through `SshConnectionsApi`. The SSH API publishes a narrow execution facade returning a lease-backed independent channel; connection establishment, secure credential loading, host-key callbacks, keepalive, capacity, revision keys, draining, and shutdown remain private to `ssh_connections`.

The first implementation supports remote exec with normalized bounded UTF-8 chunks. It requires the provider executable to be available on the remote host and uses the Session remote path as cwd. It does not forward the local inherited environment. An explicit allowlist contains only non-secret tracing/configuration keys that are safe and supported remotely; provider credentials are expected to be configured on the remote host unless a future approved secret-delivery contract exists. Thus local secrets cannot leak by inheritance.

Remote command construction uses one tested POSIX command encoder: executable, cwd, arguments, and allowlisted environment names reject NUL/control characters and are single-quote encoded as data. No caller supplies raw shell fragments. The wrapper reports a bounded opaque process reference and exit status on a dedicated control prefix, starts the command in an owned process group where supported, and forwards provider stdin separately. Cancellation first signals the owned process group through a separate bounded exec request, then closes only the Run channel and escalates within a fixed budget. If the remote platform cannot establish ownership, spawn fails rather than claiming cancellability.

Alternative considered: use a new SSH crate or run local `ssh` as a subprocess. Rejected because both bypass the existing trust, credential, revision, pool, and lifecycle invariants.

### 6. Reconnect is policy-driven inspection, never replay

Runner capabilities distinguish `none`, `inspect_only`, and `reattach` recovery. Local CLI Runs are `none` across application restart unless an in-process handle still exists. SSH v1 is `inspect_only`: a transient transport loss enters a bounded Runner-disconnected reason, reacquires the same current profile revision within retry/backoff limits, and uses the opaque remote process reference to inspect liveness. If the original channel cannot be safely reattached, the Run becomes interrupted/attention-required rather than pretending output continuity. Fake SSH tests cover both a declared reattach-capable transport and the production inspect-only path.

Startup recovery first loads canonical non-terminal Runs, asks the owning runtime to inspect Runner evidence, and commits one idempotent outcome. Changed profile revision, host trust, credential availability, permission revision, missing handle, or ambiguous destructive activity prevents automatic reconnect. No recovery path writes provider stdin or starts another provider command.

Alternative considered: automatically invoke provider resume after restart. Rejected because provider resume starts new work and is not proof that the old remote process is alive.

### 7. Canonical Run snapshots own safe Runner metadata

`AgentRun` gains optional `runner` metadata for backward deserialization:

- `kind`: `local` or `ssh`;
- bounded target id and safe display/host label;
- target revision where applicable;
- declared recovery mode;
- authority and runner capability witnesses;
- opaque recovery reference hash/id, never credential or command content.

New Agent generation Runs require metadata before the `running` transition. Existing snapshots remain readable with `runner = null`; only known legacy generation owners project as `legacy-local`, and they never gain remote/recovery claims.

The migration additively adds nullable `runner_kind` and `runner_target_id` projection columns plus an index over runner/state/update order; `snapshot_json` remains authoritative. Repository writes update snapshot and projections atomically. Mission Control filters the indexed projection and maps metadata to new summary fields without per-row lookups. No legacy row is destructively backfilled.

Alternative considered: store Runner metadata only as generic Run links. Rejected because reliable indexed filtering, immutable ownership semantics, revision evidence, and recovery classification are not generic business links.

### 8. Background behavior follows native ownership, not React lifetime

The native generation registry owns all accepted Runner handles and monitors. React unmount cleanup removes only listeners; it never calls cancel merely because a Session tab, chat surface, or route is hidden. Main-layout navigation keeps Mission Control as the bounded cross-Run recovery surface. Window minimize and close-to-tray preserve the process and therefore active handles.

Explicit Session archive/delete retains its existing owner policy and can cancel owned generation. Explicit application quit stops admission, transitions/cancels Local Runs, requests bounded SSH cancellation/close, flushes terminal evidence and unified logs, then uses existing graceful shutdown. The UI and Web adapter explicitly state that application/browser process exit is not a background guarantee.

Alternative considered: detach Local children from VaneHub. Rejected because it would weaken permission, logging, process-tree cleanup, and truthful recovery.

### 9. Permissions and secrets are evaluated against Runner authority

Runner preparation builds a permission context from stable principal, execution action, runner kind, target id/revision, Session/project scope, and policy revision. The existing permission API remains authoritative. Local grants do not authorize SSH; stale or incomplete context fails closed. A second revision check occurs immediately before spawn.

The Runner launch spec contains an allowlisted environment map assembled after permission admission. Secret handles, if any future provider requires them, are resolved natively at the last responsible moment and scoped to the selected Runner/process. SSH v1 forwards no local secrets. Values are zeroized/dropped with preparation state where practical and never enter serialization, SQLite, frontend, logs, telemetry, or error text.

Negative tests cover stale SSH revisions, changed host key, missing credential, Local-only grant used remotely, environment-name injection, control characters, unsafe cwd, raw shell fragments, unauthorized secret requests, and no silent Local fallback.

### 10. Runner errors, logs, and output remain separated

`RunnerError` has stable categories for unsupported capability, invalid selection, authority stale, permission denied, preparation, spawn, input, transport disconnected, reconnect exhausted, cancellation, inspection, cleanup, and resource exhausted. Provider failures keep their existing classification. Orchestration chooses Runner classification only when transport/ownership failed before a trustworthy provider terminal result.

Runner lifecycle diagnostics use the `operations` diagnostic port with canonical Run, operation, runner kind, safe target id, category, attempt count, and bounded numeric metadata. Raw command, cwd, arguments, environment, prompt, output, credential/key path, and endpoint user info are excluded before the unified redaction layer. Provider-visible output remains in chat/terminal presentation and existing content stores, not diagnostic logs or Run summaries.

### 11. Tauri and Web adapters stay contract compatible

`AgentService` adds runner descriptors/selection and extends Run/Mission summaries. Only `tauri-agent-client.ts` invokes the new native query; send-message DTO mapping remains in its existing command. `web-agent-client.ts` uses deterministic in-memory Local and SSH fixtures, common state transitions, disconnect/reconnect outcomes, cancellation, filtering, and background route navigation while the page is alive. Components receive typed service data and contain no `invoke`, SSH, or OS branches.

Contract normalization accepts missing Runner fields from older native/mock fixtures as legacy Local/unavailable metadata, while all newly created Runs return explicit Runner data. Compile-time adapter parity and runtime contract parsers cover every new field and error code.

### 12. Structural performance budgets accompany dedicated evidence

The existing versioned harness gains deterministic metrics for maximum active Runner handles, per-Run event queue/chunk/byte caps, global retained bytes, SSH transports per compatible profile revision, channels per Run, reconnect attempts, cancellation records, and zero live handles after cleanup. Fixtures cover 1, 8, and the declared maximum mixed Local/SSH concurrency plus exactly-one-bound negative cases.

Dedicated Windows evidence records Local spawn/cancel throughput and resource growth; fake SSH provides portable deterministic evidence, while live SSH latency remains dedicated/informational. Shared CI enforces structural counts only. Mission Control retains constant query count and bounded page size with runner filtering over 100 and 1,000 Run fixtures.

## Risks / Trade-offs

- [Provider coordinator extraction changes Local behavior] → Preserve golden invocation/output fixtures for every built-in CLI and land the Local split before enabling SSH selection.
- [Closing an SSH channel does not terminate the remote process] → Require a verified owned process reference/process group and bounded cancel probe; reject spawn when ownership cannot be established.
- [SSH shell encoding enables injection] → Accept typed executable/args/cwd/env only, reject controls and raw fragments, use one property-tested encoder, and run negative fixtures before opening a channel.
- [Remote host lacks the provider executable] → Perform a bounded non-interactive availability probe during preparation and return a Runner preparation classification without Local fallback.
- [Network loss makes output continuity unknowable] → Declare production SSH recovery `inspect_only`, expose disconnected/interrupted truthfully, and never claim reattach without a transport capability and evidence.
- [Run migration slows Mission Control] → Add nullable projections and a composite index transactionally; do not backfill payloads or add per-row reads.
- [Concurrent Runner output grows memory] → Use per-handle bounded queues plus global caps and deterministic cleanup/resource tests.
- [Active Skill Evolution background work conflicts with shutdown] → Reuse the composition-root shutdown supervisor and do not alter its domain or `desktop-background-lifecycle` delta.
- [Web fixtures imply native background behavior] → Mark descriptors simulated and limit continuity to page-active in-memory behavior.

## Migration Plan

1. Add Runner contracts, metadata types, Local default normalization, contract tests, and additive Run migration with selection disabled in UI.
2. Extract Local process ownership behind `LocalRunner`; run all provider compatibility, cancellation, terminal, Session, and desktop tests before proceeding.
3. Publish the narrow SSH execution facade and implement fake-transport `SshRunner` conformance, security, pool reuse, reconnect, cancellation, and cleanup tests.
4. Enable native SSH Runner selection only after current binding, availability, permission, host-trust, credential, and ownership checks pass.
5. Extend `AgentService`, Tauri/Web adapters, Mission Control projection/filtering, localized selector/badges, background navigation, and responsive visual tests.
6. Add startup reconciliation, explicit shutdown ordering, unified diagnostic mapping, deterministic performance datasets, live Windows desktop integration, and full validation evidence.

Rollback first disables non-Local selection in runner discovery while continuing to read and display persisted Runner metadata. Active Runs are reconciled or cancelled by their recorded owner before removing execution wiring. The additive columns/index and optional snapshot fields remain readable; rollback does not delete Runner evidence or rewrite legacy records. Local remains the compatible default throughout.
