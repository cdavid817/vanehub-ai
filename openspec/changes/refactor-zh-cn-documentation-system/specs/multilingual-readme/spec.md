## ADDED Requirements

### Requirement: README start instructions are executable
Every command a README gives for starting or building the project SHALL run as written against the repository at that revision. Because the parity check compares command blocks verbatim across languages, a correction to one README's command SHALL be applied to every supported language in the same change.

#### Scenario: Reader follows the desktop start instruction

- **WHEN** a reader runs the command a README gives for starting the desktop application
- **THEN** the command SHALL resolve to a defined package script

#### Scenario: Command corrected in one language only

- **WHEN** a command block is corrected in one README and not in the others
- **THEN** the parity check SHALL fail and name the diverging file

#### Scenario: Command carries a shell-specific prelude

- **WHEN** a README command block would only work in one shell
- **THEN** the block SHALL either be shell-neutral or SHALL state the shell it assumes

### Requirement: README routes to guides rather than reproducing them
A README SHALL route readers to the maintained guides by grouped entry point and SHALL NOT reproduce a guide's chapter list. A README SHALL NOT restate a component total, capability inventory, or catalogue that a linked guide or generated reference already owns.

#### Scenario: Reader looks for a chapter

- **WHEN** a reader opens a README seeking a specific documentation chapter
- **THEN** the README SHALL provide a grouped entry point into the guide that contains it
- **AND** the guide's own table of contents SHALL remain the complete list

#### Scenario: A component total changes

- **WHEN** the number of components a guide documents changes
- **THEN** no README SHALL require an edit
