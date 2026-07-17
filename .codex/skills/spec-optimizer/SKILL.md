---
name: spec-optimizer
description: Analyze and safely optimize OpenSpec main specifications. Use when asked to find semantically duplicate Requirements or Scenarios, cluster related capabilities, enforce spec line/token budgets, or generate reviewable consolidation mappings and diffs without changing main specs automatically.
---

# Spec Optimizer

Analyze `openspec/specs/` only. Never edit `openspec/specs/` or `openspec/changes/archive/` during analysis.

1. Parse capability, Requirement, Scenario, `SHALL`, and `MUST` units.
2. Report duplicate candidates and capability clusters as recommendations, not facts.
3. Flag specs above 500 lines or 8,000 tokens unless the user supplies different budgets.
4. For every proposed merge, create a mapping from each source Requirement and Scenario to a retained target or an explicit rejection reason.
5. Fail the coverage check when any source `SHALL` or `MUST` is missing from the mapping.
6. Write review artifacts in a new OpenSpec change: `analysis/report.md`, `analysis/mapping.md`, `analysis/coverage.md`, `analysis/diff.md`, and proposed delta specs.
7. Stop before applying changes. Ask the user to review the artifacts and use the normal OpenSpec workflow for approved edits.

Use the archive catalog only to locate historical context after a main-spec candidate is identified; do not optimize or rewrite archive artifacts.
