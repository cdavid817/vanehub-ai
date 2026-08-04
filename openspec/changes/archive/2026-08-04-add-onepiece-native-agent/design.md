## Context

VaneHub already has a complete `launch_kind = api` execution path: API-Agent registration, credential storage, Anthropic and OpenAI-compatible streaming, tool execution and approval, Skills, memory, compaction, MCP tools, and Loop participation. The missing product layer is a stable first-party Agent identity. Today every API Agent is created by the user and couples its identity directly to one provider configuration.

Two current seams block OnePiece from behaving like a normal session Agent:

- the create-session dialog derives candidates from a fixed four-CLI id list instead of the service-backed registry;
- the native sessions context recognizes only `browser`, `native-desktop`, and `cli` when validating a selected Agent, even though the shared frontend and Agent Runtime types already include `api`.

System-prompt assembly currently joins optional bound Skills and scoped memories. OnePiece needs non-removable, versioned core instructions that remain present when no Skill is bound or Skill lookup fails. API-Agent availability also needs to reflect actual credential presence, not only SQLite model and Base URL columns.

The design crosses React, the frontend service adapters, Agent Runtime, Sessions, SQLite seed/migration behavior, and the OS credential boundary. It must preserve existing CLI agents and user-created API agents, avoid direct Tauri calls from components, and keep Web/mock behavior deterministic.

The Agent configuration page currently contains four tabs: OnePiece plus Claude Code, Codex CLI, and OpenCode. OnePiece already owns a 25-entry fixed-host catalog, while the three CLI tabs duplicate smaller provider-preset lists and catalog presentation. Cherry Studio's MIT-licensed provider registry models a vendor as a stable identity with a partial `endpointConfigs` map, and CC Switch records protocol-specific CLI endpoints plus explicit format conversion where its own proxy supports it. These references establish that endpoint protocol is a child of provider identity, not a property that should be collapsed into one vendor-wide Base URL.

## Goals / Non-Goals

**Goals:**

- Provide a stable built-in Agent identity with id `onepiece`, display name `OnePiece`, `launch_kind = api`, and API as its only interaction mode.
- Keep OnePiece visible before configuration, with a safe and actionable readiness state.
- Let users manage multiple named OnePiece provider Profiles selected from a built-in provider catalog aligned with CLI configuration, each with its own secure credential, and explicitly choose the single Profile used by runtime generation without replacing the Agent identity.
- Give the four Agent configuration tabs one shared 25-vendor directory, provider marks, catalog presentation, and protocol-aware endpoint metadata while retaining Agent-specific Profile persistence and application rules.
- Preserve every reviewed Anthropic Messages, OpenAI Chat Completions, and OpenAI Responses endpoint independently when a vendor publishes more than one protocol.
- Reuse the existing API generation, tools, approval, Skill, memory, compaction, MCP, and Loop implementations.
- Inject versioned OnePiece core instructions before optional Skills and memories on every generation.
- Discover create-session candidates from registry capabilities, create local OnePiece sessions using `interactionMode = api`, and enforce the same eligibility at the native boundary.
- Preserve desktop and Web/mock service-contract parity.
- Preserve existing data and behavior for the four CLI agents and ordinary user-created API agents.

**Non-Goals:**

- A general Agent Profile/template marketplace, import/export format, or arbitrary custom avatar system.
- A new provider protocol, general-purpose provider SDK, launch kind, or terminal runtime. Model discovery is limited to the fixed OnePiece catalog and existing Anthropic/OpenAI-compatible protocols.
- User editing or disabling of OnePiece core instructions in the first version.
- Per-session snapshots of provider configuration or core-instruction versions; existing sessions use the Agent's current configuration when generation starts.
- Remote-workspace execution for OnePiece. The first version supports local project folders and local Git worktrees because the native tool catalog does not provide remote shell/file execution for API agents.
- Changing ordinary user-created API agents' provider/interface immutability or delete rules.
- Arbitrary custom providers or user-entered Base URLs for OnePiece; provider identity, protocol, and endpoint come from the versioned built-in catalog.
- Inventing an Anthropic or OpenAI endpoint for a vendor that the reviewed references expose through only one protocol.
- Adding CC Switch's local Responses/Chat/Anthropic translation proxy in this increment. Endpoint compatibility is explicit and fail-closed until VaneHub owns and tests an equivalent conversion boundary.

## Decisions

### 1. OnePiece is a seeded Agent identity with Agent-specific provider Profiles, not another runtime

Add OnePiece to the native Agent seed catalog with stable id `onepiece`, `launch_kind = api`, mode `api`, provider placeholder `VaneHub`, and capability tags such as `agent`, `api`, `coding`, `tools`, and `memory`. It has no executable, terminal mode, browser mode, model, interface format, Base URL, or credential until configured.

The Web/mock registry seeds the same identity and starts it in the same unconfigured state. The frontend visual-identity catalog maps the stable id to the OnePiece icon/tone, while candidate eligibility still comes from registry metadata rather than an id allowlist.

This reuses the domain's existing stable Agent identity and API routing. OnePiece provider Profiles configure the upstream runtime used by that single identity; they do not instantiate additional Agents or introduce a general cross-Agent Profile abstraction.

**Alternative considered:** register OnePiece as an ordinary API Agent or model it as a built-in Skill. Rejected because an ordinary Agent is not persistently discoverable before credentials exist, while a Skill can be disabled, deleted, or fail lookup and therefore cannot carry mandatory identity instructions.

### 2. Record management origin explicitly and adopt a pre-existing API row on id collision

Add an additive `agent_origin` column with values `builtin` and `user`. Migration backfills the four seeded CLI ids as `builtin`, existing API agents as `user`, and future API registration writes `user`. The OnePiece seed writes `builtin`.

If an existing row already uses id `onepiece` and has `launch_kind = api`, migration adopts it in place as the built-in OnePiece identity. Its provider configuration, credential key, sessions, messages, Skills, memories, usage, Loop references, and trust setting remain attached to the same id. The seed ensures the API mode and required capability tags without overwriting a complete provider binding. If a non-API row somehow owns the reserved id, initialization fails safely with a diagnostic instead of silently changing its launch semantics.

Adoption avoids a cross-store rekey operation spanning every `agent_id` foreign/reference column and the OS credential key.

**Alternative considered:** reserve a different id such as `vanehub-onepiece`. Rejected because the product contract and downstream identity mapping require the stable `onepiece` id.

### 3. Store OnePiece provider Profiles separately and project the active Profile onto the Agent runtime row

Add an additive `onepiece_provider_profiles` table containing a stable Profile id, user-facing name, source provider id/version, source endpoint type, provider label, model id, interface format, optional Base URL, active flag, and timestamps. The table stores no credential material. Provider/interface/Base URL values are endpoint-derived runtime snapshots rather than user-authored endpoint configuration. A partial unique index permits at most one active Profile. The existing `agents` row remains the runtime projection: its provider/model/interface/Base URL fields mirror the active Profile, or return to the unconfigured `VaneHub` defaults when no Profile is active. This lets the existing API process gateway and registry readiness logic continue resolving OnePiece without a provider-specific execution branch.

Each Profile credential is stored through the OS credential service under a Profile-scoped account. The active credential is also projected to the existing `onepiece` runtime credential account so current readiness and provider execution ports remain compatible. Activation snapshots the previous active credential into its Profile account when required, loads the target Profile credential, then updates the runtime credential and SQLite projection with compensation on failure. Raw secrets never enter SQLite, DTOs, logs, or React state returned from services.

The service exposes a versioned provider directory plus OnePiece Profile list, save, activate, and delete operations. A save request supplies the selected provider id, endpoint type, Profile name, model id, and optional replacement credential; the application service rejects unknown/deprecated providers, absent endpoint types, or endpoint protocols unsupported by the native runtime, then resolves provider/interface/Base URL and discovery metadata before persistence. Saving a first Profile activates it automatically; later Profiles remain inactive until explicitly activated. Editing keeps the selected provider and endpoint immutable; switching either is represented by adding another Profile. Editing an active Profile updates both the Profile and runtime projection. Deleting an inactive Profile leaves runtime state unchanged. Deleting the active Profile requires confirmation, clears the runtime projection and credential, and leaves remaining Profiles inactive rather than silently choosing another provider. The existing reset operation remains an explicit remove-all compatibility operation and preserves the OnePiece identity and all Agent references.

An additive migration converts a complete legacy OnePiece binding into one active Profile with a deterministic id and name. It records matching provider and endpoint ids only when the legacy provider/interface/endpoint exactly matches a known endpoint configuration; otherwise it preserves a nullable legacy source so the Profile can continue to run, be activated, or be deleted without exposing a custom-provider creation path. Editing a legacy Profile requires choosing a supported provider endpoint. Because SQLite migration cannot read the OS credential store, the first Profile-aware service operation lazily copies the existing active credential into that Profile's scoped credential account before a provider switch. Unconfigured OnePiece rows create no Profile.

Catalog resolution owns provider/interface/Base URL validation for OnePiece, while model and credential validation remain part of Profile saving. Ordinary user-created API-Agent lifecycle behavior and its custom Base URL support remain unchanged.

### 4. Decorate registry reads with credential-aware API readiness

Keep SQLite mapping responsible for structural configuration and add a credential-aware registry decorator at the infrastructure/composition boundary. It wraps `AgentRegistryRepository`, `ApiCredentialPort`, and unified logging and adjusts API-Agent availability without making a provider network call:

- missing model or required Base URL: `unavailable` with a configuration reason;
- complete structural configuration but missing credential: `needs-auth`;
- complete configuration and credential present: `available`;
- credential-store access failure: non-selectable state with a safe reason plus a redacted warning log.

The decorated registry is shared by Agent Runtime selection, Loop eligibility, the session-eligibility adapter, and list/get projections so every entry point observes the same readiness. Ordinary API agents gain the credential-presence check required by the existing `api-agent-runtime` specification.

**Alternative considered:** store a `credential_configured` boolean in SQLite. Rejected because it can diverge from the OS credential store and would not prove actual credential presence.

### 5. Core instructions are an application port backed by a compile-time versioned asset

Add a narrow `AgentCoreInstructionsPort` returning optional `{version, content}` for an Agent id. The native adapter serves OnePiece content from a repository-owned Markdown asset compiled with `include_str!`; other agents return no core instructions. The Web adapter uses the same stable version identifier and a deterministic mock marker without reproducing secrets or calling a provider.

API prompt assembly becomes:

```text
OnePiece core instructions (when present)
Bound and enabled Skills (when present)
Scoped memories (when present)
```

Each source is a distinct delimited section. Core instructions are resolved independently and cannot be removed by Skill lookup failure. Skill and memory failures retain their existing best-effort behavior and logging. The complete assembled prompt stays outside conversation turns and compaction. The core version is attached to safe generation diagnostics/prompt tracing so behavior can be correlated with an application release without logging prompt content.

For non-OnePiece API agents, no core section is returned; if they also have no Skills or memories, generation continues with no system prompt exactly as today.

**Alternative considered:** seed `onepiece-core` as a mandatory Skill. Rejected because current Skill lifecycle permits disabling/deletion and intentionally degrades to generation without the Skill.

### 6. Session discovery is capability-driven; presentation ordering may recognize built-ins

Replace `preferredAgentIds` eligibility with a pure frontend selector over `AgentRegistryEntry[]`. A candidate must declare `cli` or `api` interaction support. Ready candidates are selectable; non-ready OnePiece remains visible and disabled with an action that opens its configuration surface. Other non-ready candidates may remain visible with their existing reason rather than disappearing.

Presentation groups are:

1. built-in CLI agents in Codex CLI → Claude Code → Gemini CLI → OpenCode order, with Codex CLI as the default selectable candidate;
2. VaneHub native (`onepiece`), immediately below the built-in CLI group;
3. user API agents.

Known built-in ids may define presentation order and visual identity, but they do not decide eligibility. Selecting OnePiece sets `agentId = onepiece` and derives `interactionMode = api` from its declared modes. The shared `CreateSessionInput` shape is unchanged.

On the Agent configuration page, the three CLI configuration tabs precede OnePiece and the OnePiece tab is placed last. Direct navigation to OnePiece still selects it regardless of tab position.

When OnePiece is selected, the first version permits local folders and local Git worktrees and disables remote workspace selection with a localized explanation. CLI behavior is unchanged.

### 7. Sessions owns a narrow eligibility port backed by the decorated Agent registry

Split Agent validation out of the current SQLite-oriented `SessionCreationContextPort` into a narrow sessions-owned `SessionAgentEligibilityPort`. Its native adapter receives the same decorated `AgentRegistryRepository`, loads the stable id, parses the requested interaction mode through the shared domain enum, and calls the Agent's selectability rule. This removes the duplicated `browser | native-desktop | cli` string allowlist and admits `api` only when the selected Agent declares it and is ready.

The adapter depends on the registry and credential abstractions, not the full Agent Runtime service, avoiding a composition cycle: the registry and credential adapters are constructed first, Sessions receives the eligibility adapter, and Agent Runtime is then composed with the Sessions gateway.

The Sessions application layer also rejects the unsupported OnePiece-plus-remote-workspace combination. Web/mock session creation enforces the equivalent candidate, mode, readiness, and workspace rules in memory.

**Alternative considered:** update the native string allowlist to include `api`. Rejected because it would still accept configuration-blind requests and continue duplicating Agent Runtime capability rules.

### 8. Lifecycle protection is enforced in application and persistence layers

The generic delete API rejects built-in API agents using `agent_origin`, with OnePiece-specific localized UI guidance to reset configuration instead. The repository repeats the guard so bypassing the UI or command layer cannot delete the identity. Ordinary user API agents retain existing reference checks and deletion behavior.

OnePiece configuration mutations are exposed through `AgentService`; React components never call Tauri directly. `tauri-agent-client.ts` is the only frontend layer invoking the new native commands. `web-agent-client.ts` maintains non-secret mock state and the same validation/results.

### 9. Safe defaults and observability remain inherited, not special-cased in execution

OnePiece starts with `auto_approve_tools = false`. Enabling trust uses the existing explicit confirmation flow; MCP calls and plan-mode restrictions remain unchanged. Once ready, the existing composite API process gateway handles its generations with no `onepiece` branch in provider execution.

Setup, reset, readiness failure, prompt-version selection, and rejected session creation emit structured events through unified logging. Logs include stable Agent id and safe configuration metadata but never API keys, credential values, full core instructions, Skill bodies, memory bodies, or raw provider payloads.

### 10. OnePiece provider setup follows the shared CLI configuration hierarchy

The OnePiece tab uses the established CLI configuration composition: compact status and toolbar controls, search/filter-ready provider-card lists, persistent active emphasis, and application-owned dialogs for add/edit/delete/activate operations. An unconfigured OnePiece shows an “Add configuration” primary action and an empty state instead of exposing every field permanently. Creating a Profile starts with the same searchable official/common provider-catalog hierarchy used by CLI configuration, but OnePiece omits the custom-provider action. Each card shows Profile name, catalog provider, model, resolved endpoint, credential presence, readiness, and active state.

The add/edit dialog reuses the established provider catalog, overlay, form spacing, sticky actions, loading prevention, and narrow-viewport behavior of CLI profile dialogs while retaining OnePiece-specific validation and secret handling. New Profiles select a built-in catalog provider; provider and Base URL are never free-text fields. The selected preset supplies the default model, while the model remains editable for compatible models offered by that provider. Stored API keys are never repopulated; editing may submit an optional replacement key. Saving a non-first Profile does not activate it. Applying a Profile and deleting a Profile use application-owned confirmation dialogs rather than browser prompts.

Profile activation affects only which upstream provider future OnePiece generations use. It does not select a different Agent, mutate stable id `onepiece`, or alter the current Session. Existing in-flight generations retain their start-time configuration snapshot.

**Alternative considered:** reuse `CliConfigProfileDialog` and `CliConfigProfile` directly. Rejected because CLI Profiles render provider-specific CLI files and support import/drift semantics, while OnePiece Profiles feed the native API runtime. The interaction hierarchy is shared, but contracts and persistence remain bounded to their owning contexts.

### 11. A shared versioned provider directory owns vendor identity and multiple protocol endpoints

Replace the OnePiece-only flat preset list and duplicated CLI provider identity facts with one reviewed, versioned 25-vendor directory consumed by frontend Agent configuration and compiled into the native OnePiece runtime. Each provider owns its stable id, display name, category, icon key, provider label, default and fallback models, and help links. Each provider also owns a partial `endpointConfigs` map keyed by `anthropic-messages`, `openai-chat-completions`, or `openai-responses`; an endpoint record owns its immutable Base URL, runtime interface mapping, model-discovery strategy/URL, authentication strategy, and source note. A provider may expose one, two, or three endpoint types. Absence means unsupported and is never filled by URL suffix inference.

The initial endpoint matrix is reviewed against Cherry Studio `packages/provider-registry` and CC Switch's Claude/Codex/OpenCode presets. Cherry's endpoint registry is the primary structural reference; CC Switch is the CLI-compatibility reference where it exposes a distinct coding endpoint. Conflicts are resolved conservatively and recorded in provenance. Only fixed HTTPS hosts that use the existing Anthropic Messages, OpenAI Chat Completions, or OpenAI Responses configuration contracts are included. Gemini/Vertex/Bedrock/Azure-specific transports, OAuth-only identities, embedding-only providers, and local/custom hosts remain excluded.

Agent-specific adapters translate the directory rather than duplicating it:

- OnePiece exposes endpoint selection for `anthropic-messages` and `openai-chat-completions`; `openai-responses` remains unavailable until its native API runtime supports it.
- Claude Code selects a published `anthropic-messages` endpoint. An OpenAI endpoint is not presented as directly compatible because VaneHub does not yet own CC Switch's conversion proxy.
- Codex CLI prefers `openai-responses`, falls back to `openai-chat-completions`, and does not claim a direct Anthropic endpoint without an explicit bridge.
- OpenCode maps Anthropic endpoints to `@ai-sdk/anthropic`, first-party OpenAI endpoints to `@ai-sdk/openai`, and other OpenAI-compatible endpoints to `@ai-sdk/openai-compatible`.

Provider search, category filtering, catalog cards, endpoint badges/selection, help links, and brand rendering become shared React components. Agent-specific Profile forms remain separate because CLI import/drift/file-writing semantics differ from OnePiece credential and activation semantics.

Provider marks are copied without visual alteration from Cherry Studio's `@cherrystudio/ui` icon package, whose package manifest declares MIT, using light/dark variants when provided and documented filename aliases for the 25-vendor set. The repository includes the applicable MIT license text, exact upstream paths/revision, and a trademark/non-affiliation notice. Missing or ambiguous marks retain the initials fallback rather than inventing artwork.

**Alternative considered:** flatten every provider to one preferred Base URL and let each Agent rewrite it. Rejected because the Anthropic and OpenAI endpoints frequently use different paths and sometimes different adapter behavior. **Alternative considered:** port CC Switch's protocol-conversion proxy as part of catalog reuse. Rejected for this increment because request/stream/tool/reasoning conversion is a separate runtime and security boundary, not provider metadata.

### 12. Model discovery is a credential-aware native operation with a static fallback

Add a narrow service operation that accepts a provider id and endpoint type plus either a saved Profile id or a transient replacement API key. The native application resolves the compiled endpoint record, obtains the Profile-scoped credential when needed, and delegates HTTP listing to a proxy-aware infrastructure port. It never accepts a discovery URL, Base URL, parser, or arbitrary header from React.

Discovery strategies initially cover Anthropic's model-list shape, the common OpenAI `data[].id` shape, and catalog-only fallback. Provider-specific variants remain explicit catalog strategies rather than URL heuristics. Requests use catalog-owned HTTPS endpoints, no redirects, bounded timeout/response/model counts, redacted unified logs, and no persistence of response bodies or credentials. Results are trimmed, deduplicated, sorted, filtered for known non-chat modalities, and merged with bundled fallback models so configuration remains possible when the optional upstream list is incomplete or unavailable.

The add dialog enables discovery after a new credential is entered. Editing an existing Profile can refresh with its stored credential or a transient replacement. The model control is a searchable selector rather than free text. A legacy or previously selected model absent from the latest result remains visible with an unavailable/legacy marker and is not silently replaced. The Web/mock adapter exposes the same contract using deterministic catalog models and performs no network or secret persistence.

**Alternative considered:** fetch `/models` directly in React. Rejected because it would expose credentials to component networking, duplicate provider authentication logic, bypass proxy/logging controls, and violate the service boundary.

### 13. API-key verification is a shared minimal provider request with Agent-owned resolution

Add one shared native provider-probe adapter that accepts only an already validated effective protocol, endpoint, model, and secret. It supports Anthropic Messages, OpenAI Chat Completions, and OpenAI Responses and sends a minimal request with no conversation history, system prompt, tools, file content, or application context. The request caps generated output at one token, uses the proxy-aware no-redirect client, has a 15-second timeout, and is not retried so a user action cannot create duplicate provider cost. The adapter reads at most a bounded error body for native classification and never returns or logs that body.

Resolution remains owned by each configuration context. OnePiece resolves provider, endpoint, and model from its fixed directory plus the selected Profile or transient dialog fields, and loads a Profile-scoped credential only when no transient replacement is supplied. CLI configuration validates the submitted draft through the existing payload validator, or loads an existing Profile and its scoped credential; Claude Code maps to Anthropic Messages, Codex maps its `wire_api` to Responses or Chat Completions, and OpenCode uses its reviewed source preset endpoint type or its supported SDK/provider shape for an existing custom Profile. The probe command never accepts a second arbitrary URL independent of the Profile payload already governed by the owning save contract.

The result is an ephemeral discriminated state shared by all four settings surfaces:

- `valid`: the provider accepted authentication and returned a successful response;
- `invalid-credential`: HTTP 401 or 403;
- `configuration-rejected`: HTTP 400, 404, or 422, which may indicate the selected model or endpoint rather than a bad key;
- `rate-limited`: HTTP 429;
- `provider-unavailable`: timeout, DNS/TLS/connectivity failure, or HTTP 5xx;
- `unsupported`: the Profile uses an authentication mode without an API key or a protocol the validator does not implement.

Other HTTP statuses remain safely inconclusive instead of being guessed as valid or invalid. Validation does not save the transient credential, mutate the Profile, activate/apply it, change Agent readiness, or persist a “last valid” flag because credentials and provider policy can change after any check. Unified logs record only Agent id, safe Profile/provider identity, protocol, classification, and latency; they exclude credentials, authorization headers, prompts, response bodies, and query strings.

The React layer reuses one validation action/status component in CLI and OnePiece dialogs and Profile cards. Dialog checks prefer the current transient credential and current draft; card checks use the saved scoped credential. Each request has a run id and cancellation/stale-result guard so changing provider, endpoint, model, or credential cannot let an older result overwrite current UI state. Web/mock implements the same contract with deterministic, non-network classification and never retains transient secrets.

**Alternative considered:** reuse CC Switch's current reachability check. Rejected because it deliberately treats 401/403 as reachable and cannot answer whether a key is accepted. **Alternative considered:** use model discovery as key validation. Rejected because catalog-only and permission-limited model-list endpoints can fail while generation succeeds, and the existing discovery path intentionally degrades to fallback models. **Alternative considered:** persist validation status. Rejected because it becomes stale and could incorrectly imply that an expired or revoked key remains valid.

## Risks / Trade-offs

- **[Risk] A historical user-created API Agent already owns `onepiece`.** → Adopt that API row in place, preserve its references and credential, mark it built-in, and add missing modes/tags; reject only an impossible/non-API collision.
- **[Risk] Credential-store reads during registry listing add latency or can fail.** → Use a narrow existence check/decorator, keep the registry bounded, never call the provider, map per-Agent failures to a safe non-selectable state, and log without failing unrelated Agent entries.
- **[Risk] Core instructions plus Skills and memories can grow the provider prompt.** → Give each section an explicit independent character budget in the specs/design implementation tasks, preserve deterministic truncation/omission, and log only ids/versions and sizes.
- **[Risk] A core-instruction update changes behavior in old sessions.** → Version the asset and trace the version; accept current-configuration semantics for the first version rather than introducing session snapshots.
- **[Risk] Provider reconfiguration during an active generation creates mixed expectations.** → Snapshot the resolved provider configuration and core version at generation start; subsequent generations use the new configuration.
- **[Risk] SQLite activation and multiple credential-store writes cannot share one transaction.** → Use ordered compensation, keep the old Profile active until target credentials are proven available, and map partial failures to a safe non-selectable state with redacted unified logs.
- **[Risk] Deleting the active Profile could silently route requests through another provider.** → Clear the active runtime projection and require the user to explicitly activate a remaining Profile.
- **[Risk] CLI and OnePiece provider choices drift.** → Derive the OnePiece catalog from the same reviewed provider definitions where protocols are compatible, expose it through `AgentService`, version every entry, and cover native/Web catalog parity with contract tests.
- **[Risk] A provider is incorrectly advertised through a protocol it does not publish.** → Store a partial endpoint map, require an evidence/source note per endpoint, never synthesize URLs, and test each Agent adapter against explicit protocol compatibility.
- **[Risk] Cherry Studio or CC Switch changes an endpoint after release.** → Snapshot reviewed facts under a catalog version, record upstream repository revisions, and update through a reviewed catalog diff rather than consuming either project at runtime.
- **[Risk] A provider's model endpoint returns embeddings, image/audio models, or incomplete capability data.** → Apply provider-specific exclusions, merge reviewed fallback models, preserve unknown existing selections, and never claim that an unverified discovered model supports capabilities beyond chat execution.
- **[Risk] Model-list network failures leak credentials or prevent setup.** → Keep requests native, catalog-addressed, proxy-aware, bounded and redacted; return bundled fallback models with a safe warning instead of logging headers, URLs containing secrets, or response bodies.
- **[Risk] A credential check is mistaken for a durable health guarantee or creates avoidable cost.** → Keep results ephemeral, label configuration and transient failures separately from invalid credentials, cap output at one token, avoid retries, and state that successful verification proves only that the selected request was accepted at that moment.
- **[Risk] Bundled brand icons introduce license or trademark issues.** → Track provenance per asset, use official or permissively licensed sources, retain vendor marks unmodified where required, and fall back to initials when provenance is unclear.
- **[Risk] Reset spans SQLite and the OS credential store without a distributed transaction.** → Make SQLite unusable first, compensate configuration-save failures, and treat leftover inaccessible credentials as orphaned secrets that are logged safely and never used.
- **[Risk] Capability-driven discovery exposes agents not intended for chat sessions.** → Require explicit `cli` or `api` interaction support and native selectability; browser/native-desktop-only entries are not session candidates.
- **[Trade-off] OnePiece cannot target remote workspaces initially.** → Keep the UI explicit and reject the combination at the service boundary until remote API-agent tools have their own capability and specification.

## Migration Plan

1. Add `agent_origin` additively and backfill existing seeded CLI agents and user API agents.
2. Detect an existing `onepiece` row: adopt it if it is API-based; fail safely if its launch kind is incompatible.
3. Extend seed data with OnePiece, its API mode, tags, and default unconfigured/safe state. Startup seeding upgrades both clean and existing databases idempotently.
4. Ship registry readiness decoration and OnePiece configuration/reset operations before exposing it as a selectable session candidate.
5. Add service adapters and UI: OnePiece settings, visual identity, dynamic candidate grouping, disabled setup state, and local-session selection.
6. Enable native/Web `api` session validation through the shared eligibility rules and add regression coverage for existing CLI session creation.
7. Verify migrations and rollback against clean, pre-OnePiece, configured-API-Agent, and existing-`onepiece` fixtures.
8. Add the Profile table and migrate a complete legacy binding into one active Profile; lazily preserve its credential in the Profile-scoped account before the first switch.
9. Replace single-binding service/UI contracts with list/save/activate/delete Profile operations in native and Web adapters while retaining reset as remove-all compatibility behavior.
10. Replace the derived OnePiece provider list with the reviewed runtime-specific catalog, add provider-icon assets and provenance, then introduce service-backed model discovery before removing the model text input.
11. Migrate the flat provider catalog to the shared 25-vendor `endpointConfigs` model, backfill OnePiece Profile endpoint ids by exact match, generate compatible CLI presets through adapters, and replace duplicated catalog/icon presentation across all four Agent configuration tabs.

Rollback to an older application leaves the additive column and seeded OnePiece row in SQLite. Older create-session UI ignores it because of its fixed CLI list; the unconfigured row cannot generate successfully, and user data remains intact. Reinstalling the new version reseeds any removed metadata. No destructive down migration is required.

## Resolved Implementation Details

- The reviewed core-instruction asset ships as semantic version `1.0.0` and is bounded to 8,000 Unicode characters.
- Each injected Skill is bounded to 8,000 Unicode characters and the aggregate Skill section is bounded to 16,000 characters; oversized or non-fitting Skills are skipped as whole items in deterministic binding order.
- Resolved: OnePiece appearing in the candidate list does not replace the built-in CLI default. Codex CLI is the first/default built-in CLI candidate. A ready OnePiece is selected by default only when the user explicitly selected it previously; an unavailable prior selection falls back to Codex CLI or the next selectable built-in CLI.

## Implementation Inventory

The stable `agents.id` is referenced by Sessions, Skill bindings, memories, usage records,
Loop worker/verifier definitions, scheduled tasks, coordination/workflow state, and their
fixtures. API-Agent configuration crosses the Agent Runtime application service and gateway,
SQLite registry, Tauri command DTO/mapper/registry, TypeScript contracts and both native and
Web service adapters. The OnePiece migration therefore adopts an existing API row in place and
never rewrites its id, while deletion protection remains enforced at both application and
repository boundaries.
