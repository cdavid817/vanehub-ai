## Purpose

Defines safe local/private OpenAI-compatible endpoint Profiles and deterministic Hybrid Routing so Agent work can use local models without overstating capabilities, privacy, capacity, usage, or fallback safety.

## ADDED Requirements

### Requirement: Endpoint Profiles are explicit and provenance-bearing
Each endpoint Profile SHALL identify its owning Agent, runtime kind, Base URL, interface format, model id, optional credential reference, timeout, privacy classification, declared and verified capabilities, and context-window value with source and confidence. The runtime MUST NOT infer context capacity or capabilities from the model id alone.

#### Scenario: Save a manual local Profile
- **WHEN** a user saves a structurally valid OpenAI-compatible local or private endpoint Profile
- **THEN** the system SHALL persist its non-secret fields and optional credential reference through the existing Agent Runtime boundary
- **AND** it SHALL label user-entered capability and context metadata as configured rather than verified

#### Scenario: Model name resembles a known cloud model
- **WHEN** an endpoint reports a model id that also exists in a reviewed catalog
- **THEN** the system SHALL NOT copy catalog capacity or capabilities unless the Profile is bound to that reviewed endpoint and metadata source

### Requirement: Local discovery is explicit and bounded
Automatic discovery SHALL run only after an explicit user action, probe a versioned allowlist of loopback endpoints with bounded concurrency and timeout, and inspect readiness/model metadata only. It MUST NOT scan LAN addresses, upload task or repository content, or execute a generation.

#### Scenario: Probe common localhost services
- **WHEN** the user starts local discovery
- **THEN** the desktop runtime SHALL probe only allowlisted `localhost`, `127.0.0.1`, or `[::1]` service URLs for Ollama, LM Studio, vLLM, SGLang, and generic OpenAI-compatible metadata
- **AND** results SHALL identify endpoint, interface, models, metadata provenance, latency bucket, and safe failure category

#### Scenario: A discovered response is malformed or slow
- **WHEN** a probe times out, exceeds its response bound, redirects outside loopback, or returns malformed metadata
- **THEN** that candidate SHALL be rejected without blocking other probes
- **AND** raw response bodies, credentials, and source content SHALL NOT enter logs

#### Scenario: Web runtime discovers endpoints
- **WHEN** local discovery is requested in Web/mock mode
- **THEN** the adapter SHALL return deterministic simulated operation results without network access or implying native readiness

### Requirement: Manual private endpoints do not weaken discovery restrictions
The system SHALL allow a user to enter a valid HTTP or HTTPS enterprise/private Base URL, but automatic discovery MUST remain loopback-only and the UI MUST distinguish manual trust from verified service identity.

#### Scenario: Save an enterprise endpoint
- **WHEN** a user manually confirms a syntactically valid non-loopback private endpoint
- **THEN** the system SHALL save it as user-configured and require explicit verification before marking it ready
- **AND** it SHALL NOT scan neighboring hosts or claim that the endpoint is local or secure

### Requirement: Verification is side-effect-free and metadata-only
Endpoint verification SHALL use bounded readiness or model-metadata requests appropriate to the configured interface, SHALL NOT send conversation content or invoke tools, and SHALL publish asynchronous operation state through the shared service boundary.

#### Scenario: Verify an OpenAI-compatible endpoint
- **WHEN** a user verifies a Profile whose service supports model listing
- **THEN** verification SHALL confirm reachable protocol shape and return bounded model/capability metadata
- **AND** it SHALL NOT send a chat completion

#### Scenario: Model listing is unsupported
- **WHEN** a reachable endpoint does not support a model-list operation
- **THEN** verification SHALL report readiness as inconclusive or manually configurable rather than issuing a real task

### Requirement: Hybrid Routing is deterministic and visible
The first Hybrid Routing policy SHALL map a versioned task class to a preferred Profile and optional fallback Profile, SHALL be user-visible and disableable, and SHALL record the selected rule and bounded reason for each routed generation.

#### Scenario: Route an eligible summarization task
- **WHEN** enabled rules classify a task as summarization and its preferred local Profile is ready and capable
- **THEN** the runtime SHALL capture that Profile for the generation and record the matching rule and reason

#### Scenario: Routing is disabled
- **WHEN** the user disables Hybrid Routing
- **THEN** generation SHALL use the explicitly active Profile and SHALL NOT select another Profile from task class

#### Scenario: No rule matches
- **WHEN** no enabled rule matches a task
- **THEN** generation SHALL use the explicitly active Profile and record a no-match reason without guessing a task class

### Requirement: Privacy policy controls fallback
Every routed task SHALL have one data policy of `cloud-allowed`, `local-preferred`, or `local-only`. A `local-only` task MUST NOT send any prompt, context, tool payload, or derived content to a Profile classified as cloud.

#### Scenario: Local-only preferred Profile is down
- **WHEN** a `local-only` task's selected local Profile is unavailable
- **THEN** the system SHALL stop before provider contact and enter an actionable waiting-for-user-choice outcome
- **AND** it SHALL NOT automatically use a cloud fallback

#### Scenario: Local-preferred Profile is down
- **WHEN** a `local-preferred` task's preferred Profile is unavailable and its configured fallback is ready and policy-compatible
- **THEN** the runtime SHALL use the fallback and record the failure category and fallback reason

#### Scenario: Cloud fallback conflicts with policy
- **WHEN** a fallback Profile's privacy classification conflicts with the task policy
- **THEN** routing SHALL reject that fallback before request construction

### Requirement: Runtime capabilities are negotiated before execution
Profiles SHALL explicitly represent support for text generation, tool calling, image input, structured output, and reasoning fields as configured, verified, unsupported, or unknown. The runtime MUST NOT emit unsupported request fields or pretend unknown support is available.

#### Scenario: Tool calling is unsupported
- **WHEN** a generation requires tools and the routed Profile declares tool calling unsupported
- **THEN** the runtime SHALL reject or select a policy-compatible capable fallback before provider contact
- **AND** it SHALL NOT silently run a tool-less request as equivalent work

#### Scenario: Optional reasoning field is unavailable
- **WHEN** the selected Profile does not support a reasoning field
- **THEN** the request SHALL omit that field while preserving ordinary text generation

### Requirement: Context-limit failures are bounded
The runtime SHALL use the selected Profile's effective context budget before request construction and SHALL classify provider context-limit failures without unbounded retries.

#### Scenario: Projected request exceeds the effective budget
- **WHEN** protected request content cannot fit the Profile's conservative or verified context budget
- **THEN** generation SHALL return the existing typed overflow path before provider contact

#### Scenario: Provider rejects context after estimation
- **WHEN** an endpoint rejects a request as over context limit despite local estimation
- **THEN** the runtime SHALL perform at most one policy-authorized reduced retry and SHALL otherwise fail with actionable capacity evidence

### Requirement: Usage and billing remain evidence-based
Missing or malformed provider usage SHALL remain unavailable or explicitly estimated under the existing accounting policy, and local/private Profiles without a reviewed price SHALL NOT produce fabricated billing cost.

#### Scenario: Local stream omits usage
- **WHEN** a successful local streaming response contains no valid usage object
- **THEN** the response SHALL remain successful with reduced usage coverage
- **AND** billed cost SHALL remain unavailable

### Requirement: Streaming remains responsive and bounded
Large local streaming responses SHALL use the existing asynchronous generation event path with bounded frame, queue, persistence, and rendering work so the UI remains responsive.

#### Scenario: Fake local server emits a large stream
- **WHEN** a deterministic fake endpoint emits a large valid response in varied chunk partitions
- **THEN** content order and terminal state SHALL be correct
- **AND** structural performance tests SHALL enforce bounded buffered bytes and work per flush independently of shared-runner timing

### Requirement: Hybrid operations use unified observability
Discovery, verification, routing, fallback, capability rejection, timeout, and context-limit outcomes SHALL use existing operation lifecycle and unified logging contracts with redaction.

#### Scenario: Endpoint authentication fails
- **WHEN** verification or generation receives an authentication failure
- **THEN** page-visible operation state SHALL expose a safe category and retry action
- **AND** logs SHALL exclude credentials, headers, raw response bodies, prompts, code, and full private URLs beyond the permitted origin metadata

