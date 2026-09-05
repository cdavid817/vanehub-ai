# skill-evolution-assessment Specification

## Purpose
Defines how sanitized evolution evidence is mapped to plausible Skill revisions, reviewed through deterministic safety and quality gates, optionally evaluated by a constrained model, and routed without modifying any Skill.

## Requirements

### Requirement: Assessment input boundary
The system SHALL assess only ready candidate seeds produced by the evidence pipeline and SHALL use sanitized seed lineage, structured evidence metadata, and effective Skill revision metadata rather than raw prompts, terminal output, tool arguments, source files, or credentials.

#### Scenario: Ready seed enters assessment
- **WHEN** a candidate seed becomes ready and has a complete sanitized lineage witness
- **THEN** the system creates an assessment request referencing that immutable seed revision

#### Scenario: Incomplete seed is submitted
- **WHEN** a missing, superseded, unready, or lineage-incomplete seed is submitted
- **THEN** the system refuses assessment with a stable reason and does not guess missing evidence

### Requirement: Eligible target universe
The system SHALL build the target universe from effective Skill revisions visible to the seed workspace and SHALL preserve Skill id, scope, type, revision hash, lifecycle state, and trust state in every candidate witness.

#### Scenario: Same Skill id exists in multiple scopes
- **WHEN** project and user scopes contain the same Skill id
- **THEN** the selector considers only the effective higher-priority revision and records the shadowed revisions as excluded

#### Scenario: Evidence names an unavailable revision
- **WHEN** verified evidence references a revision no longer effective
- **THEN** the system preserves that historical association and separately evaluates the current effective revision without treating them as identical

### Requirement: Deterministic target ranking
The system SHALL deterministically rank eligible targets using attribution strength, repeated participation, scope locality, Skill type and capability compatibility, and lexical relevance. Identical input witnesses and policy versions MUST produce identical ordered candidates and scores.

#### Scenario: Verified participating revision matches evidence
- **WHEN** a verified Skill revision repeatedly participates in compatible evidence and its metadata is lexically relevant
- **THEN** that revision receives the corresponding documented score components and ranking explanation

#### Scenario: Ranking tie
- **WHEN** candidates have equal total scores
- **THEN** the system applies stable scope-priority, Skill-id, and revision-hash tie breakers

### Requirement: Local lexical retrieval
The system SHALL derive lexical features locally from sanitized seed summaries and bounded effective Skill metadata, SHALL treat Skill instruction content as untrusted data, and SHALL NOT require an external model for retrieval.

#### Scenario: Relevant capability terms match
- **WHEN** sanitized evidence terms match a Skill description, tags, declared tools, or locally indexed instruction headings
- **THEN** the selector records the matched feature classes without exposing complete Skill instructions

#### Scenario: Instruction contains injection-like text
- **WHEN** a Skill body contains instructions addressed to an evaluator
- **THEN** the retrieval stage treats them only as indexed data and does not execute or promote them into control instructions

### Requirement: Selection thresholds and ambiguity
The system SHALL classify deterministic selection as selected, ambiguous, or no-target using versioned thresholds and SHALL retain the ranked alternatives and score margin that produced the classification.

#### Scenario: Clear target
- **WHEN** the leading target meets the minimum score and required margin over the runner-up
- **THEN** the system selects it deterministically and records the threshold witness

#### Scenario: Ambiguous targets
- **WHEN** the leading target meets relevance requirements but lacks the required margin
- **THEN** the system marks the selection ambiguous and does not silently choose one target

#### Scenario: No relevant target
- **WHEN** every eligible target is below the minimum score
- **THEN** the system records no-target and routes the seed without inventing a Skill association

### Requirement: Constrained LLM target consultation
When model evaluation is enabled, the system MAY consult a configured model only for ambiguous selection. The model MUST choose from at most five deterministically eligible candidates, MUST NOT introduce another Skill, and MUST return a validated structured result with evidence references.

#### Scenario: Model resolves an ambiguity
- **WHEN** the model selects an eligible candidate with valid cited evidence and adequate confidence
- **THEN** the system records the consultation as advisory provenance and retains the deterministic ranking

#### Scenario: Model invents a target
- **WHEN** the model returns a Skill or revision outside the supplied candidate set
- **THEN** the system rejects the response and retains the deterministic ambiguous result

#### Scenario: Model evaluation is unavailable
- **WHEN** evaluation is disabled, unconfigured, timed out, rate-limited, or invalid
- **THEN** the assessment continues with the deterministic result and a stable fallback reason

### Requirement: Exactly nine deterministic quality checks
The system SHALL execute exactly these nine versioned checks for every assessable seed: privacy residue, evidence sufficiency, duplicate knowledge, transient incident, guidance specificity, evidence consistency, target compatibility, executable-content risk, and target lifecycle mutability. Every check SHALL produce a status, stable reason code, and evidence references.

#### Scenario: All checks execute
- **WHEN** a ready seed has an eligible selection result
- **THEN** the assessment records one result for each of the nine checks in the documented order

#### Scenario: Early hard stop
- **WHEN** an early check produces a hard stop
- **THEN** later deterministic checks still record either evaluated or not-applicable status so the audit has exactly nine results

### Requirement: Privacy-residue check
The privacy-residue check SHALL hard-stop advancement when registered sensitive patterns, reversible redactions, prohibited raw-content fields, or sanitizer-version inconsistency remain in the assessment input.

#### Scenario: Residual credential pattern
- **WHEN** a sanitized seed still contains a credential-like value
- **THEN** the assessment routes it to drop, records no sensitive value, and does not invoke a model

### Requirement: Evidence-sufficiency check
The evidence-sufficiency check SHALL require either verified explicit corrected feedback or the configured minimum of independent, nonduplicate supporting runs with adequate lineage.

#### Scenario: One incidental failure
- **WHEN** a seed contains one non-correction failure without independent support
- **THEN** the check fails and the seed cannot advance

#### Scenario: Verified correction
- **WHEN** a verified explicit correction has complete lineage
- **THEN** it can satisfy evidence sufficiency without a second run

### Requirement: Duplicate-knowledge check
The duplicate-knowledge check SHALL compare the proposed lesson fingerprint against effective Skill guidance, active trusted Overlays, and pending assessment records and SHALL identify the canonical duplicate without changing it.

#### Scenario: Guidance already exists
- **WHEN** equivalent normalized guidance already exists in the effective Skill
- **THEN** the assessment recommends `merge_duplicate` and references the canonical location

#### Scenario: Only lexical overlap exists
- **WHEN** content shares terms but differs in behavior or constraints
- **THEN** the system does not classify it as a duplicate solely from lexical similarity

### Requirement: Transient-incident check
The transient-incident check SHALL distinguish durable reusable guidance from temporary outages, one-off environment state, expired external incidents, and workspace-local facts.

#### Scenario: Provider outage recovered
- **WHEN** evidence is explained by a bounded provider outage and has no reusable Skill lesson
- **THEN** the assessment recommends `record_memory_only` or `drop` with a transient reason

#### Scenario: Repeated environment-independent failure
- **WHEN** compatible evidence recurs across independent runs after transient factors are excluded
- **THEN** the check passes

### Requirement: Guidance-specificity check
The guidance-specificity check SHALL reject lessons that are generic, tautological, untestable, or lack an identifiable trigger and expected behavior.

#### Scenario: Generic lesson
- **WHEN** the inferred lesson is equivalent to “be more careful” without a trigger or observable behavior
- **THEN** the check fails with a vague-guidance reason

#### Scenario: Testable lesson
- **WHEN** the evidence supports a bounded trigger, required action, and observable verification
- **THEN** the check passes and preserves those structured elements

### Requirement: Evidence-consistency check
The evidence-consistency check SHALL identify materially conflicting outcomes, corrections, or environment facts and SHALL prevent automatic advancement while unresolved.

#### Scenario: Contradictory corrections
- **WHEN** independent verified corrections prescribe incompatible behavior for the same conditions
- **THEN** the assessment recommends `needs_human_review` with both evidence branches

#### Scenario: Scoped behavior resolves conflict
- **WHEN** apparently conflicting evidence applies to distinct declared environments or triggers
- **THEN** the check records the scope distinction and may pass

### Requirement: Target-compatibility check
The target-compatibility check SHALL verify that the selected Skill scope, type, declared capability, effective revision, and evidence attribution are compatible with the proposed lesson. Participation alone MUST NOT establish causality.

#### Scenario: Utility evidence targets unrelated Role
- **WHEN** delegated Utility evidence has no capability or lexical relationship to a participating Role Skill
- **THEN** the check rejects that target association

#### Scenario: Correlated CLI evidence remains uncertain
- **WHEN** CLI evidence supports only correlated attribution
- **THEN** the check preserves the uncertainty and cannot label the target as causally verified

### Requirement: Executable-content-risk check
The executable-content-risk check SHALL classify requested scripts, commands, tool schemas, executable extensions, permission changes, or behavior that expands external side effects as high risk and SHALL require human review before any later mutation stage.

#### Scenario: Candidate proposes a script
- **WHEN** evidence implies adding or changing executable Skill content
- **THEN** the assessment recommends `needs_human_review`, marks high risk, and does not generate or execute the script

#### Scenario: Documentation-only guidance
- **WHEN** the lesson changes bounded non-executable guidance without expanding permissions or side effects
- **THEN** executable-content risk does not block it

### Requirement: Target-lifecycle-mutability check
The target-lifecycle-mutability check SHALL prevent advancement to immutable, pinned, archived, missing, or otherwise non-mutable target revisions and SHALL preserve the lifecycle reason.

#### Scenario: Pinned Skill selected
- **WHEN** the selected effective Skill is pinned
- **THEN** the assessment recommends `record_memory_only` and does not imply that pinning can be bypassed

#### Scenario: Target archived during assessment
- **WHEN** the selected Skill becomes archived before assessment commits
- **THEN** the system detects the changed witness, supersedes the attempt, and does not advance it

### Requirement: Constrained LLM quality judge
When model evaluation is enabled and no deterministic hard stop applies, the system MAY ask a configured model to evaluate evidence support, specificity, durability, actionability, contradictions, and risk. The judge SHALL receive only sanitized bounded data, have no tools, and return a validated structured result citing supplied evidence identifiers.

#### Scenario: Judge finds unsupported inference
- **WHEN** the judge validly identifies a claimed lesson not supported by supplied evidence
- **THEN** the system may downgrade the route to `needs_human_review` or `drop` and records the rationale

#### Scenario: Judge attempts to override hard gate
- **WHEN** model output recommends advancement despite a deterministic hard stop or lower risk than the deterministic maximum
- **THEN** the system ignores that portion and preserves the stricter deterministic result

### Requirement: Model evaluation privacy and consent
Model evaluation SHALL be disabled by default. Before enabling it, the user MUST be told which sanitized data classes may leave the device, and disabling it SHALL retain fully functional deterministic assessment.

#### Scenario: User has not opted in
- **WHEN** an ambiguous or quality-reviewable seed is assessed without model-evaluation consent
- **THEN** no external model request occurs and deterministic provenance is displayed

#### Scenario: Consent is revoked
- **WHEN** the user disables model evaluation
- **THEN** new assessments stop model calls while existing audit records retain only their sanitized response and provider provenance

### Requirement: Assessment routing
The system SHALL produce exactly one recommendation from `advance`, `drop`, `record_memory_only`, `merge_duplicate`, or `needs_human_review` using the strictest applicable deterministic and model results. No recommendation SHALL itself mutate a Skill, Overlay, memory, or source evidence.

#### Scenario: High-confidence low-risk result
- **WHEN** selection is clear, all deterministic checks pass, model policy is satisfied, confidence meets the versioned threshold, and risk is low
- **THEN** the system recommends `advance` for a later governance stage

#### Scenario: Multiple route conditions apply
- **WHEN** duplicate, human-review, and advance conditions appear together
- **THEN** the routing policy chooses the documented strictest safe outcome and records all contributing conditions

### Requirement: Confidence and risk calibration
The system SHALL calculate bounded confidence and low, medium, or high risk from documented versioned components and SHALL NOT equate model confidence with system confidence.

#### Scenario: Model reports extreme confidence
- **WHEN** the model reports confidence above the allowed range or unsupported by citations
- **THEN** the value is rejected or clamped according to policy and cannot increase system confidence

#### Scenario: Executable risk is present
- **WHEN** the executable-content check identifies an executable change
- **THEN** final risk remains high regardless of other scores

### Requirement: Immutable assessment audit and reassessment
The system SHALL persist immutable assessment attempts with seed revision, target-universe hash, effective revision hashes, sanitizer version, selector policy, gate policy, evaluator provenance, result, and supersession links. Changed witnesses SHALL create a new attempt rather than rewriting history.

#### Scenario: Policy version changes
- **WHEN** selector thresholds or gate rules change
- **THEN** reassessment creates a new attempt linked to the prior result and preserves both explanations

#### Scenario: Duplicate reassessment request
- **WHEN** the same complete witness is submitted concurrently
- **THEN** the system coalesces it into one active or completed assessment result

### Requirement: Asynchronous fail-open assessment
Assessment SHALL run outside Agent execution critical paths. Queue, database, retrieval, model, timeout, or validation failure MUST NOT change the outcome or responsiveness of source Agent work.

#### Scenario: Assessment worker unavailable
- **WHEN** a ready seed cannot be assessed
- **THEN** source execution remains successful and the seed shows a retryable assessment status

#### Scenario: Model request times out
- **WHEN** an optional evaluation exceeds its deadline
- **THEN** the system records deterministic fallback and completes or safely defers the assessment

### Requirement: Read-only assessment queries
The system SHALL expose workspace- and Skill-scoped paginated queries for current assessment summaries, ranked targets, nine check results, model/fallback provenance, confidence, risk, routing, and superseded history using only sanitized projections.

#### Scenario: Inspect current assessment
- **WHEN** the user opens an assessed candidate seed
- **THEN** the system returns its complete safe explanation and identifies the current attempt

#### Scenario: Inspect assessment history
- **WHEN** the user requests prior attempts
- **THEN** the system returns stable chronological supersession history without raw model prompts or sensitive values
