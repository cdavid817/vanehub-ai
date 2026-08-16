## ADDED Requirements

### Requirement: Architecture fitness CI gate
Continuous integration SHALL execute the repository architecture fitness command as an explicit named gate for every pull request and push to the main branch.

#### Scenario: Architecture remains conformant
- **WHEN** all registered frontend, native, and repository architecture checks pass
- **THEN** the named architecture fitness CI step SHALL succeed

#### Scenario: Architecture violation is introduced
- **WHEN** any registered architecture check reports a violation
- **THEN** the named architecture fitness CI step SHALL fail and expose the rule id, affected source location, and repair direction in job output

#### Scenario: Existing validation remains required
- **WHEN** the architecture fitness gate is added
- **THEN** existing lint, build, contract, coverage, Rust, browser, desktop, and strict OpenSpec validation SHALL remain enabled and SHALL NOT be weakened
