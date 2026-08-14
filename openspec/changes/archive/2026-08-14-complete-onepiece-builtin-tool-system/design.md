## Context

See `proposal.md` for motivation and the delta specs for observable behavior. The native API runtime already owns provider translation, a multi-turn tool loop, permission evaluation, approval waits, cancellation, tool activity persistence, and baseline file/search/shell/Skill/LSP adapters. Most dispatch still converges in `api_process_adapter.rs`, while definitions are assembled in `application/tool_catalog.rs`; adding six large execution domains directly to that branch structure would make eligibility, policy, and cleanup increasingly difficult to verify.

The desktop runtime is the only trusted boundary that can own local processes, browsers, network clients, SQLite, credential handles, and managed files. React must remain behind shared service interfaces, and Web/mock must preserve contracts without pretending that native effects occurred. All new tools are product capabilities of stable Agent id `onepiece`, not general abilities of every API Agent and not new Agent identities.

The design also has to bridge very different isolation strengths. Browser and Web access intentionally reach external origins, code execution must not inherit that network authority, local OCR consumes private content without remote fallback, and external CLI delegation needs provider connectivity while its generated child commands remain offline. Prompt instructions help models behave correctly but are not security controls; native enforcement remains authoritative.

## Goals / Non-Goals

**Goals:**

- Replace name-based branching with a fixed registry whose catalog eligibility and dispatch checks are explicit and testable.
- Add independently governed Browser, Web, code-execution, OCR, Artifact, and external-CLI delegation domains without duplicating permission, cancellation, logging, or operation infrastructure.
- Keep every new provider schema stable as installations, versions, sessions, and Artifacts change.
- Make Artifact identity and lineage the transfer boundary between tools rather than arbitrary paths or unbounded inline bytes.
- Make external delegation reviewable and reversible: isolated execution first, immutable ChangeSet second, exact target application last.
- Provide honest desktop/Web adapters and mode-specific readiness instead of optimistic feature flags.
- Ship in independently gated phases so read-only paths can mature before edit/application paths.

**Non-Goals:**

- Giving the new tools to user-created API Agents or CLI-wrapped chat Agents.
- Attaching automation to the user's normal Chrome profile, importing its cookies, or silently reusing unrelated browser sessions.
- Providing arbitrary Internet access to shell, code execution, OCR, Artifact rendering, or delegated child commands.
- Installing arbitrary code-execution packages or accepting user-defined runtimes in V1.
- Uploading Artifacts to a public hosting provider or creating public share links.
- Treating OCR output, Web content, provider reports, or delegated model claims as trusted facts.
- Recursive or multi-level external delegation, automatic provider retry, automatic merge/rebase, partial ChangeSet application, commit, or push.
- Replacing existing local-extension lifecycle management, general CLI chat, session terminal, MCP, or Loop orchestration.

## Decisions

### 1. Refactor the native tool loop around a fixed handler registry

Introduce a `NativeToolHandler` application contract with immutable metadata and separate runtime execution:

```text
NativeToolHandler
├─ definition() -> NativeToolDefinition
├─ eligibility(context) -> ToolEligibility
├─ permission_request(input, context) -> ActionResource
├─ validate(input) -> ValidatedInput
└─ execute(validated, execution_context) -> ToolResultEnvelope
```

`NativeToolDefinition` owns stable name, schema version, provider-neutral JSON schema, description, operation categories, hard-limit profile, and Plan-mode compatibility. `ToolEligibilityContext` carries stable Agent id, session/generation identity, permission mode, canonical workspace, readiness snapshots, and available cross-context ports. `ToolExecutionContext` adds cancellation, deadline, approval witness, progress sink, operation/run correlation, and redacted logger.

The initial extended names are fixed:

```text
browser
web_search
web_fetch
code_execution
ocr
artifact
delegate_cli
apply_delegation_changes
```

The existing shell/file/edit/grep/glob/Skill/LSP/MCP paths are adapted to the same registry incrementally. Catalog construction evaluates eligibility, but dispatch repeats eligibility and policy checks. The provider never receives one tool per browser page, runtime, OCR language, Artifact, Skill, or installed CLI.

`agent_runtime` continues to own the provider loop and registry. Domain-specific work is reached through narrow application ports:

```text
agent_runtime
├─ BrowserAutomationPort      -> browser_automation context
├─ WebResearchPort            -> web_research context
├─ CodeExecutionPort          -> code_execution context
├─ OcrInferencePort           -> local_extensions context
├─ ArtifactPort               -> artifacts context
└─ CliDelegationPort          -> cli_delegation context
```

This keeps provider wire formats out of the execution domains and keeps process/network/storage implementation out of `api_process_adapter.rs`.

Alternatives considered:

- Continue adding `match tool_name` branches to `api_process_adapter.rs`. Rejected because catalog, validation, permission, and dispatch would continue drifting.
- Register one tool per dynamic resource. Rejected because schemas would churn, collide, and grow without bound.
- Build a generic arbitrary plugin executor. Rejected because each domain needs a different isolation and evidence contract.

### 2. Enforce OnePiece-only eligibility twice

Every new handler declares `agent_id == "onepiece"` as a hard eligibility predicate. Catalog construction excludes it for every other Agent; dispatcher authorization repeats the stable-id check before parsing sensitive input or requesting approval. Display name, provider, model, origin-like metadata, and capability tags are never substitutes.

OnePiece chat readiness remains independent from optional tool readiness. A missing browser or OCR runtime removes only that operation. Readiness is exposed as a capability map with stable reason codes, not injected by mutating the OnePiece Agent row for every transient dependency.

Alternatives considered:

- Use `launch_kind = api` or a `tools` capability tag. Rejected because that would grant the tools to custom API Agents.
- Hide only in the frontend. Rejected because provider calls and direct native commands could bypass presentation.

### 3. Centralize lifecycle, limits, and permission mapping

All handlers use a common `ToolOperation` aggregate:

```text
ToolOperation
├─ operation_id / call_id / execution_run_id
├─ agent_id / session_id / generation_id
├─ tool_name / operation_category
├─ input_hash / limit_snapshot / policy_revision
├─ state
├─ progress_sequence
├─ started_at / terminal_at
└─ terminal_summary
```

States are monotonic: `prepared`, `awaiting_approval`, `queued`, `running`, `stopping`, `sealing`, `cleaning`, then `completed`, `denied`, `failed`, `cancelled`, `timed_out`, `limit_exceeded`, `interrupted`, or `recovery_required` where supported. Existing operation/task projections are reused for UI; feature aggregates remain the authoritative state when they carry richer invariants.

Each operation maps to stable permission actions such as:

```text
browser.navigate / browser.inspect / browser.interact / browser.evaluate
web.search / web.fetch
code.execute
ocr.extract
artifact.read / artifact.publish / artifact.download
agent.delegate
delegation.changes.apply
```

Approval resources contain canonical identifiers and safe hashes, never raw prompt, source, page, credential, or file bodies. `delegation.changes.apply` is hard-coded as Once-only even if the principal otherwise uses a trusted policy.

Alternatives considered:

- Give each context its own approval queue. Rejected because the project already has durable unified evaluation and approval semantics.
- Treat start approval as blanket approval for later actions. Rejected because later targets and resources are not yet known.

### 4. Use Artifact ids as the cross-tool binary boundary

Create a new `artifacts` bounded context. SQLite stores logical metadata and references; bytes live below the application-data directory in a content-addressed blob store:

```text
artifacts/
├─ blobs/sha256/ab/<remaining-hash>
├─ staging/<operation-id>/
└─ recovery/<operation-id>/
```

Core records:

```text
artifact_blobs(hash, size, media_type, storage_state, ref_count, created_at)
artifacts(id, blob_hash, display_name, creator_kind, creator_id,
          visibility, lifecycle, metadata_json, created_at, expires_at)
artifact_links(parent_id, child_id, relation, operation_id)
artifact_references(artifact_id, owner_kind, owner_id)
```

Sealing writes to an owned staging file, fsyncs as supported, calculates SHA-256 while streaming, validates size/type/name, atomically moves into the blob store, and commits metadata/reference changes transactionally. A logical Artifact is immutable; publication and retention are separate records/state transitions. Deduplication may reuse a blob but never erases distinct provenance.

The fixed `artifact` tool exposes bounded `list`, `metadata`, `read_text`, and `publish` operations. Native tools produce draft Artifacts through internal ports and may request publication only after sealing. Publication means availability through VaneHub's authenticated UI/service boundary, not Internet upload. Desktop download uses an application-owned save flow; React never receives the managed source path. Safe previews use bounded DTOs or an application-owned custom protocol with short-lived authorization, strict media headers, and no active-content execution. Web/mock uses in-memory deterministic records marked simulated.

Alternatives considered:

- Pass temporary paths among tools. Rejected because paths leak authority, become stale, and are difficult to authorize or retain.
- Store all bytes in SQLite. Rejected because large screenshots, documents, and ChangeSets would increase database contention and backup cost.
- Publish directly to a cloud object store. Rejected as a separate credential, privacy, retention, and product scope.

### 5. Run Playwright through an owned stdio sidecar

Use a VaneHub-managed Node/Playwright sidecar behind `browser_automation`. Rust owns process launch, cancellation, limits, policy, and Artifact sealing; the sidecar owns Playwright objects. Communication uses framed JSON-RPC over stdio with request ids and bounded messages, not a listening HTTP port.

One logical browser session owns one isolated Playwright browser context and bounded pages. Default persistence is session-scoped and ephemeral; it never opens the user's Chrome profile. A headed managed Chromium window is used for human handoff. Automation pauses during handoff, and resumption invalidates all prior element references.

`browser` uses an operation enum rather than dynamic tools:

```text
start | navigate | back | forward | inspect | click | type
screenshot | evaluate | extract | handoff | resume | close
```

Inspection returns an accessibility/semantic snapshot with generated element references, bounded visible text, origin, frame id, and revision. References are valid only for the page revision that created them. Screenshots and admitted downloads are quarantined, scanned by type/size policy, then sealed as Artifacts. Downloads never become executable automatically.

Navigation is checked in Rust before the sidecar receives it and rechecked for every redirect/navigation event. Only HTTP(S) is supported. URL credentials, `file:`, `data:`, `javascript:`, browser-internal schemes, loopback, private/link-local/metadata ranges, denied ports, and DNS rebinding are blocked unless a future explicit policy adds a narrower local-testing mode. Page requests inherit the same network policy; service workers and popups remain owned and bounded.

Alternatives considered:

- Automate the Tauri WebView directly. Rejected because it does not offer Playwright's stable browser protocol and would mix application origin with untrusted pages.
- Attach to the user's Chrome profile. Rejected because cookies, extensions, tabs, and unrelated sessions would cross the trust boundary.
- Expose the sidecar over loopback HTTP. Rejected because stdio provides simpler ownership and no port/authentication surface.

### 6. Separate DuckDuckGo search from guarded fetching

`web_research` contains two adapters:

```text
SearchProviderPort -> DuckDuckGo adapter
GuardedFetchPort   -> isolated reqwest client + extractors
```

`web_search` queries the reviewed DuckDuckGo endpoint with bounded query, locale/safety, count, timeout, and user agent. Provider-specific parsing is isolated and covered by captured fixtures so HTML/API drift produces `provider_protocol_changed`, not fabricated results.

`web_fetch` uses a client with no cookie jar, proxy credentials, ambient authorization, or local schemes. Every hop normalizes URL, resolves addresses, rejects disallowed IP ranges/ports, connects with DNS-rebinding protection, caps redirects, and applies compressed and expanded byte limits. Only admitted media types reach extractors. HTML is parsed without executing scripts; readable text and provenance are returned. Supported binary documents are routed through reviewed bounded extractors or optionally sealed as Artifacts; unsupported content returns metadata only.

Search snippets and fetched content retain different evidence kinds. Both keep normalized/final URL, capture time, title, provider, and truncation so OnePiece can cite honestly.

Alternatives considered:

- Let the model use Browser for every search/fetch. Rejected because deterministic HTTP retrieval is cheaper, easier to bound, and does not require a browser session.
- Give the code sandbox `curl`. Rejected because it collapses network policy and provenance into arbitrary code execution.

### 7. Make `code_execution` a sandbox service, never a shell alias

Create `code_execution` with a backend abstraction selected by readiness. V1 targets reviewed Python and JavaScript runtimes discovered by canonical executable/version probes; user-defined commands and package installation are not accepted. Source is written by the controller under a generated name and launched by an argument array without a shell.

The Windows implementation requires all of:

- a restricted token/AppContainer-compatible process identity;
- a Job Object enforcing kill-on-close, memory, CPU/time, and process-count limits;
- an ACL-owned disposable directory with read-only `inputs/`, writable `work/`, and write-only/controlled `outputs/` admission;
- no network capability under the sandbox backend;
- a minimal environment and no inherited credential/config homes.

Other platforms implement the same port with a reviewed native isolation backend. If the current OS cannot prove the required isolation, `code_execution` is unavailable; it never falls back to ordinary shell execution.

Inputs are selected Artifact ids materialized read-only after hash verification. Output candidates are enumerated without following links, checked for path/type/count/byte limits, then sealed as derived Artifacts. Stdout/stderr are streamed into bounded buffers and safe progress. Cancellation, timeout, output flood, sandbox violation, or cleanup failure kills the full process tree and prevents a successful terminal result.

Alternatives considered:

- Reuse `shell`. Rejected because shell inherits workspace context and cannot express a strong independent filesystem/network boundary.
- Require Docker/Podman. Rejected as the only V1 backend because it is not reliably installed on the desktop target and adds daemon trust; a future backend may use it after explicit readiness.
- Embed an unrestricted Python interpreter in-process. Rejected because a language runtime bug would share the Tauri process boundary.

### 8. Extend PaddleOCR with a separate stdio inference worker

The existing local-extension management sidecar remains health-only. Add a versioned `OcrInferencePort` and backend-owned PaddleOCR worker invocation using the managed environment and an stdio protocol. It accepts only a controller-created input path inside an invocation directory and a bounded configuration; callers cannot supply executables, modules, endpoints, environment, or free-form process arguments.

OCR inputs must be Artifacts. Images are metadata-checked before decode. PDFs are rasterized page-by-page through a reviewed `pdfium-render` adapter with a checksum-verified managed PDFium binary, pixel/page/byte limits, and no active PDF actions. Rendered pages remain private temporary inputs and are deleted after inference.

The normalized result includes engine/version, source hash, page and block ordering, text, geometry, reported confidence, language configuration, warnings, truncation, and timing. Plain text and JSON are derived projections and can become new Artifacts. No OCR input or result enters durable diagnostic logs.

Alternatives considered:

- Send images to the configured OnePiece multimodal provider. Rejected because it would be a remote fallback with different privacy and cost semantics.
- Add inference to the existing loopback health server. Rejected because it would turn a management surface into a content-bearing network endpoint.

### 9. Persist readiness separately from capability declaration

Each optional domain provides a side-effect-free readiness snapshot. Structural capability is compiled; readiness is observed. Checks never launch an interactive browser, execute user code, OCR content, or consume model quota.

```text
CapabilityReadiness
├─ capability_id / mode?
├─ state: ready | degraded | blocked
├─ checked_at / expires_at
├─ dependency_fingerprints
├─ reason_codes[]
└─ warnings[]
```

Dispatch rechecks cheap witnesses. Expensive probes are cached by executable/runtime hash, adapter version, OS/architecture, and policy revision. Settings exposes explicit self-tests; tests that contact a provider or process user content disclose effects first.

Alternatives considered:

- Infer readiness from installation rows. Rejected because protocol and isolation compatibility can fail independently.
- Hide all readiness until a call fails. Rejected because users and OnePiece need actionable availability without starting work.

### 10. Model CLI delegation as its own aggregate

Create `cli_delegation`; do not reuse ordinary CLI chat sessions, terminal sessions, Utility Skill child attempts, or Loop worker sessions. They can share process, permission, observation, Git, and provider parsing infrastructure through ports.

```text
Delegation
├─ delegation_id / parent session-generation-message
├─ agent_id = onepiece
├─ task identity
└─ attempts[]

DelegationAttempt
├─ attempt_id / target / mode
├─ immutable context and policy snapshot
├─ executable/provider/adapter fingerprints
├─ state / counters / timestamps
├─ execution_run_id / operation_id
├─ safe terminal result
└─ output Artifact ids
```

Attempt states are:

```text
prepared -> awaiting_start_approval -> queued -> preparing -> running
         -> awaiting_child_approval -> sealing -> cleaning -> terminal
```

Stopping may be entered from queue, preparation, running, approval, or sealing. Terminal states include completed, denied, failed, cancelled, timed-out, limit-exceeded, interrupted, and cleanup-failed. State and attempt events are append-only; projections in chat and task center are rebuildable.

The attempt is persisted before first provider contact. No automatic retry occurs because provider requests and child actions may consume money or have already changed the isolated clone. An explicit retry creates a new attempt.

Alternatives considered:

- Create a normal chat session for the child CLI. Rejected because it leaks conversation semantics and pollutes navigation.
- Reuse delegated Utility Skill attempts. Rejected because external CLI protocol, authentication, Git cloning, and ChangeSet evidence are materially different.

### 11. Use an independent detached clone with no remote

Delegation requires a local Git repository, clean captured HEAD, and canonical repository identity. The controller creates an independent clone with its own object store, detaches at the exact commit, removes every remote, disables hooks, and writes controller-owned Git configuration. It does not use a linked worktree because linked object/config state and remotes remain shared.

Attempt layout:

```text
attempt/
├─ workspace/       independent clone; read-only for analyze, scoped-write for edit
├─ inputs/          selected Artifacts, read-only
├─ output/          private structured final output
├─ control/         never visible to child
└─ recovery/        controller-owned cleanup evidence
```

Analyze compares full Git/index/worktree state before and after and fails on any mutation. Edit calculates changes from the exact base through a controller-owned temporary index so untracked files, deletes, renames, modes, and binary changes are included. Control-plane files, gitlinks/submodules, unsafe links, device names, control-character paths, and platform case collisions are rejected from applyable ChangeSets.

Alternatives considered:

- Run in the user's current worktree. Rejected because child edits and failure cleanup would affect user state before review.
- Use `git worktree`. Rejected because the shared repository and object store retain remote/config/GC coupling.
- Leave origin for read-only fetch. Rejected because child tools must not gain repository network authority.

### 12. Keep CLI authentication owned by each CLI and scrub the child environment

The controller starts the installed CLI through its normal account integration but never reads or copies raw OAuth tokens and never injects API keys. It builds a minimal environment needed for executable/runtime discovery and the CLI's own auth lookup, strips common credential/API/proxy variables from child actions, and ensures prompt, logs, SQLite, and Artifacts contain no credential material.

Provider control-plane network is separated from child tool network by the controller sandbox. For V1, Claude Code receives no Bash or command-execution tool at all; analyze is limited to controlled reads and edit adds only controlled Edit/Write operations inside the detached clone. This removes the child-command network path while still allowing the Claude process to reach its configured provider. Codex delegation remains unavailable on Windows until an AppContainer/WFP-compatible backend and explicit canary prove that provider traffic and child-command traffic are independently enforced. Commands, hooks, MCP, browser extensions, and code started inside any delegated task remain denied unless an explicitly approved future policy says otherwise.

If the installed CLI cannot separate authentication from unsafe ambient configuration at a supported version, that mode is not ready. Readiness reports authentication as ready/required/unknown without exposing account data.

Alternatives considered:

- Copy the CLI auth file into a temporary home. Rejected because it duplicates raw OAuth material.
- Inject the user's API key. Rejected because delegation is defined around CLI-owned authentication and would create another credential lifecycle.

### 13. Freeze an explicit delegation prompt envelope

The start approval binds a `DelegationContextSnapshot`:

```text
task hash
context_summary hash
selected Artifact ids and hashes
repository identity / base commit
repository guidance paths and hashes
target / mode / provider-model snapshot
capability and limit hashes
adapter and result-schema versions
```

Controller safety/mode/result instructions use the target's supported high-priority channel. The user task travels in the instruction payload. `context_summary`, Artifact bodies, repository source, command output, and Web content are labelled and delimited as untrusted data. Full parent transcript, hidden reasoning, unrelated memories, environment, and unselected files are not copied.

Claude filesystem settings that could load hooks, agents, skills, MCP, or ambient memory are disabled for delegation; VaneHub snapshots and supplies only admitted repository guidance. Codex project guidance is preflighted and hashed according to its supported discovery contract, while command flags/config disable unrelated MCP and unsafe extensions. Any provider version that cannot satisfy instruction isolation is blocked for delegation.

Task or context overflow fails instead of truncating. Any snapshot change after approval requires a new attempt/approval.

Alternatives considered:

- Include the last N messages. Rejected because message count is neither a privacy nor relevance boundary.
- Inline Artifact contents into the system/developer prompt. Rejected because it raises untrusted data to the wrong authority and causes unbounded context growth.
- Rely on prompt-injection keyword scanning. Rejected as a security control; scanning is advisory only.

### 14. Use target-specific reviewed invocations

`ClaudeDelegationInvocation` owns managed flags equivalent to:

```text
claude -p
  --output-format stream-json
  --include-partial-messages
  --verbose
  --session-id <fresh uuid>
  --json-schema <DelegationAgentReportV1>
  --setting-sources <isolated set>
  --strict-mcp-config --mcp-config <empty/private config>
  --tools Read,Glob,Grep[,Edit,Write]
```

It disables Chrome integration, session persistence/resume, auto memory, unsafe hooks, and unreviewed tools, and applies controller-owned maximum turns and optional dollar budget. The exact supported flags are compatibility-manifest data validated by readiness.

`CodexDelegationInvocation` owns managed flags equivalent to the following contract, but Windows readiness remains blocked until the independent network-isolation canary described above passes:

```text
codex exec
  --json
  --ephemeral
  --sandbox read-only|workspace-write
  --ask-for-approval never
  --output-schema <DelegationAgentReportV1>
  --output-last-message <private output path>
  -
```

Codex never receives `--yolo` or dangerous bypass flags. Analyze uses read-only, edit uses workspace-write inside the independent clone. The controller's child-action policy remains authoritative even though the CLI is invoked non-interactively.

User CLI parameter profiles may select reviewed model/reasoning options but cannot replace owned delegation flags, output destinations, sandbox, approval policy, prompt delivery, or session behavior.

Alternatives considered:

- Reuse ordinary CLI generation invocation verbatim. Rejected because it resumes sessions, uses a lenient parser, and lacks delegation-specific isolation and final evidence.

### 15. Add stateful provider-specific protocol adapters

Do not use the current broad `StructuredJson` fallback as delegation truth. Add:

```text
DelegationProtocolAdapter
├─ decode_stdout_line(&mut self, bytes)
├─ decode_stderr_chunk(&mut self, bytes)
└─ finalize(&mut self, exit, captured_final)

ClaudeDelegationAdapter
CodexDelegationAdapter
```

Both normalize:

```text
Initialized
Progress
ActionStarted / ActionUpdated / ActionCompleted
UsageUpdated
ProviderRetry
FinalCandidate
ProviderError
UnknownEvent
```

Claude tracks nested stream events and content/tool blocks; only a successful `result` with valid `structured_output` can terminate successfully. Codex tracks thread/turn/item events; it requires one successful `turn.completed` and a valid private final-output file. The private file must be regular, non-link, inside the controller directory, bounded, and schema-valid.

Unknown fields and valid unknown events are forward-compatible. Unknown event data is reduced to type/hash/size. Non-JSON stdout in declared JSONL mode, malformed frames, missing/duplicate terminals, invalid schema, and exit/terminal disagreement are protocol failures. Stderr is arbitrary bounded diagnostic text and is redacted before persistence.

No raw chain-of-thought is stored or shown. Reasoning events become high-level progress only.

Alternatives considered:

- Convert non-JSON stdout to chat tokens. Rejected because a delegation protocol violation could be mistaken for success.
- Share one generic state machine between providers. Rejected because Claude block streams and Codex turn/item/final-file semantics differ.

### 16. Use one structured narrative schema and independent host evidence

Both targets receive `DelegationAgentReportV1`:

```text
schema_version = 1
outcome = completed | blocked | needs_input
summary
findings[]
actions_taken[]
verification_claims[]
risks[]
follow_ups[]
limitations[]
```

All fields and collections are bounded. The provider report is narrative evidence only. The host independently computes changed files/diff, observed actions and approvals, process exit/usage, sandbox/policy violations, and cleanup. Verification claims are labelled `provider_reported`; a host-run check, if added later, is a separate `locally_verified` observation.

Successful terminal predicate requires one valid provider terminal, zero process exit, valid final schema, no fatal protocol/sandbox violation, analyze immutability or admitted edit diff, successful Artifact sealing, and successful required cleanup. Timeout/cancellation/limit/cleanup failure never produces an applyable ChangeSet.

Alternatives considered:

- Trust the model's list of changed files and tests. Rejected because it is neither complete nor authoritative.

### 17. Enforce bounded delegation scheduling

Initial controller ceilings:

```text
global active delegations                  2
active delegation per OnePiece session    1
attempts per parent generation             3
global queue                               16
maximum queue wait                         10 minutes
analyze wall time                          15 minutes
edit wall time                             30 minutes
attempt events                             2,048
final structured output                    1 MiB
```

Additional stdout/stderr, clone disk, diff, file count, binary, process, and Artifact quotas are central policy constants captured in the attempt. Claude receives maximum turns and configured cost ceiling; Codex has no dependable hard dollar control, so duration/turn/output ceilings and reported usage are used. Request fields may lower limits only.

No hidden queue per model response: multiple delegation calls are processed deterministically, the active one runs, and excess calls receive concurrency-limit results. No automatic retry occurs.

Alternatives considered:

- Unlimited background delegations. Rejected because provider cost, disk, process, and cancellation ownership would become unbounded.

### 18. Seal an immutable complete ChangeSet

For a successful edit, the host creates `DelegationChangeSetV1`:

```text
artifact / delegation / attempt identity
repository identity / exact base commit
provider, CLI, adapter, prompt-schema fingerprints
file manifest with operations, modes, sizes, before/after hashes
full canonical binary-capable patch
diff hash and aggregate counts
provider report
host-observed verification and policy evidence
risk classification / limitations / applyability
```

The temporary-index calculation includes untracked output. Unsafe paths, control-plane files, submodules/gitlinks, unsupported links, excessive content, or an incomplete diff make it non-applyable. The Artifact is immutable. V1 applies the whole ChangeSet only; selecting or editing hunks would require a new derived ChangeSet with invalidated evidence and new approval.

Frontend presents a summary card, then a full review surface reusing the current file list and unified/split diff presentation. It distinguishes provider claims from host evidence. Pagination is allowed, irrecoverable truncation is not. The acknowledgement binds Artifact and diff hashes.

Alternatives considered:

- Return only a patch string in chat. Rejected because provenance, binary metadata, integrity, evidence, retention, and exact approval need structured durable identity.

### 19. Apply ChangeSets through an exact atomic transaction

`apply_delegation_changes` creates a separate `DelegationApplyAttempt`:

```text
requested -> awaiting_approval -> preflighting -> applying
          -> verifying -> applied
                      \-> rolling_back -> failed_rolled_back|recovery_required
```

The specialized review UI is the Approval Broker presentation; checking the exact-review acknowledgement and pressing “Apply these changes” resolves one Once approval. There is no second generic scope chooser.

Preflight acquires an exclusive canonical-workspace mutation lease and verifies repository identity, exact HEAD, clean index/worktree including untracked files, no merge/rebase/cherry-pick/bisect state, Artifact/diff integrity, safe paths, and current approval witness. It never cleans the workspace or modifies Git history.

Before mutation, a controller-private rollback capsule stores every touched path's bytes/type/mode/existence and the clean witness. The patch is checked, applied completely to the working tree without staging, and the resulting full diff is compared to the ChangeSet hash. Failure triggers restoration and verification. If exact restoration cannot be proven, the capsule is retained, the workspace enters `recovery_required`, and further automatic mutations are blocked until user review. No automatic reattempt occurs.

Alternatives considered:

- `git stash` then apply. Rejected because it mutates user state and complicates recovery.
- Three-way apply or automatic rebase. Rejected because the reviewed diff and verification evidence would no longer describe the result.
- Stage or commit automatically. Rejected because the tool only transfers reviewed working changes.

### 20. Use capability probes, fixtures, fake CLIs, and local circuit breaking

Add `DelegationReadiness` separate from ordinary provider capabilities. Passive probes canonicalize the executable/launcher, read version/help, verify required flags, sandbox/process-tree/Artifact backends, and optionally classify auth status without network or quota. Cache keys include binary fingerprints, adapter version, compatibility manifest, OS/architecture, and policy revision.

Version policy:

```text
below minimum                 blocked analyze/edit
tested range                 ready if all probes pass
newer than tested            degraded analyze; blocked edit
newer + explicit live canary ready for passed modes
unparseable/incompatible     blocked
```

Real sanitized versioned JSONL fixtures drive adapter golden tests. Fake Claude/Codex executables inject malformed output, unknown events, missing/duplicate terminals, exit mismatch, invalid schema, output floods, hangs, descendants, outside writes, symlink output, secret stderr, sealing crashes, and cleanup failures. Explicit live canaries run only after user/release consent because they contact providers and may cost money.

Repeated protocol, sandbox, process-tree, or cleanup integrity failures for one binary fingerprint open a local circuit. Ordinary provider refusal, task failure, bad answer, or project test failure does not. A fingerprint or compatibility-policy change resets the probe path, not historical audit records.

Alternatives considered:

- Trust semantic version alone. Rejected because flags and protocols can drift independently.
- Run a paid canary at every startup. Rejected because readiness must not create network cost or side effects.

### 21. Keep frontend state as projections of native aggregates

Extend `AgentService` and its Tauri/Web implementations with DTOs for capability readiness, browser handoff, operations, Artifact queries/publication/download, OCR results, delegation attempts/events, ChangeSet review, apply attempts, and recovery. React components never call `invoke()`.

UI surfaces:

- OnePiece capability diagnostics in Agent configuration;
- chat tool-activity progress and approval cards;
- managed browser/handoff view;
- Artifact preview and lineage view;
- task-center projections for long operations and delegation;
- full ChangeSet review/application dialog;
- recovery-required notice and bounded affected-file details.

Native aggregates remain authoritative. Events update queries optimistically, but revision gaps/stale witnesses trigger authoritative reload. Web/mock shares DTO validation and deterministic state transitions while returning `desktop_runtime_required` for real effects.

Manual delegation controls use the same native dispatcher, permission evaluation, approval witness, workspace authority, and lifecycle as provider-originated `delegate_cli` and `apply_delegation_changes` calls. Tauri commands do not invoke delegation ports directly, infer a writable target from Artifact/provider strings, or treat a UI acknowledgement as a substitute for the unified once-only approval.

Alternatives considered:

- Store operation truth in React state. Rejected because restart, cancellation, approval, and multi-surface consistency require native durable state.
- Put all details into assistant message JSON. Rejected because pagination, recovery, retention, and cross-surface queries need normalized ownership.

### 22. Persist bounded metadata and use unified logs only

Add additive SQLite migrations for Artifact metadata/links/references, tool-operation recovery facts where not already covered, delegation logical/attempt/event/apply records, readiness cache metadata, and workspace recovery gates. Large bytes, raw prompts/context, browser bodies, OCR text, source code, full stdout/stderr, full diffs, credentials, hidden reasoning, and raw external transcripts do not belong in rows or logs; they live only in bounded message results or protected Artifact blobs when product-visible.

Every domain emits `error`, `warn`, `info`, and `debug` through unified logging with operation/run correlation and redaction before persistence. SDK/CLI/process output remains available in bounded page-visible operation views while only safe summaries reach durable logs.

Alternatives considered:

- Create per-feature log files. Rejected by unified-log governance and because redaction/retention would drift.
- Persist raw transcripts for debugging. Rejected due to privacy, reasoning, secret, and size risks.

## Risks / Trade-offs

- [Risk] Playwright/Chromium installation and protocol versions drift independently from the app. → Pin reviewed SDK/browser revisions, fingerprint both, use captured fixtures and readiness probes, and remove only the affected tool when unavailable.
- [Risk] Human handoff exposes a managed browser window that users may mistake for their normal profile. → Use a distinct application-owned profile, persistent VaneHub branding/status, explicit handoff/resume controls, and no automatic profile attachment.
- [Risk] SSRF can occur through redirects, DNS rebinding, alternate IP encodings, proxies, or browser subresources. → Centralize URL/address policy, validate every hop and resolved peer, strip ambient proxy credentials, apply the policy in Rust and browser request interception, and add hostile-network fixtures.
- [Risk] Windows sandbox primitives may not support an installed runtime or may be weakened by host policy. → Make isolation capability-tested and fail closed; never fall back to shell; ship code execution after the Windows escape/failure matrix passes.
- [Risk] PDFium and managed Python/Node/CLI dependencies increase supply-chain surface. → Pin versions/checksums, use the existing SDK/dependency governance, record fingerprints, and keep installers and runtime invocations backend-owned.
- [Risk] Artifact storage can grow quickly from screenshots, PDFs, sandbox outputs, and ChangeSets. → Enforce per-operation/global quotas, content deduplication, reference-aware retention, visible usage, and idempotent cleanup.
- [Risk] Content-addressed blobs reveal equality through hashes. → Keep hashes inside authorized metadata surfaces, avoid public URLs, and do not use them as authorization secrets.
- [Risk] Search/provider HTML drift causes silent parsing errors. → Validate minimum structural invariants, use versioned fixtures, fail explicitly, and avoid switching providers silently.
- [Risk] Prompt injection in pages, repositories, OCR, or Artifacts influences the model. → Mark data provenance/authority, minimize context, and enforce all permissions, filesystem, network, and apply rules outside the model.
- [Risk] External CLI provider connectivity and child-command network isolation are difficult to separate on every OS/version. → Require target/mode readiness evidence, strict tool/MCP flags, OS sandbox enforcement, live canaries for edit, and block unsupported combinations.
- [Risk] A CLI update breaks JSONL output after passive probes pass. → Use strict terminal predicates, tolerate only valid unknown events, block edit for untested versions, and open a fingerprint-scoped circuit on integrity failures.
- [Risk] Rollback can fail after partial filesystem mutation. → Capture exact preimages, hold an exclusive mutation lease, verify rollback, retain recovery capsules, and block further automatic mutation until acknowledged.
- [Risk] A single large change is hard to ship safely. → Implement behind per-capability gates in dependency order; do not enable Browser, sandbox, OCR, delegation edit, or apply merely because their UI exists.
- [Trade-off] Plan mode excludes useful Web/Browser research and delegated analysis. → Preserve the existing no-arbitrary-network/no-external-agent Plan contract; users can use ordinary execution mode with read-only operation approvals when research is intended.
- [Trade-off] V1 does not partially apply ChangeSets. → Preserve evidence integrity and offer manual export; add derived ChangeSets only in a future separately specified change.
- [Trade-off] Artifact publication is application-local, not a public share service. → Provide safe preview/download now and defer external publishing credentials and retention policy.

## Migration Plan

1. Add domain contracts, fixed handler registry, shared DTOs, Web/mock parity, and disabled capability/readiness flags. Adapt existing baseline tools without changing externally observable behavior.
2. Add Artifact schema/blob store, integrity/retention/recovery tests, and internal producer/consumer ports. Keep provider-facing Artifact publication disabled until storage and preview tests pass.
3. Add Web search/fetch with hostile URL/redirect fixtures and enable only `web_search`/`web_fetch` for OnePiece after network-policy verification.
4. Add the Playwright sidecar, pinned dependency management, semantic inspection, screenshot Artifact flow, and managed handoff. Enable Browser separately after process-tree and SSRF matrices pass.
5. Add the platform sandbox abstraction and Windows backend, reviewed Python/JavaScript runtime probes, fake runtime fixtures, and Artifact I/O. Keep `code_execution` unavailable on any machine that fails isolation readiness.
6. Add the PaddleOCR inference worker and PDFium rasterizer behind the existing extension lifecycle, then enable `ocr` only after inference self-test and Artifact privacy tests pass.
7. Add CLI delegation persistence, independent clone preparation, prompt snapshots, target invocations, strict adapters, fake-CLI fault matrix, and analyze-only readiness. Enable explicit live canaries before real analyze delegation.
8. Add edit-mode diff sealing and ChangeSet review with application disabled. Validate real edit canaries and long-running cleanup/restart recovery.
9. Add once-only exact application, rollback capsules, workspace mutation leases, recovery gates, and destructive-failure injection. Enable apply last.
10. Run the complete repository validation suite, UI E2E for behavior changes, Windows native integration tests, OpenSpec strict validation, and manual Claude/Codex live compatibility checks before removing experimental gates.

Rollback is capability-based: disable the affected handler/readiness entry while preserving Artifact, attempt, approval, and recovery records. Additive migrations are not destructively reversed. In-flight owned processes are cancelled and reaped; unresolved apply recovery remains visible until inspected. No rollback step deletes user worktree changes or retained evidence automatically.
