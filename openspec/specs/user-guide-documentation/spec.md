# user-guide-documentation Specification

## Purpose
Govern the localized, task-oriented user guides for VaneHub AI: their locale set, chapter equivalence across languages, truthful coverage of delivered workflows, and the determinism and safety of screenshots and media. Both guides are complete and equivalent sets describing the installed desktop application; neither is subordinate to the other, neither opens its chapters with a runtime or feature-state status line, and neither documents Web/mock browser-preview behavior as a user workflow.
## Requirements
### Requirement: User-guide locale set is distinct from application UI locales
The user-guide locale set SHALL be exactly English and Simplified Chinese. Application UI resource locales that are not in the user-guide locale set (for example, Japanese, Traditional Chinese, Korean) SHALL NOT be advertised as having a user guide, and README localization claims SHALL distinguish between delivered application UI resources and delivered user guides.

#### Scenario: Application UI locale has no user guide
- **WHEN** an application UI locale is delivered (for example, Japanese UI resources) but is not in the user-guide locale set
- **THEN** the README and documentation entry points SHALL NOT claim a user guide exists for that locale
- **AND** the README SHALL state which locales have application UI resources and which have user guides, as separate facts

#### Scenario: Reader expects a guide in a UI-only locale
- **WHEN** a reader who uses the Japanese UI looks for a Japanese user guide
- **THEN** the documentation SHALL state that user guides are available in English and Simplified Chinese only
- **AND** the documentation SHALL NOT present the absence of a Japanese guide as a defect or a broken promise

### Requirement: Deterministic documentation screenshots
User-guide screenshots SHALL be produced by named Playwright scenarios with fixed fixtures, viewport, locale, visual style, reduced motion, and sanitized dynamic content.

#### Scenario: Regenerate a screenshot
- **WHEN** a maintainer runs the documented screenshot generation command
- **THEN** it SHALL capture the expected named assets from deterministic Web/mock state
- **AND** repeated generation with the pinned environment SHALL not introduce unrelated timestamp, id, path, animation, or font changes

#### Scenario: Start an isolated capture server
- **WHEN** the screenshot command starts its Web/mock capture server
- **THEN** it SHALL select an available loopback port that is valid on the host
- **AND** it SHALL NOT reuse a server owned by another process or worktree

#### Scenario: Detect stale screenshot inventory
- **WHEN** a workflow changes its required screenshot set or an expected asset is missing
- **THEN** the screenshot check SHALL fail with the scenario and asset name

#### Scenario: Capture desktop-only behavior
- **WHEN** a screenshot represents behavior unavailable in Web/mock mode
- **THEN** it SHALL be explicitly labeled as a reviewed desktop capture
- **AND** no Web/mock capture SHALL be presented as evidence of native side effects

### Requirement: Safe and accessible guide media
Every documentation screenshot SHALL have localized descriptive alternative text and SHALL exclude credentials, tokens, personal filesystem paths, unredacted logs, and other sensitive information.

#### Scenario: Validate screenshot references
- **WHEN** documentation validation checks a referenced image
- **THEN** it SHALL require non-empty localized alternative text and an existing asset

#### Scenario: Prepare screenshot fixture data
- **WHEN** a screenshot scenario renders user, project, operation, or log data
- **THEN** it SHALL use synthetic values suitable for publication
- **AND** it SHALL not read sensitive local runtime state to populate the image

### Requirement: Guide media resolves from the authored source

An image reference in a guide chapter SHALL resolve from that chapter's committed location, not only from a location produced by a build step. A reference that resolves solely in an assembled-site layout SHALL be treated as broken.

Documentation validation MUST NOT rewrite an authored path to a location the path does not name in order to make it resolve. Where an authored path cannot resolve as written, validation SHALL report it.

#### Scenario: A chapter image is read from the repository

- **WHEN** a reader opens a guide chapter's Markdown at its committed path
- **THEN** every image the chapter references SHALL resolve to a committed file
- **AND** it SHALL do so without depending on a directory that a build step creates

#### Scenario: Validation compensates for a path instead of reporting it

- **WHEN** documentation validation resolves an authored media path by substituting a different directory from the one the path names
- **THEN** that substitution SHALL be treated as a defect in the authored path rather than as validation behavior to preserve

#### Scenario: A media path is authored incorrectly

- **WHEN** a chapter references an image by a path that resolves in neither the repository nor the assembled site
- **THEN** documentation validation SHALL fail and name the chapter and the unresolved path

#### Scenario: A locale's media is scoped to that locale

- **WHEN** a capture exists for one guide locale only
- **THEN** it SHALL be stored under that locale's book rather than in a location shared with a locale that does not reference it

### Requirement: A link resolves at its anchor, not only its file

Where a documentation link carries a fragment, the fragment SHALL identify a heading that exists in the target document. Documentation validation SHALL verify the fragment, not only the file, using the heading-identifier rules of the documentation toolchain.

A link SHALL be authored for the surface that entry points direct readers to. Where a project publishes no assembled site, a link that resolves only in an assembled site SHALL be treated as broken.

#### Scenario: A fragment names no heading

- **WHEN** a link's fragment does not match any heading identifier in the target document
- **THEN** documentation validation SHALL fail and name the linking file, the target, and the fragment

#### Scenario: A fragment is only checked as a file today

- **WHEN** documentation validation strips a fragment before checking a target
- **THEN** that SHALL be treated as missing coverage rather than as intended behavior

#### Scenario: Authored links follow the read surface

- **WHEN** entry points direct readers to documentation in one form and the project publishes no other form
- **THEN** cross-document links SHALL resolve in that form
- **AND** any transformation needed by a generated form SHALL be applied when generating it, not by authoring against it

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

