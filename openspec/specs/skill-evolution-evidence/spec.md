# skill-evolution-evidence Specification

## Purpose
Creates a privacy-first and auditable evidence layer that converts structured Agent outcomes into attributable Skill-improvement signals and deterministic candidate seeds without changing any Skill.

## Requirements

### Requirement: Structured evidence source envelope
The evidence pipeline SHALL accept only versioned structured source envelopes from registered runtime boundaries. Each envelope SHALL contain a stable source-event id, event type, occurrence time, stable Agent and run correlation when available, terminal or verification classification, observed effective Skill revisions, fidelity, workspace scope, and bounded safe fields. It SHALL NOT require a raw log file, prompt, transcript, hidden reasoning, command body, tool result body, or file body.

#### Scenario: Native API execution event
- **WHEN** a native API run emits a registered structured terminal, tool, verification, feedback, or delegation event
- **THEN** the evidence boundary SHALL receive its safe correlation fields and exact observed Skill revision associations

#### Scenario: Managed CLI terminal event
- **WHEN** a managed CLI run emits a structured process or verification outcome
- **THEN** the evidence boundary SHALL receive the outcome plus the captured binding and mount snapshot available for that run

#### Scenario: Interactive CLI is opaque
- **WHEN** an interactive CLI exposes only an opaque terminal boundary
- **THEN** the envelope SHALL identify opaque fidelity and SHALL NOT invent tool, Skill-use, or verification details

#### Scenario: Unknown event version
- **WHEN** the pipeline receives a future unsupported source-envelope version
- **THEN** it SHALL reject evidence ingestion safely without affecting the originating Agent execution

### Requirement: Six deterministic signal extractors
The pipeline SHALL run exactly six versioned deterministic extractor families: explicit user feedback, execution and tool failure, verification outcome, retry and recovery delta, delegated Utility outcome, and Skill usage or lifecycle anomaly. An extractor SHALL emit zero or more bounded signals without invoking an LLM.

#### Scenario: Explicit feedback extracted
- **WHEN** a user submits helpful, unhelpful, or correction feedback for a completed assistant message
- **THEN** the explicit-feedback extractor SHALL create a signal linked to that message and its observed Skill revision set

#### Scenario: Failure extracted
- **WHEN** a structured Agent, provider, process, tool, permission, timeout, limit, or sandbox event ends unsuccessfully
- **THEN** the failure extractor SHALL create a classified negative signal without copying raw error output

#### Scenario: Verification extracted
- **WHEN** a correlated test, build, lint, type, security, specification, or acceptance verification completes
- **THEN** the verification extractor SHALL create a positive or negative signal containing safe verifier identity, status, and bounded counts

#### Scenario: Recovery delta extracted
- **WHEN** a failed attempt is followed by an independently identified retry or repair attempt for the same redacted task fingerprint
- **THEN** the retry/recovery extractor SHALL create a delta signal that references both attempt outcomes without duplicating their content

#### Scenario: Delegated Utility outcome extracted
- **WHEN** a Utility delegation reaches a terminal state
- **THEN** the delegation extractor SHALL create a signal linked to the canonical Utility revision, parent Agent, limits, tool and approval counts, and safe status

#### Scenario: Usage anomaly extracted
- **WHEN** structured usage shows repeated load refusal, repeated unavailable dependency, repeated conflict, repeated budget omission, or sustained use without successful completion correlation
- **THEN** the lifecycle extractor SHALL create a bounded anomaly signal only after its deterministic threshold is reached

### Requirement: Signal classification
Every signal SHALL record extractor id and version, category, polarity, severity, evidence strength, source fidelity, occurrence and ingestion time, workspace scope, safe summary, source references, observed Skill associations, redaction version, deduplication key, and lineage status. Classification values SHALL be closed enums and unknown values SHALL fail validation.

#### Scenario: Negative tool failure
- **WHEN** a bounded tool failure signal is persisted
- **THEN** it SHALL carry a failure category, negative polarity, classified severity, source fidelity, and attribution strength rather than an unstructured error string

#### Scenario: Successful verification
- **WHEN** a verification passes after a correlated recovery
- **THEN** the signal SHALL carry positive polarity and SHALL remain linkable to the preceding negative evidence

#### Scenario: Cancellation is neutral by default
- **WHEN** a user cancels an Agent run without a structured failure or negative feedback
- **THEN** the pipeline SHALL classify cancellation as neutral lifecycle evidence and SHALL NOT treat it as a Skill defect automatically

### Requirement: Skill attribution levels
Each observed Skill association SHALL be classified as `verified`, `correlated`, `weak`, or `unattributed` with a machine-readable rationale. Attribution SHALL use recorded runtime facts and SHALL NOT infer that a configured or visible Skill was used when no such fact exists.

#### Scenario: Native injected revision verified
- **WHEN** native prompt assembly records an eager Skill revision in the exact generation that produced the source event
- **THEN** association to that Skill revision SHALL be verified

#### Scenario: Native loaded revision verified
- **WHEN** the exact generation successfully loads a Role Skill or starts a Utility delegation
- **THEN** association to the loaded or delegated canonical revision SHALL be verified

#### Scenario: CLI mount snapshot correlated
- **WHEN** a CLI run starts with a captured effective mount snapshot but the CLI does not expose which mounted Skill influenced its output
- **THEN** associations to mounted revisions SHALL be correlated rather than verified

#### Scenario: Configured binding only is weak
- **WHEN** the runtime knows a CLI binding exists but lacks a captured active mount snapshot for the source run
- **THEN** the association SHALL be weak

#### Scenario: No Skill evidence
- **WHEN** no injected, loaded, delegated, or active mounted Skill fact exists
- **THEN** the signal SHALL remain unattributed

### Requirement: Targeting eligibility from attribution
The pipeline SHALL mark verified evidence as eligible for later automated target consideration, correlated evidence as human-review-only target evidence, and weak or unattributed evidence as ineligible for automatic Skill targeting. This change SHALL NOT select a target Skill.

#### Scenario: Verified seed hint
- **WHEN** a seed contains verified evidence for one canonical Skill revision
- **THEN** it MAY expose that identity as a verified target hint for a later selector

#### Scenario: Correlated CLI seed hint
- **WHEN** a seed contains only correlated CLI associations
- **THEN** any Skill hints SHALL be marked human-review-only and SHALL NOT be eligible for later automatic application

#### Scenario: Weak CLI evidence
- **WHEN** a signal contains only weak or unattributed associations
- **THEN** its seed SHALL contain no target Skill identity and SHALL remain generic evidence

### Requirement: Twelve-class privacy sanitization
Before any signal content is persisted, fingerprinted, logged, queried, seeded, or exported, the pipeline SHALL apply a versioned sanitizer covering exactly these twelve redaction classes: private-key blocks; API, access, refresh, and session tokens; authorization headers and cookies; password, secret, and credential assignments; credential-bearing URLs; credential-bearing connection strings; secret environment-variable values; user-home and profile paths; email addresses; phone numbers; IP addresses and internal hostnames; and cloud, tenant, subscription, account, or project identifiers.

#### Scenario: Multiple sensitive classes
- **WHEN** one bounded source summary contains a token, email address, and user-home path
- **THEN** all three values SHALL be replaced before the summary reaches durable evidence state

#### Scenario: Redaction before fingerprinting
- **WHEN** a source field contains a sensitive value used to derive a task or deduplication fingerprint
- **THEN** the value SHALL be sanitized before fingerprint computation so the fingerprint is not derived directly from the secret

#### Scenario: Private key detected
- **WHEN** input contains a private-key block
- **THEN** the sanitizer SHALL omit the block and emit only its redaction class and bounded count

#### Scenario: Safe structured metadata retained
- **WHEN** stable internal ids, enum status, counts, durations, safe hashes, and canonical Skill ids do not match a redaction class
- **THEN** the sanitizer SHALL retain them within field limits

### Requirement: Non-reversible redaction markers
Redaction markers SHALL identify only the redaction class and an installation-scoped non-reversible correlation token when correlation is permitted. They SHALL NOT contain the original value, a reversible encoding, or a globally comparable unsalted hash.

#### Scenario: Same secret in one installation
- **WHEN** the same sensitive value appears in two permitted fields on one installation
- **THEN** the sanitizer MAY emit the same installation-scoped marker to support deduplication

#### Scenario: Evidence exported or copied
- **WHEN** a marker leaves its original installation through a user-authorized export in a future capability
- **THEN** it SHALL not enable comparison against markers generated by another installation

### Requirement: Bounded content policy
Evidence records SHALL default to metadata-only capture. Explicit feedback and configured redacted summaries SHALL be sanitized and bounded before storage. The pipeline SHALL never persist raw prompts, full conversations, hidden reasoning, unrestricted commands, tool arguments, tool results, terminal output, file contents, credentials, or full absolute paths in its own tables.

#### Scenario: Failure has raw stderr elsewhere
- **WHEN** a source event references a managed operation whose raw output is presented elsewhere under existing policy
- **THEN** evidence SHALL store only correlation id, safe classification, counts, and sanitized bounded summary rather than copying stderr

#### Scenario: Correction feedback exceeds limit
- **WHEN** a user correction note exceeds the evidence text limit
- **THEN** feedback submission SHALL reject or require shortening before durable evidence persistence rather than silently storing the full note

#### Scenario: Redacted capture disabled
- **WHEN** optional redacted content capture is disabled
- **THEN** non-feedback signals SHALL contain metadata templates only and no free-form source summaries

### Requirement: Idempotent signal ingestion
The pipeline SHALL persist at most one logical signal per source-event id, extractor id and version, and signal discriminator. Replayed source events, application restart, or duplicate adapter delivery SHALL not create duplicate evidence.

#### Scenario: Source event delivered twice
- **WHEN** the same source envelope is delivered twice to the same extractor version
- **THEN** the second ingestion SHALL return the existing signal identity without incrementing counts

#### Scenario: Extractor version changes
- **WHEN** a newer extractor version intentionally reprocesses a retained compatible source reference
- **THEN** resulting signals SHALL be distinguishable by version and SHALL preserve supersession lineage

#### Scenario: Partial extractor failure
- **WHEN** one extractor fails while other extractors can process the same envelope
- **THEN** successful signals SHALL persist, the failed extractor SHALL record a safe diagnostic, and the source Agent execution SHALL remain unaffected

### Requirement: Deterministic task fingerprints
Task and pattern fingerprints SHALL be derived only from sanitized normalized categories, stable safe ids, coarse operation shape, and bounded user-authorized feedback summary. They SHALL be installation-scoped and SHALL not make raw task text recoverable.

#### Scenario: Retry shares fingerprint
- **WHEN** two attempts have matching sanitized task identity and operation shape within one workspace
- **THEN** they SHALL receive the same installation-scoped task fingerprint for recovery correlation

#### Scenario: Similar text in different workspace
- **WHEN** similar tasks occur in separate workspaces
- **THEN** their workspace-scoped fingerprints SHALL not cause cross-project seed grouping by default

### Requirement: Deterministic candidate-seed construction
The seed builder SHALL group compatible retained signals by workspace scope, category, sanitized task fingerprint, compatible observed Skill revision hints, evidence strength, and configured time window. It SHALL create bounded immutable candidate seeds using deterministic templates and SHALL NOT invoke an LLM, select a final target, or propose Skill content.

#### Scenario: Repeated independent failures
- **WHEN** at least two non-duplicate negative signals from distinct runs share a compatible grouping key
- **THEN** the builder SHALL create or update one candidate seed referencing both signals

#### Scenario: Explicit correction seed
- **WHEN** one sanitized explicit correction has verified Skill attribution
- **THEN** the builder MAY create a candidate seed from that single high-value signal with its single-source status visible

#### Scenario: Failure followed by successful recovery
- **WHEN** a negative signal and positive recovery signal share a task fingerprint and verified Skill association
- **THEN** the seed SHALL preserve both polarities and identify the recovery delta without inferring the corrective instruction

#### Scenario: Incompatible Skill revisions
- **WHEN** otherwise similar signals refer to incompatible canonical Skill revisions or different workspaces
- **THEN** they SHALL not merge unless the seed records separate version cohorts explicitly

#### Scenario: No actionable pattern
- **WHEN** a signal remains neutral, duplicated, isolated below threshold, or unattributed without explicit feedback
- **THEN** it SHALL remain queryable evidence but SHALL not create a ready candidate seed

### Requirement: Candidate-seed lineage
Every candidate seed SHALL contain seed id and version, contributing signal ids, extractor and sanitizer versions, grouping key hash, workspace scope, categories, polarities, severity distribution, attribution distribution, verified and human-only target hints, source-Agent distribution, first and last occurrence, independent-run count, and readiness state.

#### Scenario: Inspect seed lineage
- **WHEN** a seed is queried
- **THEN** the service SHALL return bounded contributing signal summaries and exact lineage metadata sufficient to reproduce deterministic grouping

#### Scenario: Source signal purged
- **WHEN** a purge removes any signal contributing to a seed
- **THEN** the system SHALL remove or rebuild that seed transactionally so it never claims unavailable lineage

#### Scenario: Seed builder rerun
- **WHEN** the builder reruns with unchanged retained signals and version
- **THEN** it SHALL produce the same grouping and SHALL not duplicate seed versions

### Requirement: Evidence retention and quotas
First-version evidence signals and candidate seeds SHALL have a 90-day local retention window. The system SHALL enforce bounded global and per-workspace record and byte quotas, prefer retaining explicit feedback and verified verification or recovery evidence over weak anomalies during pressure, and expose dropped and expired counts.

#### Scenario: Evidence expires
- **WHEN** a signal and its dependent seed lineage exceed 90 days
- **THEN** retention SHALL delete or rebuild them transactionally without changing source messages, logs, traces, Skill usage, or execution records owned by other capabilities

#### Scenario: Workspace quota reached
- **WHEN** accepting more evidence would exceed a workspace quota
- **THEN** the system SHALL evict or reject the lowest-retention-priority evidence according to deterministic policy and SHALL not block the originating Agent

#### Scenario: High-value evidence protected
- **WHEN** quota pressure can be relieved by discarding weak lifecycle anomalies instead of explicit correction or verified recovery evidence
- **THEN** weak anomalies SHALL be discarded first

#### Scenario: Retention maintenance fails
- **WHEN** scheduled evidence retention fails
- **THEN** the system SHALL emit a rate-limited redacted unified diagnostic and SHALL not affect Agent execution

### Requirement: Scoped evidence purge
Users SHALL be able to purge evidence globally or by canonical workspace, canonical Skill id, source stable Agent id, time range, and evidence kind. Purge SHALL remove dependent seeds and evidence-only feedback projections without deleting source conversations, execution records, permission audits, logs, Skill usage, or Skill and Overlay content.

#### Scenario: Purge one Skill's evidence
- **WHEN** a user confirms purge for a canonical Skill id
- **THEN** the system SHALL delete signals associated with that Skill according to purge scope and rebuild or remove dependent seeds

#### Scenario: Purge unattributed evidence
- **WHEN** a user purges unattributed evidence
- **THEN** attributed evidence and its seeds SHALL remain unchanged

#### Scenario: Purge failure
- **WHEN** a purge transaction fails
- **THEN** the system SHALL leave either the complete pre-purge or complete post-purge evidence state and return a safe error

### Requirement: Runtime fail-open isolation
Evidence extraction, sanitization, persistence, seed construction, retention, query projection, or diagnostics failure SHALL never fail, delay beyond a bounded enqueue operation, retry, cancel, or alter the originating Agent, CLI, verification, delegation, or feedback operation.

#### Scenario: Evidence database unavailable
- **WHEN** the evidence repository cannot accept a runtime envelope
- **THEN** the source operation SHALL continue normally and the runtime SHALL increment a bounded drop counter plus a rate-limited unified diagnostic

#### Scenario: Evidence queue full
- **WHEN** the bounded ingestion queue is full
- **THEN** the source operation SHALL drop the evidence envelope according to priority without blocking the Agent execution thread

#### Scenario: Feedback persistence fails
- **WHEN** a user explicitly submits feedback and evidence persistence fails
- **THEN** the feedback operation SHALL report failure to the user because feedback was not saved, while the completed assistant message remains unchanged

### Requirement: Read-only evidence queries
The service SHALL provide bounded paginated summaries and details for collection status, signal funnel, extractor counts, attribution distribution, source-Agent distribution, category, polarity, severity, Skill revision, candidate seeds, retention, quotas, dropped counts, and lineage. Queries SHALL enforce workspace and Skill scope and SHALL not expose prohibited raw content.

#### Scenario: Query Skill evidence summary
- **WHEN** a user requests evidence for one canonical Skill in an accessible workspace context
- **THEN** the service SHALL return only scoped bounded counts, distributions, recent sanitized signals, and seeds associated with that identity

#### Scenario: Query unattributed pool
- **WHEN** a user requests unattributed evidence
- **THEN** the service SHALL return generic signals and no fabricated Skill identity

#### Scenario: Unknown workspace
- **WHEN** a query references an unknown or inaccessible workspace
- **THEN** it SHALL not return evidence belonging to another workspace

### Requirement: Observable evidence degradation
The native runtime MUST distinguish unavailable evidence from absent feedback and SHALL expose a safe diagnostic when credential, processor, or repository initialization fails.

#### Scenario: Feedback storage query fails
- **WHEN** message feedback cannot be loaded because its repository fails
- **THEN** the command SHALL NOT silently present every message as having no feedback and SHALL return or record a safe classified failure

#### Scenario: Evidence pipeline cannot initialize
- **WHEN** the installation key or ingestion processor cannot be initialized
- **THEN** the runtime SHALL expose disabled health with a safe reason and SHALL write a redacted unified diagnostic
