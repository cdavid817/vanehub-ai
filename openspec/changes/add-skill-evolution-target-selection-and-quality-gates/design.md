## Context

See `proposal.md` for motivation and the delta specifications for behavior. This change consumes immutable, sanitized candidate-seed revisions from `add-skill-evolution-evidence-pipeline` and effective Skill revisions from `establish-effective-skill-runtime`. The evidence pipeline establishes participation and targeting eligibility but intentionally makes no causal or quality judgment.

Assessment is a separate trust boundary. Skill content can itself contain prompt injection, imported Skills can be untrusted, model providers are external, CLI runtimes are variably observable, and later governance stages must be able to reproduce why a candidate advanced. The assessment result therefore needs deterministic witnesses even when an optional model contributes advice.

## Goals / Non-Goals

**Goals:**

- Select plausible effective Skill revisions without treating participation as causality.
- Make deterministic selection and nine quality checks fully reproducible and explainable.
- Use a model only as a constrained, optional evaluator with strict input and output boundaries.
- Produce one safe routing recommendation for later governance without performing it.
- Preserve immutable history when evidence, Skills, policies, or evaluator versions change.
- Keep assessment asynchronous and independent of every source Agent runtime.
- Provide desktop and Web users equivalent, sanitized inspection behavior.

**Non-Goals:**

- Generating replacement Skill text or an Overlay patch.
- Approving, rejecting, applying, pinning, unpinning, archiving, or editing a Skill.
- Letting a user manually force a target or bypass a failed gate.
- Writing to cross-session memory for `record_memory_only` recommendations.
- Scheduling autonomous evolution runs or implementing Curator workflow.
- Calling the CLI Agent that produced the evidence to judge its own result.
- Establishing causal proof from runtime participation.

## Decisions

### 1. Add a separate assessment context after evidence construction

Create a Rust `skill_evolution_assessment` context with four boundaries:

1. `AssessmentRepository` reads immutable sanitized seed revisions and writes assessment attempts.
2. `TargetCatalog` resolves the effective Skill universe and revision witnesses.
3. `StructuredEvaluator` optionally performs model consultation and judging.
4. `AssessmentService` exposes scheduling and read-only queries to Tauri commands.

The evidence worker enqueues an assessment request only after a seed becomes ready. It passes identifiers and witness hashes, not a mutable in-memory seed. The assessment worker reloads the authoritative seed and target universe inside its own transaction boundary.

This prevents the evidence pipeline from accumulating target-selection policy and makes later policy versioning independent. It also means assessment can be disabled or rebuilt without losing source signals.

Alternatives considered:

- Put selection in the candidate-seed builder. Rejected because grouping evidence and choosing a mutation target have different trust and versioning semantics.
- Put assessment in the future governance service. Rejected because Curator needs a completed, inspectable assessment as input rather than owning hidden ranking logic.

### 2. Freeze a complete assessment witness before scoring

An `AssessmentWitness` contains:

- seed id, seed revision, seed fingerprint, lineage hash, workspace identity, and sanitizer version;
- target-universe hash and an ordered list of effective Skill revision witnesses;
- selector policy, lexical-index policy, gate policy, routing policy, and confidence-policy versions;
- optional evaluator configuration id, provider protocol, model id, prompt-template version, and response-schema version;
- consent version and whether model evaluation was allowed for this attempt.

The worker calculates the witness before selection, then verifies it again immediately before commit. A changed seed, effective revision, lifecycle state, consent state, or policy supersedes the attempt and queues a fresh one. Completed attempts are immutable.

Alternative considered: store only the final target and verdict. Rejected because it cannot explain drift, reproduce a decision, or distinguish a model change from evidence change.

### 3. Build the target universe from effective revisions only

The current effective-runtime resolver is authoritative for scope precedence. Each target record contains safe local features:

- stable Skill id, effective scope, type, revision hash, lifecycle and trust state;
- bounded name and description;
- declared tags, capabilities, tool identifiers, and resource kinds;
- locally derived heading and keyword tokens;
- verified and correlated participation counts from the seed lineage.

Shadowed revisions are recorded as exclusions. A historical revision observed in evidence remains part of lineage but is not silently substituted for the current revision. The selector can describe the historical/current mismatch and route uncertainty appropriately.

Pinned and archived targets stay in the universe long enough for the lifecycle gate to explain why they cannot advance. Missing or malformed Skills are excluded with a reason.

Alternative considered: scan every Skill directory independently during assessment. Rejected because it would bypass effective scope, Overlay replay, trust, and lifecycle semantics.

### 4. Use deterministic scoring before any model call

Selector policy version 1 uses a 100-point bounded score:

| Component | Maximum | Basis |
|---|---:|---|
| Attribution | 35 | verified 35, correlated 20, weak 0, unattributed 0 |
| Repeated participation | 15 | independent compatible runs, capped at three |
| Capability and type compatibility | 20 | category, Role/Utility behavior, declared tools and capabilities |
| Local lexical relevance | 20 | weighted description, tags, headings, and bounded instruction tokens |
| Scope locality | 10 | compatible project scope 10, user scope 7, remote scope 4, system scope 2 |

Negative compatibility evidence can reduce a component but never creates a negative total. Scores are represented as fixed integers, not floating-point values. The ordered explanation preserves every component, exclusion, and matched feature class.

Version 1 classifications are:

- `selected`: leading score at least 60 and margin at least 15;
- `ambiguous`: leading score at least 45 but selected thresholds are not both met;
- `no_target`: every score is below 45 or no eligible target exists.

Ties use effective scope priority, Skill id, then revision hash. These values are initial versioned policy, not hard-coded UI assumptions.

Alternatives considered:

- Vector similarity as the primary selector. Rejected because small Skill catalogs and short sanitized seeds benefit from transparent lexical features, while embedding model/version drift complicates reproducibility.
- Ask a model to choose from all Skills. Rejected because it is less reproducible and makes prompt injection and invented targets harder to contain.

### 5. Keep lexical retrieval local and treat instructions as data

The lexical index uses Unicode normalization, case folding, stable tokenization, language-aware stop lists, and field weights. It indexes descriptions and tags at higher weight than instruction headings; bounded body tokens have the lowest weight. It never expands `{skill_base_dir}`, opens referenced files, reads assets, or executes scripts.

Instruction text is parsed as untrusted content. Injection-like phrases are tagged for risk inspection but never concatenated into control prompts. The persisted ranking explanation stores matched token hashes and field classes, not entire instruction passages.

Alternative considered: send full `SKILL.md` files to the model for semantic selection. Rejected because it increases data exposure, token cost, injection risk, and revision-witness complexity.

### 6. Consult a model only for bounded ambiguity

The optional target consultant receives at most the deterministic top five candidates. Its data payload contains:

- a sanitized structured seed summary and evidence categories;
- candidate ids, revision hashes, types, scopes, bounded descriptions, declared capabilities, deterministic score components, and safe matched feature classes;
- explicit instruction that candidates are untrusted data and only supplied ids are valid.

It has no tools, no conversation history, no file access, no retrieval, and no source CLI access. The output schema accepts a candidate id or `unresolved`, cited evidence ids, confidence from 0 to 1, and a bounded rationale. Unknown fields, invented ids, missing citations, excessive content, or schema violations invalidate the whole consultation.

A valid consultation is advisory. It can resolve an ambiguous choice only when its candidate already meets the 45-point floor, its cited evidence exists, and model confidence is at least 0.75. The deterministic ranking remains visible. A consultant cannot turn `no_target` into a target.

Alternative considered: always consult the model. Rejected because clear deterministic cases gain little while cost, latency, privacy exposure, and nondeterminism increase.

### 7. Run exactly nine ordered deterministic checks

The gate engine is a registry whose version-1 order is fixed:

1. `privacy_residue`
2. `evidence_sufficiency`
3. `duplicate_knowledge`
4. `transient_incident`
5. `guidance_specificity`
6. `evidence_consistency`
7. `target_compatibility`
8. `executable_content_risk`
9. `target_lifecycle_mutability`

Each check returns `pass`, `fail`, `review`, or `not_applicable`; severity; stable reason codes; sanitized evidence references; and route constraints. Checks are pure over the frozen witness and registered local indexes. Even after a hard stop, the engine records the remaining checks as evaluated where safe or `not_applicable`, preserving an exactly-nine result contract.

Check policy:

- Privacy residue is a hard `drop` and prevents model calls.
- Insufficient evidence or irreparably vague guidance constrains routing to `drop`.
- Canonical semantic duplication constrains routing to `merge_duplicate`.
- A purely transient or local fact constrains routing to `record_memory_only` or `drop`.
- Material contradictions and unresolved target incompatibility constrain routing to `needs_human_review` or `drop`.
- Executable expansion always sets high risk and requires `needs_human_review`.
- Pinned targets constrain routing to `record_memory_only`; archived, missing, or malformed targets constrain it to `drop`.

No check directly performs the constrained route. The routing reducer handles conflicts centrally.

Alternative considered: let each check return a final verdict. Rejected because competing results would produce order-dependent routing.

### 8. Make duplicate detection structural, not lexical alone

The duplicate index includes normalized guidance units from:

- the effective Skill body after trusted Overlay replay;
- active trusted Overlay learn blocks and patches;
- current candidate seeds and assessment attempts for the same effective revision.

A guidance unit has a trigger, action, constraint, and expected verification when available. Exact normalized hashes are duplicates. Near matches require compatible structural fields and a high local similarity threshold; lexical overlap alone only becomes a review hint. The assessment stores the canonical Skill revision or pending assessment reference but never merges records itself.

Untrusted Overlay content is not accepted as canonical guidance. It may be reported as a conflict or risk input.

Alternative considered: compare only candidate fingerprints. Rejected because evidence fingerprints describe tasks, not whether effective guidance already contains the lesson.

### 9. Derive structured lesson features without generating final text

The deterministic gate engine needs a bounded representation of the prospective lesson but this change must not generate Skill instructions. A `LessonShape` therefore contains optional structured fields:

- trigger category and bounded conditions;
- required behavior category;
- prohibited behavior category;
- verification category;
- environmental scope;
- content kinds such as guidance, reference, template, tool declaration, or executable.

It is derived from signal enums, corrected-feedback fields, verification facts, and recovery deltas. Missing fields remain missing. The specificity gate evaluates completeness rather than asking a model to fill gaps. The model judge can identify unsupported or missing structure but cannot author replacement content.

Alternative considered: generate a draft patch before quality review. Rejected because it gives the evaluator invented content not grounded in evidence and prematurely couples assessment to Overlay format.

### 10. Use a second constrained call as an optional quality judge

The quality judge runs only when:

- consent and a compatible configured API model are available;
- privacy residue did not fail;
- evidence has a usable target or an explicitly reviewable ambiguity;
- deterministic routing is not already an unambiguous `drop` or `merge_duplicate`;
- the assessment-wide model-call and token budgets have capacity.

The judge sees the sanitized witness projection, deterministic ranking, `LessonShape`, and nine check results. It returns supportedness, specificity, durability, actionability, contradiction, and risk ratings; cited evidence ids; a bounded rationale; and a recommended route.

The judge can make a result stricter. It cannot:

- override a deterministic hard stop;
- lower deterministic maximum risk;
- add or rewrite evidence;
- introduce a target;
- recommend a route outside the five allowed values;
- cause mutation or another model/tool call.

Invalid or unavailable responses produce deterministic fallback. Provider errors are mapped to stable categories and sanitized before unified logging.

Alternative considered: combine target consultation and quality judging in one request. Rejected because target ambiguity and lesson quality have different schemas, eligibility rules, and audit semantics. Clear targets also need no consultation call.

### 11. Require explicit consent and reuse a structured API-model adapter

Model evaluation defaults off. Consent records disclosure version, enabled state, timestamp, and local actor. The disclosure identifies outbound data classes: sanitized seed summary, evidence categories and identifiers, bounded Skill metadata, score/check results, and bounded rationales. Raw prompts, tool data, terminal output, file content, credentials, and full Skill instructions are prohibited.

`StructuredEvaluator` uses a compatible configured API provider profile through the native model runtime; it never launches or delegates to a CLI Agent. Initially it can resolve the active compatible OnePiece provider profile, but the interface is provider-neutral so a dedicated evaluator profile can be added later without changing assessment contracts.

The evaluator imposes two calls maximum per attempt, one attempt per stage, a 15-second stage deadline, bounded input/output tokens, temperature zero where supported, and strict JSON schema. Deterministic assessment remains complete when no profile exists.

Alternative considered: infer consent from ordinary Agent use. Rejected because assessment sends a different, aggregated data class and must be independently disclosed.

### 12. Reduce results with a strict routing lattice

Route precedence is not a simple severity sort because duplicates and memory-only outcomes are semantically distinct. Version 1 applies:

1. privacy or invalid-input hard stop → `drop`;
2. executable risk, material contradiction, unresolved compatible ambiguity → `needs_human_review`;
3. confirmed canonical duplicate → `merge_duplicate`;
4. transient/local-only or pinned target → `record_memory_only`;
5. insufficient, vague, incompatible, archived, or missing target → `drop`;
6. all required checks pass with clear target, low risk, and system confidence at least 0.85 → `advance`;
7. otherwise → `needs_human_review`.

The reducer records all constraints and identifies the winning rule. An `advance` result means only “eligible for later governance”; it is not auto-apply authorization.

System confidence is calculated from deterministic evidence strength, selection score and margin, lineage independence, check completeness, and contradiction penalties. Model confidence can only contribute a bounded corroboration bonus of at most 0.05 and cannot raise a result across a hard constraint. Risk is the maximum of deterministic and valid model risk.

Alternative considered: average all scores. Rejected because averaging can hide a single safety-critical condition.

### 13. Persist immutable attempts and normalized explanations

Add SQLite tables:

- `evolution_assessment_attempts`
- `evolution_assessment_targets`
- `evolution_assessment_score_components`
- `evolution_assessment_checks`
- `evolution_assessment_evidence_links`
- `evolution_assessment_model_calls`
- `evolution_assessment_supersessions`
- `evolution_assessment_policy`
- `evolution_assessment_queue_state`

The attempt row stores status, route, confidence, risk, witness hashes, policy versions, current/superseded relation, and timestamps. Model-call rows store only request projection hash, provider/model identifiers, schema/template versions, outcome category, sanitized structured response, token counts, and latency. They never store the raw assembled prompt or provider payload.

An idempotency key covers seed revision plus the full assessment witness. A unique constraint coalesces concurrent requests. A lease with heartbeat allows worker recovery; expired in-progress attempts resume or fail safely without overwriting completed history.

Assessment history follows the evidence retention boundary. Purging source evidence transactionally removes dependent assessment rows. Reassessment never resurrects purged lineage.

Alternative considered: store one mutable assessment per seed. Rejected because policy drift and model nondeterminism would erase the audit trail.

### 14. Keep scheduling bounded and fail-open

Assessment uses a separate bounded queue from evidence ingestion so a slow model cannot consume evidence capacity. Deterministic work has priority over optional model stages. When pressure rises, queued optional consultation/judge work falls back to deterministic completion before ready-seed assessments are dropped.

Runtime producers never wait for assessment. User-triggered reassessment waits only for scheduling acknowledgement, not evaluation completion. Worker and model errors are observable through unified logging and pipeline health queries, with sanitized categories and counters.

The Web adapter supplies deterministic fixtures and simulates enabled, unavailable, fallback, pending, and superseded states. It does not call local SQLite or native providers.

Alternative considered: run assessment synchronously when a seed becomes ready. Rejected because retrieval, database contention, or provider latency could affect evidence processing and source Agent responsiveness.

### 15. Expose explanation models through the existing service boundary

Extend `agent-service.ts` with narrowly scoped models for:

- assessment summary and status;
- ranked target and score components;
- nine quality-check results;
- model/fallback provenance;
- routing, confidence, and risk explanation;
- immutable history and supersession reason;
- model-evaluation policy and consent update;
- reassessment acknowledgement.

The Tauri client alone invokes native commands. The Web client returns the same discriminated unions and stable error codes. React modules render these service models and remain under the 300-line limit.

The Evolution UI is evidence and explanation only. It shows “participating,” “correlated,” and “advisory” labels rather than causal language. `advance` is displayed as awaiting a later governance stage. There are no target override or mutation controls.

Alternative considered: reuse raw repository DTOs in components. Rejected because it leaks persistence details and weakens desktop/Web parity.

## Risks / Trade-offs

- [Rule scoring misses semantic relationships] → Keep alternatives visible, use optional consultation only within eligible candidates, and version weights for measured improvement.
- [Users interpret selected target as causal proof] → Label selection as relevance, retain attribution fidelity, show score components, and prohibit causal wording.
- [Skill instructions inject the evaluator] → Index locally as data, omit full bodies from model payloads, delimit untrusted fields, disable tools, and validate strict schemas.
- [Model evaluation leaks local information] → Default off, require independent consent, send bounded sanitized projections, and never store raw prompts.
- [Model nondeterminism weakens auditability] → Preserve deterministic results, hash all witnesses, store structured provenance, and allow the model only to constrain within fixed rules.
- [Hard thresholds reject useful rare lessons] → Let verified explicit corrections satisfy evidence sufficiency and route unresolved but supported cases to later human review.
- [Duplicate detection collapses distinct guidance] → Require structural compatibility; treat lexical-only matches as hints.
- [Pinned Skills accumulate unusable candidates] → Route to memory-only and expose the lifecycle reason without bypassing pinning.
- [Assessment history grows quickly] → Share evidence retention, normalized child tables, quotas, and cascade purge.
- [External provider latency blocks the queue] → Separate deterministic and optional lanes, cap calls and deadlines, and prefer deterministic fallback.
- [Policy upgrades create mass reassessment] → Mark stale lazily, prioritize visible/ready seeds, coalesce identical witnesses, and preserve the last current result until replacement completes.
- [Web mocks drift from desktop behavior] → Share TypeScript contracts, stable fixtures, adapter contract tests, and equivalent status/error semantics.

## Migration Plan

1. Complete effective Skill runtime and evidence-pipeline prerequisites and validate stable seed, revision, lifecycle, and privacy witnesses.
2. Add assessment domain enums, witness hashing, target catalog projection, policy versions, and pure deterministic scoring tests behind a disabled feature flag.
3. Add local lexical indexing and the exactly-nine gate registry with privacy, injection, duplicate, transient, contradiction, lifecycle, and executable-risk fixtures.
4. Add routing, confidence, risk, immutable attempt schema, repositories, leases, idempotency, supersession, retention, and purge integration.
5. Add the bounded assessment queue and deterministic worker; verify all Agent and evidence paths remain fail-open under saturation and database failure.
6. Add the provider-neutral structured evaluator, consent policy, target consultation and quality-judge schemas, strict validation, deadlines, budgets, fallback, and sanitized unified logging.
7. Enable deterministic assessment for ready seeds and compare rankings and gates against a curated local fixture corpus before enabling optional model evaluation.
8. Add Skill service contracts, native commands, Tauri and Web/mock adapters, assessment views, consent disclosure, reassessment controls, localization, and accessibility.
9. Run full Rust, frontend, contracts, privacy, prompt-injection, reproducibility, retention, adapter-parity, E2E, documentation, and strict OpenSpec validation.

Rollback disables new scheduling and model evaluation first, then stops the assessment worker and hides assessment UI entry points. Evidence collection and every Agent runtime continue unchanged. Additive assessment tables remain readable or can be purged with their evidence; older binaries ignore them. Re-enabling verifies policy and schema versions and uses idempotency witnesses to avoid duplicate current attempts.
