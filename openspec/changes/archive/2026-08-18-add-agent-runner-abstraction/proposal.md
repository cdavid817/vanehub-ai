## Why

Agent generation is currently coupled to a local native process even though VaneHub already has canonical Runs, Mission Control, permission governance, and a pooled SSH runtime. A runner boundary is needed so the same provider can execute locally or through an approved remote workspace while Runs remain observable after their Session page is no longer visible and recovery never presents dead work as running.

## What Changes

- Add a provider-neutral Agent Runner contract with declared capabilities, preparation, spawn, input, event streaming, cancellation, inspection, cleanup, and supported recovery/reconnect behavior.
- Move the existing CLI subprocess behavior behind a Local Runner without changing default invocation, cwd, environment, prompt delivery, streaming, cancellation, exit, resume, logging, or terminal behavior.
- Add an SSH Runner that reuses the published `ssh_connections` profile, host-trust, native credential, connection-pool, keepalive, and channel contracts; no second SSH transport or credential store is introduced.
- Persist a bounded runner reference and recovery facts with canonical Runs, distinguish runner failures from provider failures, and reconcile non-terminal local or SSH Runs conservatively after restart without replaying prompts, tools, approvals, questions, or destructive actions.
- Decouple accepted generation ownership from the visible Session page so navigation, tab closure, window minimization, and close-to-tray do not cancel work; explicit application quit remains governed by each runner's shutdown policy.
- Extend the shared frontend service, Tauri adapter, and Web/mock adapter with runner capabilities and selection. Local remains the compatible default; Web exposes deterministic simulated local/SSH behavior and does not claim native persistence or network execution.
- Extend Mission Control with reliable local/SSH filtering, a runner badge and bounded host label, and canonical recovery/disconnect states and actions.
- Enforce runner-scoped resource budgets, cancellation/cleanup bounds, approved secret injection, safe command construction, redacted correlated diagnostics, and deterministic contract/security/performance evidence.
- Treat Docker/Sandbox and future cloud runners as unavailable declared capabilities in this stage. This change does not add a Docker transport, privileged execution, a daemon, cloud execution, or marketplace behavior.

## Capabilities

### New Capabilities

- `agent-runner-runtime`: Defines provider-neutral runner selection and contracts, Local and SSH execution, background page lifecycle, recovery, cancellation, security, compatibility, runtime parity, and bounded resource behavior.

### Modified Capabilities

- `agent-run-state-management`: Adds durable bounded runner identity/reference, runner-owned cancellation, and runner-aware conservative recovery to canonical Runs.
- `agent-mission-control`: Makes runner filtering reliable and displays safe runner kind, host, disconnect, and recovery information without becoming a lifecycle authority.
- `agent-provider-runtime`: Requires provider invocation/translation and runner execution location to remain orthogonal.
- `remote-terminal-runtime`: Reuses the existing SSH pool and independent channel lifecycle for remote Agent execution, including disconnect, reconnect, cancellation, and cleanup behavior.
- `permissions-core`: Scopes execution and secret admission to the selected runner and fails closed when runner authority is absent or stale.
- `runtime-performance-governance`: Adds deterministic concurrency, buffer, cleanup, pool-reuse, and resource-growth budgets for Agent runners.
- `unified-log-management`: Adds runner-correlated, redacted lifecycle diagnostics while keeping provider and runner error classifications distinct.

## Impact

- Native runtime: extends existing `agent_runtime`, `operations`, `ssh_connections`, `sessions`, and composition-root boundaries; it adds no bounded context and no parallel SSH, permission, logging, or Run model.
- Persistence: additive transactional Run runner metadata and recovery evidence/indexes; existing Run, Session, message, SSH, operation, and observability records remain readable.
- Frontend: extends strict runner types, `AgentService`, Tauri/Web adapters, chat run creation controls, Mission Control filters/cards, localization, and responsive visual coverage. React continues to avoid direct Tauri invocation.
- Compatibility: omitted runner selection resolves to Local, existing command names and serialized fields remain compatible, and built-in provider behavior is preserved unless SSH is explicitly selected.
- Security and operations: native code alone resolves credentials, host trust, process/channel ownership, cleanup, and unified redacted logs. Raw prompts, arguments, environment values, credentials, unrestricted paths, and remote output are excluded from Run summaries and diagnostics.
