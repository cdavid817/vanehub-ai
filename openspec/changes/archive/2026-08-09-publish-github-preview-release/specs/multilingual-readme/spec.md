## MODIFIED Requirements

### Requirement: README structural and factual parity
The multilingual README set SHALL preserve equivalent section order, command examples, repository-relative link targets, version facts, and delivered-versus-planned feature classifications across all supported languages. A version fact SHALL be compared by its declared value, independent of any presentation encoding a badge service requires.

#### Scenario: Validate equivalent README structure
- **WHEN** the documentation parity check runs
- **THEN** it SHALL compare stable section identifiers and their order across all three README files
- **AND** it SHALL report the file and missing, additional, or reordered section when parity fails

#### Scenario: Validate stable technical content
- **WHEN** a command block, relative documentation link, version fact, or roadmap classification differs between README languages
- **THEN** the documentation parity check SHALL fail with a reviewable description of the mismatch

#### Scenario: Validate canonical manifest facts
- **WHEN** a stable README fact is owned by a repository manifest
- **THEN** the documentation parity check SHALL compare the canonical value with that manifest
- **AND** it SHALL fail when the README value is stale even if all translations share the same stale value

#### Scenario: Validate a pre-release version fact
- **WHEN** the manifest version carries a semantic-versioning pre-release identifier
- **THEN** the documentation parity check SHALL accept the README version fact that matches it exactly
- **AND** it SHALL decode the badge's escaped presentation before comparison rather than failing on the encoding

#### Scenario: Pre-release version fact is stale
- **WHEN** the manifest version's pre-release identifier advances and a README still declares the previous one
- **THEN** the documentation parity check SHALL fail

#### Scenario: Translate narrative content
- **WHEN** equivalent narrative text is expressed differently for linguistic quality
- **THEN** parity validation SHALL allow the translated wording
- **AND** reviewers SHALL remain responsible for semantic translation quality

### Requirement: Concise README documentation routing
Each README SHALL act as a concise project entry point and SHALL route detailed user, developer, contribution, troubleshooting, and release information to the appropriate maintained guide. Each README SHALL also route a reader to published downloads when the project publishes installable releases.

#### Scenario: Reader seeks first-use instructions
- **WHEN** a reader follows the quick-start or user-guide navigation from a README
- **THEN** the target SHALL identify the applicable language and runtime
- **AND** it SHALL not require the reader to infer whether a step applies to desktop or Web/mock mode

#### Scenario: Developer seeks architecture details
- **WHEN** a developer follows the developer-guide navigation from a README
- **THEN** the target SHALL provide the curated mdBook guide and a discoverable link to the generated native API reference

#### Scenario: Reader seeks a download
- **WHEN** a reader opens any supported README while installable releases are published
- **THEN** the README SHALL route them to the published releases
- **AND** it SHALL identify that the current published build is a preview when the latest published release is a pre-release

#### Scenario: Download routing stays in parity
- **WHEN** download routing is added or changed in the canonical README
- **THEN** the equivalent routing SHALL appear in every supported language
- **AND** the parity check SHALL fail when a language is missing it
