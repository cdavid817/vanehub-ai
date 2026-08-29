## Purpose

Govern the localized, task-oriented user guides for VaneHub AI: their locale set, chapter equivalence across languages, truthful coverage of delivered workflows, and the determinism and safety of screenshots and media. Both guides are complete and equivalent sets describing the installed desktop application; neither is subordinate to the other, neither opens its chapters with a runtime or feature-state status line, and neither documents Web/mock browser-preview behavior as a user workflow.

## REMOVED Requirements

### Requirement: Localized task-oriented user guides
**Reason**: The authoritative-plus-transition model no longer describes the repository. Both guides are complete, the declared transition ended when `2026-08-21-complete-english-user-guide-content` was archived, and the requirement's runtime-labeling and Web/mock scenarios contradict the labeling decision applied in `26e66038`.

**Migration**: Use the `Two complete, equivalent user guides` requirement, which keeps the cross-language equivalence obligation and replaces the transition and runtime-path scenarios with symmetric ones.

### Requirement: Truthful feature-state labeling
**Reason**: Neither guide carries delivered, preview, planned, desktop-only, or Web/mock-only labels any more. Truthfulness is now achieved by omitting workflows a reader cannot exercise, rather than by labeling them.

**Migration**: Use the `Guides document delivered, user-visible workflows` requirement, which keeps the user-visible-path restriction and the verification obligation.

### Requirement: Authoritative guide covers delivered user-facing capabilities
**Reason**: There is no longer an authoritative guide and a subordinate one, so the coverage obligation and its partial-locale scenario no longer have a subject.

**Migration**: Use the `Both guides cover delivered user-facing capabilities` requirement, which applies the same coverage, exclusion, and rendered-label rules to each guide symmetrically.

## ADDED Requirements

### Requirement: Two complete, equivalent user guides
The repository SHALL provide English and Simplified Chinese user guides organized around user goals, with equivalent navigation, chapter topology, commands, prerequisites, results, and troubleshooting coverage. Both guides SHALL be complete sets, and neither SHALL be subordinate to the other: a chapter that exists in one SHALL exist in the other, under the same navigation order.

The user guides SHALL describe the desktop application a reader installs. Neither guide SHALL open a chapter with a runtime or feature-state status line, and neither SHALL document Web/mock browser-preview behavior as a user workflow. Where a step depends on something the reader must supply, install, or authorize — an authenticated CLI, a granted permission, a reachable host — the guide SHALL state that dependency in prose at the step it affects, rather than in a chapter-level label.

#### Scenario: Follow a guide in either supported language
- **WHEN** a reader selects English or Simplified Chinese
- **THEN** the user guide SHALL expose equivalent task chapters and workflow outcomes in that language
- **AND** product names, stable Agent ids, commands, paths, and configuration keys SHALL remain technically accurate

#### Scenario: Chapter sets stay equivalent
- **WHEN** a chapter is added to, removed from, or reordered in one guide
- **THEN** the other guide SHALL receive the same change in the same navigation position
- **AND** a chapter present in one guide and absent from the other SHALL be treated as a defect rather than as a declared partial state

#### Scenario: A chapter opens without a status line
- **WHEN** a reader opens any chapter of either guide
- **THEN** the chapter SHALL begin with its content rather than with a runtime or feature-state label
- **AND** the guide SHALL NOT describe browser-preview behavior, deterministic mock data, or simulated results as something the reader can do

#### Scenario: A step depends on the reader's environment
- **WHEN** a documented step cannot succeed without an installed CLI, a granted permission, or a reachable remote host
- **THEN** the guide SHALL state that dependency in prose at that step
- **AND** it SHALL NOT assert native process, SQLite, filesystem, or operating-system side effects that the step does not produce

### Requirement: Guides document delivered, user-visible workflows
Step-by-step instructions in either user guide SHALL be limited to user-visible paths a reader can exercise in the installed desktop application. A capability whose product UI is absent or disabled SHALL be omitted from the guides rather than documented under a label, and neither guide SHALL carry per-chapter or per-workflow delivered, preview, planned, desktop-only, or Web/mock-only labels.

#### Scenario: User-visible path is unavailable
- **WHEN** a service contract exists but its product UI is disabled or absent
- **THEN** the user guide SHALL omit fictitious control instructions
- **AND** the capability SHALL be left out of the guide rather than retained under a preview or planned label

#### Scenario: Delivered workflow is documented
- **WHEN** a workflow is documented as a task chapter or named section
- **THEN** an automated or recorded verification path SHALL exercise its user-visible controls from prerequisites through the documented result

#### Scenario: A documented workflow loses its user-visible path
- **WHEN** a capability's product UI is withdrawn after its workflow was documented
- **THEN** the corresponding instructions SHALL be removed from both guides in the same change
- **AND** leaving the instructions in place under a label SHALL NOT satisfy this requirement

### Requirement: Both guides cover delivered user-facing capabilities

Each user guide SHALL carry coverage for every delivered capability that a user exercises directly through the application. Coverage SHALL be a dedicated chapter or a named section of the chapter that already owns the subject; an incidental mention SHALL NOT count as coverage.

A delivered capability MAY be excluded only as a stated, reviewable decision recording why it is not user-facing. An unstated omission SHALL NOT be treated as an exclusion.

Coverage SHALL describe the capability's user-visible behavior using the labels the application actually renders, and SHALL NOT describe behavior that the capability's specification does not establish.

#### Scenario: A delivered capability has no coverage

- **WHEN** a capability that a user exercises directly through the application has shipped and a guide contains no chapter or named section covering it
- **THEN** that guide SHALL be considered non-compliant with this requirement
- **AND** an incidental mention of the capability elsewhere in the guide SHALL NOT satisfy it

#### Scenario: A capability is deliberately excluded

- **WHEN** a delivered capability is judged not user-facing
- **THEN** the exclusion SHALL be recorded as an explicit decision naming the capability and the reason
- **AND** the absence of that record SHALL be treated as a coverage gap rather than as an exclusion

#### Scenario: Coverage uses rendered labels

- **WHEN** guide coverage names a control, status, or surface of a capability
- **THEN** it SHALL use the label the application renders for that element in the guide's language
- **AND** it SHALL NOT assert behavior beyond what that capability's specification establishes

#### Scenario: Coverage lands in both guides together

- **WHEN** one guide gains coverage for a newly delivered capability
- **THEN** the other guide SHALL gain equivalent coverage in the same change
- **AND** deferring the second guide SHALL NOT be recorded as a declared transition
