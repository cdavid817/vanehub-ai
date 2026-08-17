# user-guide-documentation Specification

## Purpose
Govern the localized, task-oriented user guides for VaneHub AI: their locale set, chapter equivalence across languages, runtime labeling, feature-state truthfulness, and the determinism and safety of screenshots and media. The Simplified Chinese guide is the authoritative complete set; the English guide mirrors its chapter topology and is allowed an explicit, declared partial state during a transition.
## Requirements
### Requirement: Localized task-oriented user guides
The repository SHALL provide English and Simplified Chinese user guides organized around user goals, with equivalent navigation, commands, runtime applicability, prerequisites, results, and troubleshooting coverage. The Simplified Chinese guide SHALL be the authoritative complete set; the English guide SHALL be rebuilt to the same chapter topology.

During a declared transition period recorded in an OpenSpec change, the English guide MAY be partial, but every missing chapter SHALL be marked explicitly as a known gap in the guide's navigation, and the English guide SHALL NOT silently diverge from the Simplified Chinese guide in navigation structure, runtime labeling, or truthful feature-state labeling. Outside any declared transition period, the unconditional equivalence requirement applies in full.

#### Scenario: Follow a guide in either supported language
- **WHEN** a reader selects English or Simplified Chinese
- **THEN** the user guide SHALL expose equivalent task chapters and workflow outcomes in that language
- **AND** product names, stable Agent ids, commands, paths, and configuration keys SHALL remain technically accurate

#### Scenario: English guide is partial during a declared transition
- **WHEN** an OpenSpec change declares a transition period during which the English guide is not yet complete
- **THEN** every English chapter absent from the complete Simplified Chinese set SHALL be represented in the English navigation as an explicit known-gap marker
- **AND** the English guide SHALL NOT silently omit a chapter that exists in the Simplified Chinese guide
- **AND** chapters that ARE present SHALL match the Simplified Chinese guide in navigation order, runtime labels, and feature-state labels

#### Scenario: Guide step differs by runtime
- **WHEN** a task has different desktop-native and Web/mock behavior
- **THEN** the guide SHALL label each runtime path before the divergent steps
- **AND** Web/mock instructions SHALL not claim native process, SQLite, filesystem, or operating-system side effects

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

### Requirement: Truthful feature-state labeling
Every user-guide workflow SHALL identify whether it is delivered, preview, Web/mock-only, desktop-only, or planned, and normal step-by-step instructions SHALL be limited to user-visible paths that can be exercised in the documented runtime.

#### Scenario: User-visible path is unavailable
- **WHEN** a service contract exists but its product UI is disabled or absent
- **THEN** the user guide SHALL omit fictitious control instructions
- **AND** any retained discussion SHALL be labeled as developer-facing or preview behavior

#### Scenario: Delivered workflow is documented
- **WHEN** a workflow is labeled delivered
- **THEN** an automated or recorded verification path SHALL exercise its user-visible controls from prerequisites through the documented result

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

### Requirement: Authoritative guide covers delivered user-facing capabilities

The authoritative user guide SHALL carry coverage for every delivered capability that a user exercises directly through the application. Coverage SHALL be a dedicated chapter or a named section of the chapter that already owns the subject; an incidental mention SHALL NOT count as coverage.

A delivered capability MAY be excluded only as a stated, reviewable decision recording why it is not user-facing. An unstated omission SHALL NOT be treated as an exclusion.

Coverage SHALL describe the capability's user-visible behavior using the labels the application actually renders, and SHALL NOT describe behavior that the capability's specification does not establish.

#### Scenario: A delivered capability has no coverage

- **WHEN** a capability that a user exercises directly through the application has shipped and the authoritative guide contains no chapter or named section covering it
- **THEN** the guide SHALL be considered non-compliant with this requirement
- **AND** an incidental mention of the capability elsewhere in the guide SHALL NOT satisfy it

#### Scenario: A capability is deliberately excluded

- **WHEN** a delivered capability is judged not user-facing
- **THEN** the exclusion SHALL be recorded as an explicit decision naming the capability and the reason
- **AND** the absence of that record SHALL be treated as a coverage gap rather than as an exclusion

#### Scenario: Coverage uses rendered labels

- **WHEN** guide coverage names a control, status, or surface of a capability
- **THEN** it SHALL use the label the application renders for that element in the guide's language
- **AND** it SHALL NOT assert behavior beyond what that capability's specification establishes

#### Scenario: A partial-locale guide inherits the coverage obligation

- **WHEN** the authoritative guide gains coverage for a capability while another locale's guide is in a declared transition
- **THEN** that locale SHALL represent the new chapter in its navigation as an explicit known gap
- **AND** it SHALL NOT silently omit the chapter

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
