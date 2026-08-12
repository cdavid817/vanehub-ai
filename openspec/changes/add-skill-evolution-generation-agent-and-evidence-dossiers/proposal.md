## Why

The current pipeline can identify, assess, and govern improvement evidence, but most candidates still require users to author mutation text manually. VaneHub needs a constrained generation stage that converts evidence into reviewable drafts while preserving a complete, privacy-safe dossier and never granting the model mutation authority.

## What Changes

- Add a deterministic 13-section evidence dossier for every generation attempt: identity/provenance, executive summary, candidate seed, signal inventory, target selection, quality review, effective Skill snapshot, relevant guidance context, outcome timeline, privacy report, mutation rationale, verification plan, and lineage/version witnesses.
- Build dossiers locally from sanitized authoritative records, with completeness markers, content hashes, bounded excerpts, immutable revisions, JSON/Markdown projections, retention, and purge behavior.
- Add a seven-stage generation workflow: freeze input, inspect target, build dossier, plan mutation, synthesize structured draft, validate/simulate, and package for governance handoff.
- Use only a configured API model through a dedicated default-off generation consent. The generator never invokes the source CLI Agent and has no shell, filesystem-write, network-search, arbitrary retrieval, or mutation tools.
- Expose a small allowlisted read-only tool set for dossier section lookup, bounded effective-Skill excerpts, exact-anchor search, schema validation, and local preview simulation.
- Treat evidence and Skill content as untrusted data, require cited dossier section/evidence ids, validate strict structured output, reject invented targets or citations, and persist no raw assembled prompts or provider payloads.
- Generate three draft kinds: learned-guidance Overlay draft, exact-match Overlay patch draft, and new Skill proposal containing only a bounded `SKILL.md`. Scripts, supporting files, assets, executable content, tool registration, permission expansion, and direct base edits are prohibited.
- Restrict existing-Skill drafts to the assessed target and Overlay scope. New Skill proposals require strong no-target evidence, use User or Project scope only, remain quarantined, and cannot be installed until an interactive Curator review commits them through the normal Skill creation boundary.
- Render final Markdown and `SKILL.md` locally from validated structured model output; model text is never written directly to Skill or Overlay storage.
- Re-run privacy scanning, injection scanning, format validation, exact-anchor validation, token budgets, target compatibility, duplicate checks, all nine draft quality gates, and effective-content preview before a draft becomes reviewable.
- Preserve immutable generation attempts, model/schema/template versions, cited evidence, validation results, draft revisions, cancellations, supersession, costs, and Curator handoff references.
- Route every model-generated draft to Curator. Model-generated and user-edited derivatives remain permanently ineligible for automatic application.
- Integrate generation into the orchestration `route_governance` stage as a bounded optional substep; generation failure never blocks assessment routing or Agent execution.
- Expose generation consent, requests, jobs, stages, dossier inspection/export, draft comparison, validation, cancellation, regeneration, and Curator handoff through the Skill service boundary and matching desktop/Web adapters.
- Add generation and evidence-dossier views to Skill Evolution and publish sanitized completion, failure, and review-ready notifications.

## Capabilities

### New Capabilities

- `skill-evolution-generation`: Privacy-safe evidence dossiers, constrained seven-stage model generation, locally rendered mutation drafts and new Skill proposals, validation, audit, retention, and Curator handoff.

### Modified Capabilities

- `skill-management`: Adds generation policy, job, dossier, draft, validation, cancellation, regeneration, export, and safe new-Skill proposal operations through desktop and Web adapters.
- `settings-skill-management-ui`: Adds generation consent, job monitoring, dossier inspection, draft/diff review, validation explanations, regeneration, cancellation, and Curator handoff surfaces.
- `notification-system`: Adds sanitized, deduplicated generation completion, failure, cancellation, and review-ready notifications with navigation-only actions.

## Impact

- Desktop/runtime: adds Rust dossier, generation-job, constrained tool, structured model, renderer, validator, quarantine, and Curator-handoff services plus SQLite persistence and Tauri commands.
- Web runtime: adds matching deterministic mock jobs, dossiers, draft types, validation, cancellation, and handoff states without external model calls or filesystem installation claims.
- Frontend: extends `agent-service.ts` and both runtime adapters; React remains service-backed with no direct Tauri invocation.
- Data: stores sanitized dossier sections, immutable job/stage attempts, model provenance and usage, structured responses, locally rendered drafts, validations, quarantine metadata, and governance references. Raw prompts, provider request bodies, terminal output, secrets, and rejected unsafe output are excluded.
- Dependencies: requires evidence, assessment, effective Skill, Overlay, Curator, and orchestration capabilities. It does not add system-owned visible sessions, arbitrary model tools, automatic application, automatic new-Skill installation, or executable Skill generation.
- Logging: generation diagnostics and model usage use unified redacted logging; no feature-local log files are introduced.
