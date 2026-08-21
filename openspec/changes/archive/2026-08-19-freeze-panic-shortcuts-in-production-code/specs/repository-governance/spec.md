## MODIFIED Requirements

### Requirement: Existing source constraints remain enforced
The architecture gate SHALL preserve the repository's existing TypeScript, React, Rust, and file-size constraints and MUST NOT introduce a new blanket or permanent exemption. An existing oversized source path SHALL be governed by a recorded line budget rather than by disabling the file-size rule for that path. The prohibition on Rust panic shortcuts SHALL be enforced mechanically against non-test targets, and SHALL NOT be enforced against test targets, where the shortcuts are permitted.

#### Scenario: Production source violates an existing constraint
- **WHEN** production TypeScript uses explicit `any` or `@ts-ignore`, a new production TypeScript file exceeds 300 physical lines, or production Rust uses a prohibited panic shortcut
- **THEN** the configured repository checks SHALL reject the source

#### Scenario: Historical oversized path is exempted from the default limit
- **WHEN** a production source path is exempted from the default file-size limit because it predates the limit
- **THEN** the exemption SHALL take the form of a recorded line budget that bounds the path, and SHALL NOT take the form of disabling the file-size rule for that path

#### Scenario: Test code uses a panic shortcut
- **WHEN** Rust test code uses `unwrap()` or `expect()`
- **THEN** the panic-shortcut check SHALL NOT reject it, and no per-module exemption SHALL be required to keep it passing

#### Scenario: A production panic shortcut predates the check
- **WHEN** a production Rust file carried a panic shortcut before the check existed
- **THEN** its exemption SHALL be recorded at that file with the reason and the work expected to retire it, rather than by weakening the check for all files
