## ADDED Requirements

### Requirement: Localized task-oriented user guides
The repository SHALL provide English and Simplified Chinese user guides organized around user goals, with equivalent navigation, commands, runtime applicability, prerequisites, results, and troubleshooting coverage.

#### Scenario: Follow a guide in either supported language
- **WHEN** a reader selects English or Simplified Chinese
- **THEN** the user guide SHALL expose equivalent task chapters and workflow outcomes in that language
- **AND** product names, stable Agent ids, commands, paths, and configuration keys SHALL remain technically accurate

#### Scenario: Guide step differs by runtime
- **WHEN** a task has different desktop-native and Web/mock behavior
- **THEN** the guide SHALL label each runtime path before the divergent steps
- **AND** Web/mock instructions SHALL not claim native process, SQLite, filesystem, or operating-system side effects

### Requirement: Truthful feature-state labeling
Every user-guide workflow SHALL identify whether it is delivered, preview, Web/mock-only, desktop-only, or planned, and normal step-by-step instructions SHALL be limited to user-visible paths that can be exercised in the documented runtime.

#### Scenario: User-visible path is unavailable
- **WHEN** a service contract exists but its product UI is disabled or absent
- **THEN** the user guide SHALL omit fictitious control instructions
- **AND** any retained discussion SHALL be labeled as developer-facing or preview behavior

#### Scenario: Delivered workflow is documented
- **WHEN** a workflow is labeled delivered
- **THEN** an automated or recorded verification path SHALL exercise its user-visible controls from prerequisites through the documented result

### Requirement: Representative Multi-Agent coding workflow
The user-guide set SHALL include a representative coding workflow that explains task decomposition, stable primary and fallback Agent selection, prerequisite relationships, execution progress, output propagation, cancellation, and final result review.

#### Scenario: Multi-Agent UI is available
- **WHEN** user-visible coordination controls can create and run a plan
- **THEN** the guide SHALL demonstrate a dependency graph with at least two independently ready implementation nodes and one dependent review or validation node
- **AND** the workflow SHALL show how the user observes actual Agent selection, node status, bounded output, and terminal run state

#### Scenario: Multi-Agent UI is not yet available
- **WHEN** implementation of this documentation change finds the creation UI disabled or absent
- **THEN** the developer guide MAY describe the coordination service and Web/mock contract
- **AND** the user guide SHALL label the workflow as preview or planned and SHALL not publish simulated screenshots as delivered desktop behavior

#### Scenario: Demonstrate fallback without misrepresenting failure
- **WHEN** a guide demonstrates ordered fallback behavior
- **THEN** it SHALL use a deterministic fixture or reproducible failure condition
- **AND** it SHALL explain that non-retryable validation, policy, cancellation, or context-bound failures do not start fallback Agents

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
