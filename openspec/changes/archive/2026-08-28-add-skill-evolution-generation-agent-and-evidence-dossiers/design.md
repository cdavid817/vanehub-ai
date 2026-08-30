## Context

See `proposal.md` for motivation and the delta specifications for behavior. Evidence, assessment, Curator, Overlay, and orchestration already define authoritative records and safe mutation paths. Generation must consume snapshots from those services rather than reading logs or runtime transcripts. It also introduces a stronger threat model: both the input Skill and evidence can contain prompt injection, and model output is untrusted content that may attempt to escape the allowed draft schema.

The generation stage is optional. Curator manual drafting must continue to work without a configured model, consent, or successful generation job. A generated artifact is a proposal, never authority.

## Goals / Non-Goals

**Goals:**

- Give reviewers a complete, reproducible, privacy-safe evidence package.
- Produce useful bounded drafts without granting a model filesystem or mutation access.
- Make every model claim traceable to frozen dossier evidence.
- Render final artifacts deterministically outside the model.
- Validate generated drafts against the same quality and Overlay invariants as manual drafts.
- Support carefully quarantined proposals for genuinely uncovered capabilities.
- Preserve immutable attempts, budgets, costs, validation, and governance lineage.
- Integrate as optional bounded work inside orchestration without blocking the pipeline.

**Non-Goals:**

- Exposing chain-of-thought or raw provider prompts.
- Letting the model browse the repository, internet, terminal, or arbitrary Skill resources.
- Generating scripts, tool implementations, assets, references, templates, or configuration.
- Automatically applying any model-generated artifact.
- Automatically installing a new Skill proposal.
- Generating or changing multiple Skills in one job.
- Running generation through a source CLI Agent or a visible system-owned conversation.
- Replacing the normal Skill creator and editor for general user-authored content.

## Decisions

### 1. Add a separate generation application context

Create a Rust `skill_evolution_generation` context with ports for evidence/assessment snapshots, effective Skill excerpts, dossier persistence, structured model calls, constrained tools, renderers, validators, Curator handoff, quarantined Skill proposals, policy/consent, clock, notification, and unified diagnostics.

The context does not depend on concrete Overlay files, Skill directories, provider clients, or React. It receives typed snapshots and calls existing application services for preview and creation. This prevents generation from becoming an alternate filesystem API.

Alternative considered: implement generation as a general Utility Skill. Rejected because governance, provider consent, persistent jobs, privacy, and mutation boundaries require native application enforcement independent of model instructions.

### 2. Freeze all source witnesses before dossier construction

`GenerationInputWitness` contains:

- workspace and candidate-seed revision;
- assessment attempt, route, target or no-target result, and quality policy;
- evidence lineage and sanitizer versions;
- effective Skill, Overlay, catalog, pin, trust, and scope witnesses when targeted;
- Curator candidate/draft state if one exists;
- generation policy, consent, dossier schema, renderer, validator, and model configuration versions.

The job reloads and hashes these at `freeze_input`. Before each model call and before packaging it confirms that privacy, assessment, target, consent, and policy witnesses remain current. Target or evidence changes supersede the job. Overlay-only drift may allow a fresh validation stage but can never silently change the frozen context given to the model.

Alternative considered: let model tools read live state throughout the job. Rejected because cited evidence and resulting patches would not refer to one coherent revision.

### 3. Define the dossier as a deterministic typed document

`EvidenceDossierV1` has a header and exactly thirteen typed sections in the specification order. Each section contains status (`complete`, `partial`, `not_applicable`, `unavailable`, `redacted`), source witnesses, safe records, truncation metadata, and section hash. The complete hash covers schema version, ordered section hashes, builder version, and sanitizer version.

The builder uses local stable templates, not a model, for the executive summary and timeline. Summaries are enum/count based with bounded authorized feedback where applicable. Records sort by stable category, occurred time, and id. Unknown source versions stop the relevant section and mark the dossier non-generatable.

Version 1 limits are 128 KiB canonical JSON, 100 signal records, 32 target/quality records, 8 KiB combined instruction excerpts, and 1,000 timeline entries summarized into bounded buckets. Truncation always retains counts and selection policy.

Alternative considered: ask the model to summarize raw evidence into a dossier. Rejected because the dossier is the audit input and must exist before, and independently of, model behavior.

### 4. Store normalized dossier sections and deterministic exports

SQLite stores dossier header, section rows, safe record projections, source links, and hashes. Large canonical JSON and Markdown exports are produced on demand into the normal user-selected export boundary; they are not hidden feature-local files. Export contains the same redacted data and an explicit manifest of completeness and versions.

The Markdown renderer is deterministic and intended for human review; JSON is canonical and intended for audit tooling. Neither export includes provider payloads, model prompts, full Skill bodies, or unsafe rejected output.

Alternative considered: store one opaque Markdown blob. Rejected because queries, pagination, purge, version comparison, and model tool lookup need typed sections.

### 5. Require generation-specific consent

`GenerationPolicy` is separate from assessment evaluation and orchestration auto-apply. It stores enabled state, disclosure version, compatible API profile reference, model id, per-job and daily budgets, allowed draft kinds, and revision. It defaults disabled.

The disclosure enumerates outbound data: sanitized dossier records, bounded effective Skill excerpts, target metadata, assessment results, and local tool responses. It also states that drafts are model-generated, permanently manual, and may incur provider usage. Imported settings never carry consent.

Revocation prevents new model/tool stages. Locally completed dossiers and already packaged drafts remain visible with historical consent provenance but do not gain approval.

Alternative considered: reuse assessment-model consent. Rejected because generation sends more Skill context and creates content intended for mutation.

### 6. Use a durable seven-stage job state machine

Stages are fixed:

1. `freeze_input`: validate eligibility and capture witnesses.
2. `inspect_target`: build bounded target or no-target context using local retrieval.
3. `build_dossier`: create and persist the thirteen sections.
4. `plan_mutation`: request a cited structured plan or deterministic no-draft result.
5. `synthesize_structured_draft`: request bounded schema fields, not final files.
6. `validate_and_simulate`: render locally, scan, validate, run quality gates, and preview.
7. `package_for_governance`: store immutable review package and idempotently hand it to Curator.

Job states are `requested`, `blocked_consent`, `queued`, `running`, `cancel_requested`, `cancelled`, `failed`, `completed`, and `superseded`. Each stage attempt is immutable and has input/output hashes, budget use, safe failure, and timestamps. A bounded repair creates a new attempt rather than rewriting model output.

Alternative considered: one opaque model call. Rejected because planning, synthesis, validation repair, and local rendering need separate observable contracts.

### 7. Reuse the provider-neutral structured API adapter

Generation resolves a compatible configured API profile through `StructuredEvaluator`, with a distinct purpose, consent, templates, and schemas. It never launches a CLI or sends a normal chat message. The provider receives no conversation history.

Default job budgets are:

- 180 seconds wall time;
- 3 model calls including at most one repair;
- 8 read-only tool calls;
- 48,000 total input tokens;
- 8,000 total output tokens;
- 1 concurrent job per workspace and 2 globally;
- 250,000 input and 50,000 output tokens per workspace per rolling day.

Temperature is zero where supported. A provider that cannot guarantee structured JSON uses strict parsing and schema rejection; freeform fallback is not accepted. Provider usage is stored as counts and identifiers, not raw request/response envelopes.

Alternative considered: delegate to whichever Agent produced the evidence. Rejected because CLI fidelity, prompt policy, tools, and provider availability vary and would make governance nonuniform.

### 8. Expose five narrow read-only tools

The model tool registry for generation has exactly five version-1 operations:

1. `read_dossier_section(section_id, cursor)`
2. `read_skill_excerpt(excerpt_id)`
3. `find_exact_anchor(query_hash_or_bounded_text)`
4. `validate_draft_structure(structure)`
5. `simulate_local_preview(structure_hash)`

Every call is bound to job and witness, validates enumerated arguments, limits response size, returns citations, and is audited. `simulate_local_preview` returns safe validation/diff metadata, not mutation authority. There is no generic file read, grep, glob, shell, network, Skill load, sub-Agent, or write tool.

Alternative considered: let the model browse all dossier-linked resources. Rejected because references and scripts may be large, sensitive, or adversarial and are not needed for first-version guidance drafting.

### 9. Separate system instructions from untrusted data

Generation prompts use a versioned static control template. Dossier sections, Skill excerpts, model-provided rationales, and tool results appear only in structured data fields with explicit untrusted labels. Control text instructs the model to cite data, but security does not depend on compliance: tool registry and output schemas enforce boundaries.

Before outbound assembly, fields pass sanitizer, injection classification, size limits, and UTF-8 normalization. Injection markers remain safe metadata so the model can avoid repeating them. Provider output passes the same scanner before persistence; rejected bodies are kept only in transient memory and are not logged.

Alternative considered: strip all injection-like text silently. Rejected because the generator should know relevant content is unsafe, while the raw pattern still must not become control instructions.

### 10. Require cited structured plans and drafts

`MutationPlanV1` includes one draft kind, target witness or new-Skill intent, bounded rationale, lesson trigger/action/verification, source evidence ids, target excerpt ids, expected behavior, and verification steps. `StructuredDraftV1` contains only renderer fields.

Validation rejects unknown ids, uncited claims, target/scope changes, multiple mutations, unknown enums, unexpected properties, excessive strings, embedded files, code payloads, external dependencies, and control-like fields. At least one citation must support every lesson trigger, action, and verification claim. Citation presence is necessary but not sufficient; deterministic compatibility checks verify it.

No chain-of-thought field exists. Rationale is capped at 1,000 characters and treated as untrusted explanatory text.

Alternative considered: accept Markdown from the model and parse it. Rejected because freeform files make hidden extra sections, scripts, and unsupported claims harder to detect.

### 11. Render three artifact kinds locally

Renderers are pure and versioned:

- `LearnBlockRenderer` creates one bounded `OverlayLearnBlock` of at most 8 KiB.
- `ExactPatchRenderer` creates one patch with combined old/new strings at most 16 KiB and `replace_all=false`.
- `NewSkillRenderer` creates one UTF-8 `SKILL.md` at most 12 KiB, with body target below 2,000 characters and hard body limit 4 KiB.

The new Skill frontmatter includes stable kebab-case id, display name, description, explicit `role` or `utility` type, version, built-in tool dependencies, and provenance fields accepted by the Skill schema. It cannot declare scripts, arbitrary tool definitions, config schema, executable resources, remote dependencies, or extra files.

Renderers escape YAML and Markdown safely, normalize line endings, reject raw HTML/script constructs, and include no hidden comments or model metadata in effective instructions. Provenance lives in registry/audit state, not user-facing prompt content.

Alternative considered: store model output verbatim for transparency. Rejected because transparency is provided by structured attempts; executable content must never reach package storage.

### 12. Constrain existing-Skill drafts to the assessment

Existing-Skill generation requires a current selected target and Curator candidate. The target id, effective revision, and Overlay scope are server-owned fields. A learn block is preferred for additive guidance. An exact patch is allowed only when an excerpt supplies one exact stable anchor and the mutation is necessary to replace contradictory guidance.

The renderer produces one mutation. `replace_all` is never generated. If the anchor has zero or multiple matches, target text changed, or the patch would delete unrelated content, validation fails. Pinned state permits draft inspection but blocks packaging as ready-to-apply until the normal lifecycle issue is resolved; the generator cannot unpin.

Alternative considered: allow the generator to pick a better target during drafting. Rejected because target selection has its own audited assessment and must be rerun upstream.

### 13. Quarantine focused new Skill proposals

New-Skill generation is permitted only when target selection is `no_target`, evidence sufficiency and non-target quality checks pass, confidence in the uncovered capability is at least 0.90, at least three independent runs support it, and a user or Curator explicitly requests proposal generation. Orchestration never autonomously installs it.

Candidate ids are checked against effective, shadowed, reserved, quarantined, archived, and recently rejected ids. Only Project scope for a canonical workspace or User scope is allowed. The proposal is stored in SQLite quarantine as validated bytes and hash, not written into a Skill directory.

The Curator review package shows the complete `SKILL.md`, dossier, scope, type, id collision report, token estimate, tools, and validation. Approval invokes the normal Skill creation transaction with current catalog and workspace witnesses. Once created, it becomes a normal base Skill; future improvements use Overlay.

Alternative considered: create an empty base Skill and apply generated content as an Overlay. Rejected because an Overlay cannot supply the required canonical base metadata and would leave a meaningless package if disabled.

### 14. Run a layered validation pipeline

`validate_and_simulate` executes:

1. structured schema and citation integrity;
2. privacy sanitizer and injection scanner;
3. prohibited-content and executable-signature checks;
4. frontmatter/Markdown or Overlay mutation schema validation;
5. size and token budgets;
6. target, scope, attribution, and exact-anchor compatibility;
7. structural duplicate detection;
8. all nine draft-bound deterministic quality checks;
9. optional constrained model judge, which can only make the result stricter;
10. Overlay effective preview or new-Skill creation preview;
11. verification-plan completeness and expected invariant checks.

Any hard failure blocks packaging. One repair attempt may receive only stable validation reason codes and the prior structured fields; it cannot access rejected unsafe bodies. The repaired output is a new immutable attempt and reruns every check.

Alternative considered: let Curator discover invalid drafts. Rejected because reviewers need a bounded queue of structurally safe artifacts, though Curator still revalidates current witnesses.

### 15. Package immutable governance handoffs

`GeneratedReviewPackage` contains job/attempt, dossier revision, structured plan/draft, renderer, artifact bytes/hash, citations, validation report, preview witnesses, provider/model and usage, policy/consent, and permanent provenance exclusion from auto-apply.

Existing-Skill packages attach idempotently to the matching Curator candidate as a new draft revision. New-Skill packages create a specialized Curator creation candidate with the same decision/audit principles but a Skill creation preview instead of an Overlay diff. The model never invokes handoff tools itself; the native workflow performs it after validation.

Any user edit creates a user-edited derivative that retains model provenance and permanent auto exclusion. Approval remains interactive and current-preview bound.

Alternative considered: consider human editing sufficient to remove model provenance. Rejected because derivation and supporting claims remain model-originated and audit should not be laundered.

### 16. Integrate generation as optional governance routing work

The orchestration `route_governance` stage can request generation when:

- generation consent and provider readiness exist;
- a Curator candidate is `awaiting_draft` or a strong no-target proposal was explicitly requested;
- job and daily budgets permit;
- no current job/package exists for the same complete witness.

The run waits only within its generation/model budget. A blocked, failed, or budget-limited job leaves the Curator candidate available for manual drafting and records a safe stage outcome. Generation has its own queue and concurrency so it cannot consume evidence-ingestion capacity.

Alternative considered: add a ninth orchestration stage. Rejected because generation is an optional method of governance routing; keeping it as a bounded substep preserves the existing fixed run-stage contract.

### 17. Persist immutable attempts and quarantined content

Add SQLite tables:

- `evolution_generation_policy`
- `evolution_generation_jobs`
- `evolution_generation_job_sources`
- `evolution_evidence_dossiers`
- `evolution_evidence_dossier_sections`
- `evolution_evidence_dossier_links`
- `evolution_generation_stage_attempts`
- `evolution_generation_model_calls`
- `evolution_generation_tool_receipts`
- `evolution_generation_structured_results`
- `evolution_generated_drafts`
- `evolution_generation_validations`
- `evolution_generation_handoffs`
- `evolution_generated_skill_quarantine`
- `evolution_generation_exports`

Model-call rows store provider/model, purpose, schema/template versions, safe outcome, token counts, latency, and structured-response hash, not raw request/response. Unsafe output never reaches durable storage. Idempotency covers complete input witness and request id; explicit regeneration adds a new attempt id.

Default detailed retention is 180 days for failed/cancelled jobs and 365 days for completed review packages, bounded by evidence purge. Committed outcomes retain minimal Curator/Skill/Overlay provenance tombstones. Export manifests are tracked, but user-selected exported files are outside automatic deletion and the UI discloses that.

Alternative considered: keep generation artifacts in Skill directories. Rejected because unapproved drafts must not be discoverable or loadable as Skills.

### 18. Expose typed desktop and Web experiences

Extend `agent-service.ts` with policy/consent, job, stage, dossier, attempt, model/tool provenance, draft, validation, quarantine, export, cancellation, regeneration, and handoff models. Tauri invocation remains only in `tauri-agent-client.ts`.

Web/mock deterministically simulates all seven stages, three artifact kinds, validation repair, cancellation, supersession, cost counters, and Curator handoff. It marks provider and filesystem operations as mock and cannot install a real Skill.

The UI has generation policy, job queue/detail, thirteen-section dossier inspector, attempt comparison, rendered draft and diff/creation preview, validation matrix, and Curator navigation. It does not display hidden reasoning, raw prompts, rejected unsafe output, or direct mutation controls. Production modules remain under 300 lines.

Alternative considered: show generation only inside Curator. Rejected because dossier export, model consent, job budgets, attempts, and failures are system-level generation concerns, while Curator focuses on decisions.

## Risks / Trade-offs

- [Model generates plausible but unsupported guidance] → Require per-claim citations, deterministic compatibility, nine gates, preview, and Curator review.
- [Skill text injects the generator] → Use structured untrusted fields, five allowlisted tools, no arbitrary reads/writes, scanner, and strict schemas.
- [Dossier redaction removes needed context] → Preserve structured categories, counts, hashes, explicit completeness, and route uncertain cases to manual drafting.
- [Thirteen sections become too large] → Normalize records, bound excerpts, paginate UI, cap canonical size, and expose truncation counts.
- [New Skill proposals fragment the catalog] → Require no-target proof, three independent runs, narrow scope, duplicate/collision checks, and interactive creation.
- [Exact patches are brittle] → Require one current anchor, `replace_all=false`, current preview, and Curator revalidation.
- [Human edits launder model provenance] → Preserve derivation and permanent auto exclusion on all descendants.
- [Provider usage becomes expensive] → Default off, per-job/daily token budgets, limited calls/tools, visible usage, and cooperative cancellation.
- [Generation slows orchestration] → Separate queue/concurrency and treat failure or budget exhaustion as non-blocking governance routing.
- [Raw provider content leaks] → Persist only validated structured results and safe hashes; scan before logs or storage.
- [Quarantine content becomes accidentally loadable] → Keep it in SQLite outside Skill discovery paths and create files only through approved Skill creation.
- [Web mock misrepresents real generation] → Mark mock provenance and avoid provider/filesystem claims.

## Migration Plan

1. Complete and verify evidence, assessment, Overlay, Curator, orchestration, and Skill creation prerequisites.
2. Add generation/dossier enums, schemas, consent, witnesses, migrations, canonical builders, renderers, and pure tests with generation disabled.
3. Implement thirteen-section local dossiers, retention, purge, pagination, JSON/Markdown export, and UI inspection without any model calls.
4. Add seven-stage jobs, budgets, idempotency, cancellation, supersession, model adapter purpose, and five constrained tools behind disabled consent.
5. Add strict plan/draft schemas, local renderers, layered validation, one repair attempt, and adversarial prompt-injection/privacy corpora.
6. Add existing-Skill generation and Curator draft handoff; verify every generated and edited derivative remains excluded from auto-apply.
7. Add quarantined new-Skill proposals, collision checks, Curator creation review, and conflict-safe Skill creation commit.
8. Integrate optional generation into orchestration routing with independent queue and budgets; verify failures leave manual Curator workflow usable.
9. Add Tauri commands, shared frontend contracts, Tauri/Web adapters, policy and job UI, notifications, localization, accessibility, and E2E coverage.
10. Enable generation only for opted-in development workspaces after full privacy, citation, injection, validation, recovery, and project checks pass.

Rollback disables new generation requests and consent, cancels queued/model stages at safe boundaries, and stops orchestration from dispatching generation. Existing dossiers and review packages remain read-only; Curator can retain already handed-off drafts but no direct mutation occurs. Quarantined new Skill proposals remain undiscoverable and can be purged. Rollback never removes an already approved Skill or Overlay; those retain normal governance provenance. Re-enabling revalidates witnesses, provider policy, and quarantine integrity before resuming jobs.
