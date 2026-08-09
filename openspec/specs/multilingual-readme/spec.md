# multilingual-readme Specification

## Purpose
Define the maintained multilingual README set, its structural and factual parity rules, and how each README routes readers to language-appropriate documentation and published downloads.
## Requirements
### Requirement: Canonical multilingual README set
The repository SHALL maintain `README.md` as the canonical English project entry point and SHALL maintain reviewed Simplified Chinese and Japanese translations in `README.zh-CN.md` and `README.ja.md`.

#### Scenario: Reader selects a README language
- **WHEN** a reader opens any supported README
- **THEN** it SHALL provide navigation to English, Simplified Chinese, and Japanese versions
- **AND** the current language SHALL be identifiable without following another link

#### Scenario: Maintainer changes canonical content
- **WHEN** a maintainer changes a translatable canonical section or stable product fact in `README.md`
- **THEN** the same change SHALL update the corresponding Simplified Chinese and Japanese sections or produce an explicit parity-check failure
- **AND** CI SHALL NOT silently regenerate or overwrite either translation

### Requirement: README structural and factual parity
The multilingual README set SHALL preserve equivalent section order, command examples, repository-relative link targets, version facts, and delivered-versus-planned feature classifications across all supported languages. A version fact SHALL be compared by its declared value, independent of any presentation encoding a badge service requires. Documentation-guide links enclosed in a declared locale-scoped block SHALL be exempt from target equality, because each README routes readers to the guides written in its own language; the presence of exactly one such block in every README SHALL remain subject to parity.

#### Scenario: Validate equivalent README structure
- **WHEN** the documentation parity check runs
- **THEN** it SHALL compare stable section identifiers and their order across all three README files
- **AND** it SHALL report the file and missing, additional, or reordered section when parity fails

#### Scenario: Validate stable technical content
- **WHEN** a command block, relative documentation link outside the locale-scoped block, version fact, or roadmap classification differs between README languages
- **THEN** the documentation parity check SHALL fail with a reviewable description of the mismatch

#### Scenario: Route readers to same-language guides
- **WHEN** a README lists documentation guides inside its locale-scoped block
- **THEN** the documentation parity check SHALL accept link targets that differ from the other languages' blocks
- **AND** the link checker SHALL still verify that every target inside the block resolves to an existing file

#### Scenario: Translation omits its locale-scoped block
- **WHEN** a README carries no locale-scoped block, or carries more than one
- **THEN** the documentation parity check SHALL fail and name the file

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

### Requirement: README claims reflect implemented state
README feature claims SHALL distinguish delivered, preview, Web/mock-only, desktop-only, and planned behavior and SHALL not present a service-layer contract as an available user workflow when no user-visible path exists.

#### Scenario: Promote a feature to delivered
- **WHEN** a README changes a feature classification to delivered
- **THEN** the change SHALL reference an implemented and testable user-visible or documented developer path
