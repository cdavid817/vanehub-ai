## ADDED Requirements

### Requirement: Generation consent and controls
The Skill Evolution UI SHALL keep model generation disabled by default and SHALL disclose outbound sanitized data, provider/model, budgets, cost boundary, mandatory Curator review, and permanent auto-apply exclusion before enabling it.

#### Scenario: User enables generation
- **WHEN** the user confirms the current disclosure
- **THEN** the UI updates consent through the Skill service and shows its version and configured provider state

### Requirement: Generation job workspace
The UI SHALL show generation requests and jobs with source assessment, target or no-target intent, seven stages, progress, budgets, attempts, costs, cancellation, supersession, failure reasons, and Curator handoff status.

#### Scenario: Job is running
- **WHEN** a generation stage is active
- **THEN** the UI shows safe progress and permits cooperative cancellation without exposing raw model reasoning

### Requirement: Thirteen-section dossier inspector
The UI SHALL present all thirteen ordered dossier sections with completeness, redaction, truncation, source links, version hashes, pagination, and sanitized JSON/Markdown export.

#### Scenario: Section data is unavailable
- **WHEN** a dossier section has a declared unavailable reason
- **THEN** the UI shows that reason rather than hiding the section

### Requirement: Generated draft review
The UI SHALL distinguish learned guidance, exact patch, and quarantined new Skill proposals and SHALL show locally rendered content, evidence citations, validation results, effective diff or creation preview, model provenance, and permanent manual-review status.

#### Scenario: Generated patch is reviewable
- **WHEN** exact-anchor and all draft validations pass
- **THEN** the UI links the immutable generated draft to its Curator candidate without offering direct apply

#### Scenario: New Skill proposal is reviewable
- **WHEN** a quarantined `SKILL.md` passes validation
- **THEN** the UI shows id, scope, type, frontmatter, instructions, collision state, and Curator creation review

### Requirement: Safe regeneration and cancellation
The UI SHALL allow regeneration as a new immutable attempt and cooperative cancellation while preserving completed dossiers and prior attempt history.

#### Scenario: User regenerates
- **WHEN** the current witnesses remain valid
- **THEN** the UI creates a linked new attempt and retains the prior draft for comparison

### Requirement: Generation UI remains non-mutating
Generation screens SHALL NOT expose direct Overlay apply, automatic apply, automatic new-Skill install, target override, shell execution, tool permission, supporting-file creation, or provider-prompt editing.

#### Scenario: Draft validation succeeds
- **WHEN** generation packages a draft
- **THEN** the only mutation path presented is navigation to explicit Curator review

