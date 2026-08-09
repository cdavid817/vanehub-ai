# Multi-CLI Provider Runtime Architecture Exploration

## Executive conclusion

The supplied direction is sound, but the repository is already halfway to the target. VaneHub has an `agent_runtime` bounded context, an Agent catalog, provider-neutral process and terminal ports, shared headless process supervision, retained PTY management, normalized generation events, fixture-backed provider output parsing, reported/estimated usage accounting, and a separate `tooling::cli` context.

The missing abstraction is not a brand-new Runtime. It is a provider strategy seam beneath the existing application ports. Provider identity still leaks into Session chat configuration, invocation building, output decoding, terminal usage discovery, Windows executable normalization, policy projection, tooling catalogs, frontend types, model lists, icons, and configuration forms.

The source document is stale in one important respect: the repository now supports five built-in CLI Agents, including `antigravity-cli`. Its archived change is useful empirical evidence: adding it touched roughly fourteen capabilities and many hard-coded rosters. That is the extensibility failure this refactor should address.

## Current architecture

```text
React UI
  │  AgentService
  ├─────────────── Tauri adapter ───────────────┐
  └─────────────── Web/mock adapter             │
                                                ▼
                                      generic Tauri commands
                                                │
                     ┌──────────────────────────┼─────────────────────────┐
                     ▼                          ▼                         ▼
                Sessions context          Agent Runtime             Tooling::CLI
                SQLite/messages           application               detect/install/version
                lifecycle/config          generation/terminal       config/parameters
                     │                          │                         │
                     └──── Sessions gateway ◀──┤                         │
                                                ▼                         │
                                  CompositeAgentProcessGateway            │
                                    │                  │                  │
                           launch.kind = cli   launch.kind = api          │
                                    │                  │                  │
                                    ▼                  ▼                  │
                         RuntimeAgentProcess   ApiAgentProcessAdapter      │
                         Adapter                                        │
                                    │                                     │
                  agent-id matches in invocation/output                   │
                                    │                                     │
                                    ▼                                     │
                       shared spawn/monitor/kill/logging ◀─────────────────┘
                                    │
                       GenerationProcessEvent
                                    │
                          messages / usage / UI

Interactive path:
Session → AgentTerminalGateway → PortablePtyAgentTerminalRuntime
        → provider invocation match → wrapper → PTY → raw terminal UI
        → provider parser/session capture + provider-specific usage readers
```

### Existing reusable seams

- `agent_runtime/domain/catalog.rs`: stable Agent ids, launch metadata, interaction modes, availability and free-form capability tags.
- `agent_runtime/application/ports.rs`: `AgentRegistryRepository`, `AgentAvailabilityGateway`, `AgentProcessGateway`, `AgentTerminalGateway`, `AgentSessionGateway`, event/logging/task ports.
- `agent_runtime/infrastructure/process_adapter.rs`: common headless spawn, stdin/stdout/stderr, child registry, monitoring, cancellation, telemetry and unified logging.
- `agent_runtime/infrastructure/terminal_process.rs`: common retained PTY lifecycle, attach/input/resize/stop, transcript retention and terminal events.
- `agent_runtime/infrastructure/providers/`: provider invocation specs, parser selection, normalized tokens/thinking/tools/session ids/completion/failure and fixtures.
- `sessions`: VaneHub Session/message persistence, nullable `runtime_session_id`, chat configuration snapshots, reported versus estimated usage records.
- `tooling::cli`: executable discovery, installation sources, version checks, conflicts and lifecycle operations.
- `tooling::cli_config` and `tooling::cli_parameters`: global native config/profile management and launch parameter definitions.
- `src/services/agent-service.ts`: correct frontend service boundary; components do not need a new native integration path.

### Current provider paths

| Provider | Headless invocation | Output/session path | Interactive/usage path | Main configuration/UI dependencies |
|---|---|---|---|---|
| Claude Code | stdin; `-p --output-format stream-json --include-partial-messages --verbose`; `--resume` | dedicated Claude JSONL parser; text, thinking, tools, rich blocks, session id, result usage/error | PTY; caller-assigned `--session-id` or resume; reads `~/.claude/projects/.../<session>.jsonl` | Claude config payload/settings JSON, parameter flags, permission hook, models/reasoning, icon |
| Codex CLI | stdin; `exec [resume <id>] --json -`; special final-output capture | shared structured parser with Codex-specific `turn.completed` usage; `thread.started`; reasoning/tool events | PTY `resume <id>`; post-hoc rollout lookup and cumulative token total | Codex TOML/auth profile, reasoning/sandbox/approval parameters, models, icon |
| Gemini CLI | prompt argv; `--resume`; `-p <prompt> -o stream-json` | shared structured parser with Gemini `result.stats`; session id/reasoning/tool inference | PTY; assigned `--session-id` or resume; reads Gemini chat JSONL and sums message deltas | parameter catalog and models; no relay-style config profile; icon |
| OpenCode | prompt argv; `run --session <id> --format json <prompt>` | shared structured parser with `step_finish.part.tokens`; session/tool inference | PTY `--session`; discovers session from OpenCode SQLite and reads totals | JSON/JSON5 provider profile, env projection, parameters, models, icon |
| Antigravity CLI | prompt argv; `--conversation <id> -p <prompt> --output-format stream-json` | dedicated NDJSON envelope parser; `init` conversation id, result status and usage | PTY `--conversation`; session capture supported; interactive usage not implemented | local JSON settings profile, script-only install, parameters/policy, default model, icon |

### Session model

- VaneHub `SessionRecord.id` is the product Session id.
- `agent_id` selects the primary Agent; multi-seat Sessions also persist ordered seats, with `agent_id` mirroring seat zero.
- `runtime_session_id: Option<String>` stores one provider-native resume id per VaneHub Session.
- messages are VaneHub-owned conversation history. Headless generations rely on provider-native resume when available rather than replaying all history.
- launch kind is Agent catalog metadata (`cli`, `api`, browser, native desktop); `CompositeAgentProcessGateway` currently branches only between API and CLI.
- Session lifecycle (`idle/starting/running/failed/stopped`) is separate from Generation lifecycle (`reserved/active/completed/failed/cancelled`) and retained terminal state.

### Usage model

- Headless provider parsers normalize completion payloads into input/output/cache-read/cache-create totals; reasoning/thinking is folded into output.
- Successful reported data is persisted as `accounting_kind = reported`, `unit = tokens`; absent/degenerate data falls back to character-count estimates.
- Terminal usage is discovered from provider-owned local artifacts: Claude JSONL, Codex rollout JSONL, Gemini chat JSONL, and OpenCode SQLite. Antigravity terminal usage is absent.
- `source` is currently a free-form string such as `cli-session-log`; accounting quality is already typed separately. Cost and duration are not part of the normalized record.
- The active `add-gemini-cli-terminal-usage-tracking` change overlaps this area and should finish or be reconciled before extracting terminal usage strategies.

### Tooling model

The source document's proposed boundary mostly already exists. Detection, executable resolution, version/install state and native configuration belong in `tooling`; runtime invocation, resume behavior, decoding and process lifecycle belong in `agent_runtime`. The current weakness is duplicated provider catalogs and direct Agent-id matches within both contexts, not incorrect top-level ownership.

### Frontend dependencies

- `AgentRegistryEntry` already carries launch metadata, availability and `capabilityTags`, and `AgentService.listAgents()` is generic.
- `ManagedCliAgentId` and equivalent arrays hard-code five ids in both contracts and types.
- `cli-parameter-catalog.ts` duplicates the Rust parameter catalog.
- `chat-configuration.ts`, `useChatConfig.ts`, `components/chat/models.ts`, icon components and settings pages contain provider-specific decisions.
- Some branches are legitimate provider-specific editors; the defect is that generic selection/configuration code owns them instead of rendering provider-declared schema or a provider-specific extension component.

## Problems by severity

### P0

| Problem | File location/current implementation | Why it blocks extension | Direction |
|---|---|---|---|
| Session domain owns provider identities | `sessions/domain/chat_configuration.rs` defines `ChatAgent`, vendor/default-model mappings and model alias/reasoning rules | Every CLI addition changes the provider-neutral Session domain; API and CLI model semantics are conflated | Move runtime/model defaults and capability constraints behind provider/catalog descriptors; keep Session validation generic |
| Provider strategy is centralized in id matches | `providers/invocation.rs`, `providers/output.rs`, and `process_adapter.rs` match ids for args, resume, parser, Codex capture, MCP insertion and OpenCode shims | A new provider edits shared hot paths and can regress all existing CLIs | Resolve a registered provider strategy and migrate each branch behind it incrementally |
| Terminal runtime mixes generic lifecycle with provider mechanics | `terminal_process.rs` combines PTY supervision, invocation, session capture, four usage readers and Windows shim resolution in a file over 1,500 lines | New CLIs repeatedly modify core terminal lifecycle and enlarge failure surface | Keep PTY supervision shared; inject invocation/session-discovery/usage/executable-normalization strategies |
| One resume id cannot represent multi-seat providers | `sessions::SessionRecord.runtime_session_id` and Agent Runtime `AgentSession.runtime_session_id` store one string while Sessions may contain multiple Agent seats | Different providers/seats can require independent native conversations; reuse can attach the wrong provider context | Add provider/seat-scoped session references in a later migration; first change only makes the current id opaque |

### P1

| Problem | File location/current implementation | Why it blocks extension | Direction |
|---|---|---|---|
| Built-in CLI rosters are duplicated | Rust schema seed, tooling definitions, CLI parameters/config ids, permission lists, TypeScript contracts/types, Web mocks and UI catalogs each enumerate ids | Lists drift; Antigravity already required wide coordinated edits | Registry becomes runtime source; expose descriptors through existing service boundary; retain specialized catalogs only where behavior truly differs |
| Structured protocols are over-generalized | `output.rs` shares one generic JSON parser for Codex, Gemini and OpenCode with embedded provider-specific usage branches | Similar-looking event names can have different semantics; protocol drift may silently degrade | One decoder per provider composed from JSONL helpers; normalize a minimal event set and retain bounded/redacted raw events |
| Terminal usage is a provider switch | `terminal_process.rs::run_terminal_usage_ingestion` dispatches to local-artifact readers | Adding a provider changes the generic terminal runtime and polling eligibility list | Declare an optional terminal usage strategy/capability per provider |
| Parameters/config are only partly metadata-driven | Rust `cli_parameters.rs`, TypeScript `cli-parameter-catalog.ts`, tagged `CliConfigPayload` and provider-specific forms | New CLI requires backend and frontend edits; drift is likely | Make launch parameter schema backend-owned; retain explicit provider config extensions for formats that cannot be faithfully expressed generically |
| Capability tags are too weak for runtime/UI decisions | Agent catalog stores string tags, while model/reasoning/resume/usage/terminal support is inferred elsewhere | Generic callers cannot safely decide what to show or request | Add typed provider capabilities; project them to frontend descriptors in a later change |

### P2

| Problem | File location/current implementation | Why it matters | Direction |
|---|---|---|---|
| “Provider” has two meanings | Agent catalog `provider`, chat `provider_id`, API endpoint providers, and proposed runtime provider | Easy to key the registry or persistence against the wrong identity | Name the runtime key `AgentProviderId`; document model/API provider ids separately |
| Error classification remains shallow | invocation has only unsupported Agent; process failures collapse into strings with retryable/non-retryable | Generic UI cannot distinguish install/auth/config/protocol failures consistently | Add a staged provider error taxonomy after the contract; preserve safe Tauri mapping |
| Usage provenance is a string | `AgentUsageRecord.source` is free-form; cost/duration absent | Comparisons across partially observable CLIs are hard | Add typed provenance only when consumers need it; do not fabricate precision |

## Assessment of the supplied proposal

### Reuse directly

- The `agent_runtime` versus `tooling::cli` boundary.
- Static registration before any dynamic plugin design.
- Capability-driven callers, shared process ownership, decoder fixtures, opaque native session ids, partial usage and strangler migration.
- Contract and architecture tests as extensibility guardrails.

### Adjust to the repository

- Evolve `AgentProcessGateway`, `AgentTerminalGateway`, `ProviderOutputEvent`, `GenerationProcessEvent`, and existing process adapters rather than creating a parallel Runtime API.
- Treat five current CLI providers as the baseline, not four.
- Keep the Agent catalog separate from the runtime provider registry.
- Do not add a stateful `AgentSession` trait in the first change; the name and lifecycle are already occupied.
- Put shared process runtime under `agent_runtime::infrastructure` and reuse `platform::process` instead of creating a new bounded context.
- Preserve existing reported/estimated usage accounting; add typed provenance/cost/duration only with evidence and a consumer.

### Do not adopt mechanically

- A giant boolean capability struct containing speculative MCP/subagent/web/image features. Start with capabilities used by current runtime/UI decisions.
- A wholly generic provider-parameter form for native config documents. Claude JSON, Codex TOML and OpenCode JSON5 have drift/credential semantics that may require explicit provider extensions.
- Unbounded raw event forwarding. Raw events must be size-bounded, redacted, and excluded from persistence/UI by default under unified logging rules.
- A second provider/session/process stack alongside existing ports.

## Target architecture

```text
AgentService (unchanged)
  ├─ Tauri adapter ── generic commands
  └─ Web/mock adapter
                         │
                         ▼
                Agent Runtime application
                         │ stable Agent id
                         ▼
                  ProviderRegistry
                         │
         ┌───────────────┼─────────────────┐
         ▼               ▼                 ▼
    CLI provider    API provider       Native provider
    strategies      strategies         strategies
         │
         ├─ invocation + resume strategy
         ├─ decoder factory
         ├─ optional terminal strategy
         ├─ optional usage strategy
         └─ typed capabilities/readiness prerequisites
                         │
                         ▼
          shared Agent Runtime infrastructure
       process spec → stdio / JSONL / PTY transport
       spawn / I/O / cancel / cleanup / logging / tracing
                         │
                         ▼
             normalized integration events
                UI / persistence / usage / logs

Tooling::CLI remains adjacent:
detection / executable / version / install / global config / parameters
```

## Brownfield migration strategy

1. Introduce the provider contract and compatibility registry with zero behavior change.
2. Make provider session references explicit, then migrate storage to provider/seat scope before relying on multi-provider seats.
3. Consolidate existing headless and PTY mechanics around a shared process spec and transport contracts.
4. Split the current parser into provider decoders and normalize events without changing frontend DTOs.
5. Migrate one provider at a time. Codex is the recommended pilot because it exercises stdin, resume subcommands, JSONL, reasoning, tools, usage and a special output file; Claude follows to validate permission-hook and custom parser behavior.
6. Move terminal usage discovery behind optional provider strategies after the active Gemini usage change is reconciled.
7. Expose descriptors through `AgentService` in both Tauri and Web/mock adapters, then convert generic frontend surfaces to capabilities/schema.
8. Add Aider only after all existing provider identity matches are either migrated or explicitly classified as provider-specific extensions.

Each step keeps old and new paths side-by-side behind compatibility adapters, pins behavior with existing fixtures, and removes only the migrated branch. No big-bang rewrite is required.

## Recommended OpenSpec changes

| Name | Why / scope | Out of scope | Dependencies | Main risk | Acceptance criterion |
|---|---|---|---|---|---|
| `introduce-agent-provider-contract` | Typed descriptors/capabilities/session ref, static registry, five compatibility providers, architecture tests | Process/event/UI migration | none | wrapper-only abstraction | all five resolve through registry; behavior fixtures unchanged |
| `scope-provider-session-references` | Persist provider/seat-scoped native session references | history rewrite | contract | data migration and legacy ambiguity | each active seat/provider resumes only its own native session |
| `consolidate-cli-process-runtime` | Extract `ProcessSpec`, stdio/PTY transports, supervision and cancellation from current adapters | provider decoding | contract | PTY and stdio lifecycle mismatch | providers describe launch; shared runtime exclusively owns OS processes |
| `normalize-agent-provider-events` | Per-provider decoders, minimal Agent event protocol, bounded/redacted raw fallback | frontend redesign | contract, process runtime | schema drift/raw leakage | all current fixtures map deterministically; unknown events remain safe |
| `migrate-codex-cli-provider` | First complete strategy migration including output capture and terminal usage hook | other CLIs | prior three | Codex special cases | shared files contain no Codex identity branch except registration/fixtures |
| `migrate-claude-code-provider` | Move Claude invocation, hook, resume and decoder behavior | other CLIs | Codex pilot | permission-hook regression | Claude behavior contract passes with no shared identity branch |
| `migrate-gemini-cli-provider` | Move Gemini argv, session id, decoder and usage behavior | other CLIs | event/process runtime; reconcile active usage change | concurrent overlap | Gemini behavior is provider-local and fixture-compatible |
| `migrate-opencode-provider` | Move OpenCode argv/env/SQLite session and decoder behavior | other CLIs | event/process runtime | native DB/schema drift | OpenCode-specific logic is provider-local |
| `migrate-antigravity-cli-provider` | Move NDJSON envelope, script-only metadata and conversation handling | adding new CLI | event/process runtime | evolving CLI protocol | Antigravity-specific runtime branches are provider-local |
| `unify-cli-terminal-usage-strategies` | Optional provider usage reader and typed provenance | cost UI | provider migrations | local artifact drift | terminal runtime has no provider-id usage switch |
| `expose-agent-provider-descriptors` | Add descriptor/readiness/capability DTOs to Tauri and Web/mock service adapters | UI behavior | contract | adapter parity drift | both adapters return contract-conformant identical shapes |
| `make-provider-ui-capability-driven` | Dynamic selector, model/reasoning/resume/terminal/parameter decisions | full redesign | descriptors | generic form loses native semantics | generic components contain no built-in id branches; extensions are isolated |
| `add-aider-provider` | Extensibility proof through public provider/tooling extension points | Goose/Crush/Qwen/Plandex | migrations and UI | hidden core dependency remains | Aider adds provider/tooling registration, metadata, decoder, tests/docs without Session/generic command/generic UI edits |

## First-change size check

`introduce-agent-provider-contract` is independently implementable and testable. It adds types, a registry, five compatibility declarations, composition wiring and guardrails. It deliberately does not change process ownership, event DTOs, terminal usage, frontend services, SQLite shape or CLI behavior. The expected production diff is confined primarily to `agent_runtime` plus composition and architecture tests; if implementation begins editing Session chat rules, terminal usage readers or generic React components, the change has exceeded its boundary.

