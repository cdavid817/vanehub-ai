## Purpose

Defines privacy-safe evidence dossiers and a constrained model-assisted workflow that produces locally rendered, fully validated Skill mutation drafts or quarantined new Skill proposals for human governance.

## ADDED Requirements

### Requirement: Generation input eligibility
The system SHALL create a generation request only from a current sanitized assessment and complete evidence lineage. Existing-Skill generation SHALL require an assessed target; new-Skill proposal generation SHALL require a strong no-target result whose non-target quality checks remain reviewable and sufficient.

#### Scenario: Current assessed target is available
- **WHEN** a Curator candidate lacks a draft and has a complete current target assessment
- **THEN** the system may create an existing-Skill generation request bound to that exact assessment and target revision

#### Scenario: Evidence or assessment is stale
- **WHEN** the source seed, assessment, target, or lineage is missing, purged, superseded, or incomplete
- **THEN** the system refuses generation without reconstructing or guessing the missing input

### Requirement: Exactly thirteen evidence dossier sections
Every evidence dossier SHALL contain exactly these ordered sections: identity and provenance; executive summary; candidate seed; source signal inventory; attribution and target selection; assessment and quality gates; current effective Skill snapshot; relevant guidance and resource context; failure, recovery, and verification timeline; privacy and redaction report; proposed mutation rationale; verification plan; and lineage and version witnesses.

#### Scenario: Complete dossier is built
- **WHEN** all registered source records are available
- **THEN** the dossier contains one versioned entry for each of the thirteen sections in the required order

#### Scenario: Optional section data is absent
- **WHEN** a section has no applicable safe records
- **THEN** the section remains present with an explicit not-applicable or unavailable reason rather than being omitted

### Requirement: Deterministic local dossier construction
The system SHALL build dossiers locally from authoritative sanitized records using a versioned schema, stable ordering, bounded projections, and content hashes. Identical source witnesses and builder versions MUST produce identical dossier bytes.

#### Scenario: Dossier is rebuilt unchanged
- **WHEN** source witnesses and builder version are identical
- **THEN** the resulting section hashes and complete dossier hash are identical

#### Scenario: Source revision changes
- **WHEN** evidence, assessment, effective Skill, policy, or sanitizer witness changes
- **THEN** the system creates a new dossier revision and preserves the prior revision as superseded

### Requirement: Dossier privacy boundary
Dossiers MUST exclude raw prompts, full provider transcripts, terminal output, tool arguments, credentials, source-file content, private paths, raw model payloads, and unsafe rejected text. All bounded free text SHALL pass the evidence sanitizer before dossier persistence, export, model use, logging, or display.

#### Scenario: Source contains multiple sensitive classes
- **WHEN** a referenced evidence record contains sensitive values
- **THEN** the dossier contains only structured metadata and non-reversible redaction markers

#### Scenario: Sanitizer version is inconsistent
- **WHEN** a source projection was sanitized by an incompatible or unknown version
- **THEN** dossier construction fails closed and no model call occurs

### Requirement: Bounded Skill context
For an existing target, the dossier SHALL include effective Skill identity, scope, type, revision, Overlay state, metadata, resource index, and only bounded relevant instruction excerpts. It MUST NOT copy complete assets, scripts, references, templates, or executable content into the dossier.

#### Scenario: Relevant instruction context exists
- **WHEN** local retrieval identifies text relevant to the assessed lesson
- **THEN** the dossier records bounded excerpts with logical location and effective revision witnesses

#### Scenario: Resource file is indexed
- **WHEN** a Skill has references or assets
- **THEN** the dossier records safe logical metadata without reading entire resources for model context

### Requirement: Dossier inspection and export
The system SHALL expose sanitized, paginated dossier inspection and deterministic JSON and Markdown exports with completeness, truncation, redaction, and version metadata.

#### Scenario: User exports a dossier
- **WHEN** the user requests an export
- **THEN** the system produces a local sanitized artifact whose hash and schema version match the inspected dossier

#### Scenario: Section exceeds display budget
- **WHEN** a section contains more safe records than the response limit
- **THEN** the system returns stable pagination and explicit completeness metadata

### Requirement: Separate generation-model consent
Model-assisted generation SHALL be disabled by default and SHALL require explicit versioned consent that discloses the sanitized data classes, configured provider, model, cost boundary, and the fact that every generated draft requires Curator review. Assessment-model consent MUST NOT imply generation consent.

#### Scenario: Assessment evaluation is enabled but generation is not
- **WHEN** a generation request is submitted without generation consent
- **THEN** the system may build the local dossier but performs no generation model call

#### Scenario: Generation consent is revoked
- **WHEN** the user revokes consent during a queued job
- **THEN** no later model stage starts and any completed local dossier remains inspectable

### Requirement: Seven-stage generation workflow
Every generation job SHALL execute the ordered stages `freeze_input`, `inspect_target`, `build_dossier`, `plan_mutation`, `synthesize_structured_draft`, `validate_and_simulate`, and `package_for_governance`. Each stage SHALL persist status, witnesses, bounded counters, and a safe outcome.

#### Scenario: Existing target job completes
- **WHEN** every stage succeeds with current witnesses
- **THEN** the job produces one validated immutable draft revision and Curator handoff package

#### Scenario: Validation fails
- **WHEN** the generated structure or rendered output fails a required check
- **THEN** the job records a non-reviewable failure or bounded retry and does not package the draft

### Requirement: Constrained generation model boundary
The generation workflow SHALL use only a compatible configured API model through the native structured-model adapter. It MUST NOT launch or delegate to a source CLI Agent and MUST NOT provide shell, file-write, external network search, arbitrary retrieval, Skill loading, Overlay mutation, or Skill creation tools.

#### Scenario: No compatible API model exists
- **WHEN** a consented job cannot resolve a compatible provider profile
- **THEN** the job remains blocked or fails with a stable configuration reason without invoking any CLI

#### Scenario: Model requests an unavailable tool
- **WHEN** model output attempts shell, network, file write, or another unregistered operation
- **THEN** the request is rejected and recorded as a safe policy violation

### Requirement: Allowlisted read-only generation tools
The model SHALL have access only to bounded dossier-section lookup, bounded effective-Skill excerpt lookup, exact-anchor search, draft-schema validation, and local preview-simulation tools. Tool inputs and outputs SHALL be sanitized, size bounded, cited, and tied to the frozen job witness.

#### Scenario: Model searches an exact anchor
- **WHEN** an exact-patch plan needs a current instruction anchor
- **THEN** the tool returns bounded matching locations from the frozen effective revision without exposing unrelated content

#### Scenario: Tool witness becomes stale
- **WHEN** the effective Skill revision changes after the job freezes input
- **THEN** subsequent tool use fails stale and the job cannot package the draft

### Requirement: Untrusted-content and prompt-injection isolation
Evidence summaries, Skill instructions, imported metadata, and model rationales SHALL be treated as untrusted data. The system SHALL delimit them from control instructions, scan injection patterns, prohibit instruction-following from data fields, and validate every tool call and model result independently.

#### Scenario: Skill text addresses the generator
- **WHEN** a bounded Skill excerpt instructs the model to ignore policy or use a tool
- **THEN** the system treats it as data, records the risk, and does not grant the requested behavior

### Requirement: Strict structured output and citations
The model SHALL return a versioned structured mutation plan with draft kind, target or new-Skill intent, rationale, lesson structure, content fields, evidence citations, expected behavior, and verification plan. Unknown fields, missing required citations, invented ids, target changes, oversized values, or invalid enums SHALL invalidate the result.

#### Scenario: Model cites an unknown signal
- **WHEN** output references an evidence id absent from the frozen dossier
- **THEN** the attempt is rejected and no rendered draft is persisted

#### Scenario: Model changes the assessed target
- **WHEN** an existing-Skill plan names another Skill, scope, or revision
- **THEN** the system rejects it and requires upstream reassessment

### Requirement: Existing-Skill draft kinds
For an assessed existing target, the generator SHALL produce only an `OverlayLearnBlock` draft or one exact-match `OverlayPatch` draft. The target and Overlay scope SHALL remain fixed by the assessment and Curator candidate.

#### Scenario: Learned guidance plan is valid
- **WHEN** evidence supports additive reusable guidance
- **THEN** the local renderer produces one bounded learned-guidance draft tied to cited evidence

#### Scenario: Multiple mutations are proposed
- **WHEN** the model attempts to combine patches, guidance blocks, or targets
- **THEN** the result is rejected rather than split or partially accepted

### Requirement: Quarantined new Skill proposal
For strong no-target evidence, the generator MAY produce one quarantined proposal containing only a valid bounded `SKILL.md` for User or Project scope. The proposal MUST remain uninstalled and unavailable to Agents until interactive Curator approval and normal Skill creation commit.

#### Scenario: No-target evidence supports a focused Skill
- **WHEN** repeated strong evidence describes a coherent reusable capability not covered by an effective Skill
- **THEN** generation may create a quarantined new-Skill proposal with collision-check witnesses

#### Scenario: Proposal requests System or Remote scope
- **WHEN** model output selects an immutable or remotely managed scope
- **THEN** the system rejects the proposal

### Requirement: New Skill content restrictions
Generated new Skill proposals SHALL contain only `SKILL.md` with valid frontmatter, stable kebab-case candidate id, explicit Role or Utility type, bounded description, declared built-in tool dependencies, and concise instructions. They MUST NOT contain scripts, tools, references, templates, assets, executable content, arbitrary configuration, credentials, or external package dependencies.

#### Scenario: Proposal includes scripts
- **WHEN** structured output requests a `scripts/` file or executable dependency
- **THEN** the proposal is rejected before quarantine persistence

#### Scenario: Candidate id collides
- **WHEN** the proposed id conflicts with an effective, shadowed, reserved, quarantined, or recently rejected Skill id
- **THEN** the system blocks packaging and requires a regenerated safe id

### Requirement: Local deterministic rendering
The system SHALL render learned guidance, exact patches, and new `SKILL.md` content locally from validated structured fields. Raw model text MUST NOT be written directly into Overlay, Skill, quarantine, or export storage.

#### Scenario: Structured output is rendered twice
- **WHEN** the same validated structure and renderer version are used
- **THEN** the rendered bytes and hash are identical

### Requirement: Comprehensive draft validation
Before packaging, the system SHALL run privacy and injection scanning, schema and frontmatter validation, content and token budgets, exact-anchor validation, duplicate detection, target compatibility, all nine draft quality checks, effective-content preview where applicable, and verification-plan completeness.

#### Scenario: Exact patch anchor is no longer unique
- **WHEN** the frozen target contains zero or multiple matches while `replace_all` is false
- **THEN** validation fails and no reviewable patch is packaged

#### Scenario: New Skill is overly broad
- **WHEN** the proposed description and instructions combine unrelated capabilities or lack a testable trigger
- **THEN** specificity or compatibility validation blocks packaging

### Requirement: Immutable generation jobs and attempts
The system SHALL persist immutable generation jobs, stage attempts, dossier revision, provider and model identifiers, prompt-template and schema versions, tool receipts, structured-result hash, renderer version, validation results, costs, timings, cancellations, and supersession links. It SHALL NOT persist raw assembled prompts or raw provider payloads.

#### Scenario: User regenerates a failed draft
- **WHEN** the user requests regeneration with unchanged input
- **THEN** the system creates a new attempt linked to the prior attempt rather than overwriting it

#### Scenario: Same request is submitted twice
- **WHEN** identical generation input and request id are submitted concurrently
- **THEN** the system returns one current job without duplicating model calls

### Requirement: Bounded generation execution
Generation SHALL enforce versioned limits for wall time, model calls, tool calls, input/output tokens, validation retries, concurrent jobs, and per-workspace daily generation cost. Exceeding a limit SHALL stop safely and preserve completed local stages.

#### Scenario: Tool-call limit is reached
- **WHEN** the model requests another tool after the job budget is exhausted
- **THEN** the attempt ends without executing the tool or packaging incomplete output

### Requirement: Cooperative cancellation and supersession
Cancellation SHALL prevent new model or tool work after the current bounded operation and SHALL never delete completed dossiers. Changes to source evidence, assessment, target, effective Skill, consent, or generator policy SHALL supersede uncommitted output.

#### Scenario: User cancels during model response
- **WHEN** cancellation is requested
- **THEN** the result is discarded or stored only as a non-reviewable safe attempt and no Curator handoff occurs

### Requirement: Curator handoff and permanent auto exclusion
Every model-generated draft and any user-edited derivative SHALL enter Curator with generation and dossier provenance and MUST remain permanently ineligible for automatic application. Handoff SHALL NOT approve, apply, install, or fabricate an interactive decision.

#### Scenario: Existing-Skill draft validates
- **WHEN** packaging completes
- **THEN** Curator receives an awaiting-review draft revision with dossier, generation, and validation witnesses

#### Scenario: Orchestration auto gate inspects generated draft
- **WHEN** a generated or edited derivative reaches automatic eligibility
- **THEN** the gate rejects it by provenance and routes it to human review

### Requirement: Interactive new Skill installation
A quarantined new Skill proposal SHALL require Curator review of the complete `SKILL.md`, evidence dossier, validation, scope, id, and creation preview. Installation SHALL use the normal conflict-safe Skill creation boundary and SHALL create no Overlay until a base Skill exists.

#### Scenario: User approves a current proposal
- **WHEN** all witnesses remain current and the user explicitly confirms creation
- **THEN** the Skill service atomically creates the User or Project Skill and records the generation and Curator provenance

#### Scenario: Name or scope becomes stale
- **WHEN** another Skill claims the id or workspace identity changes before commit
- **THEN** creation is refused and the proposal returns for regeneration or rejection

### Requirement: Dossier and generation retention
The system SHALL retain generation jobs, dossiers, drafts, exports, and quarantine content under bounded workspace policy and SHALL integrate evidence purge by deleting detailed derived content while preserving minimal non-sensitive governance tombstones for committed outcomes.

#### Scenario: Source evidence is purged
- **WHEN** the user purges the underlying evolution evidence
- **THEN** uncommitted jobs, dossiers, and drafts lose review eligibility and removable derived content is deleted

### Requirement: Generation is fail-open for Agent work
Generation model, tool, validation, database, quarantine, export, or notification failure MUST NOT affect source Agent execution, evidence collection, assessment, or ordinary Skill loading. Any uncertainty MUST fail closed for draft packaging and mutation.

#### Scenario: Provider is unavailable
- **WHEN** a background generation attempt fails to call its model
- **THEN** source Agent outcomes remain unchanged and Curator may continue with manual drafting

### Requirement: Read-only generation queries
The system SHALL expose workspace- and Skill-scoped paginated queries for consent, jobs, stages, dossiers, attempts, model provenance, tool receipts, drafts, validations, quarantine proposals, costs, supersession, and Curator handoff using sanitized projections.

#### Scenario: Inspect generation job
- **WHEN** the user opens a completed job
- **THEN** the system returns its current dossier, stage outcomes, validation, draft provenance, and governance status without raw prompts or provider payloads

