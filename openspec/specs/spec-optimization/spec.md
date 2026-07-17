# spec-optimization Specification

## Purpose
TBD - created by archiving change add-spec-optimizer. Update Purpose after archive.
## Requirements
### Requirement: Read-only optimization analysis
The repository SHALL provide a project-local Spec Optimizer that analyzes `openspec/specs/` for duplicate requirement and scenario candidates, related capability clusters, and per-spec budget exceptions without editing main specs or archived changes.

#### Scenario: Analyze main specifications
- **WHEN** a maintainer runs the Spec Optimizer
- **THEN** it analyzes only the main spec corpus by default and emits review artifacts without modifying it

### Requirement: Traceable consolidation proposal
The Spec Optimizer SHALL emit a requirement-and-scenario mapping, SHALL/MUST coverage result, before-and-after diff, and proposed delta specs for every consolidation recommendation.

#### Scenario: Recommend a merge
- **WHEN** the Optimizer identifies related requirements for consolidation
- **THEN** every source requirement and scenario is mapped to a retained target or explicitly rejected with a reason

### Requirement: Budget-driven recommendations
The Spec Optimizer SHALL report specs above the default 500-line or 8,000-token budget and SHALL recommend splitting or consolidation without automatically changing the spec.

#### Scenario: Detect an oversized spec
- **WHEN** a main spec exceeds a configured default budget
- **THEN** the report identifies the budget exception and a reviewable remediation candidate

### Requirement: Human approval boundary
The repository MUST require explicit human review of generated mappings, coverage results, diffs, and delta specs before applying any consolidation to main specs.

#### Scenario: Review a proposal
- **WHEN** the Optimizer finishes an analysis
- **THEN** it leaves the main specs unchanged until a reviewer approves the corresponding OpenSpec change
