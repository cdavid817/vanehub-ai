## MODIFIED Requirements

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
