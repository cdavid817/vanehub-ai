## Context

OpenSpec main specs contain normative requirements distributed across capability folders. Optimization must retain every requirement and scenario while keeping archived history immutable.

## Goals / Non-Goals

**Goals:** detect consolidation candidates, budget exceptions, and semantic overlap; produce traceable mappings, coverage checks, diffs, and proposed delta specs for review.

**Non-Goals:** modify `openspec/specs/`, edit `openspec/changes/archive/`, or automatically approve a merge.

## Decisions

- Implement a project-local Codex skill, not a VaneHub runtime feature.
- Analyze only `openspec/specs/` by default. Treat `### Requirement`, `#### Scenario`, `SHALL`, and `MUST` as tracked units.
- Use semantic judgement only to produce candidates; require a complete source-to-target mapping before proposing a merge.
- Enforce default warnings at 500 lines or 8,000 tokens per spec. Budgets generate recommendations, never automatic deletions.
- Emit reports and proposed delta specs in a new optimization change. A human applies them through the normal OpenSpec workflow.

## Risks / Trade-offs

- [False semantic match] -> label candidates and require reviewer approval.
- [Lost normative behavior] -> fail the report when a requirement, scenario, SHALL, or MUST has no retained mapping.
- [Archive corruption] -> exclude archive paths from analysis and edits.

## Migration Plan

Add the skill and its specification; no existing spec is moved or changed by this change.
