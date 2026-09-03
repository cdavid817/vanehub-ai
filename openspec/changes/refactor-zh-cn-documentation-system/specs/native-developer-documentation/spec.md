## ADDED Requirements

### Requirement: A chapter names the protocol it describes
A developer chapter SHALL identify a wire protocol by the name its own specification defines. A chapter SHALL NOT adopt one protocol's name as a generic label for a transport that several protocols share.

#### Scenario: Chapter describes stdio-framed JSON-RPC

- **WHEN** a chapter describes a child process that exchanges JSON-RPC messages over stdin and stdout
- **THEN** it SHALL name the transport by that mechanism
- **AND** it SHALL name the specific protocol carried over it where one applies

#### Scenario: Chapter would reuse a protocol name generically

- **WHEN** a chapter would use a named agent protocol as the label for an unrelated protocol's stdio binding
- **THEN** that label SHALL be replaced by the accurate protocol name

### Requirement: A stated component total agrees with the tree
Where a developer chapter states how many bounded contexts, modules, or generated artifacts the repository holds, that total SHALL equal the number present. Documentation validation SHALL check the stated total against the tree in the same run in which it already checks the chapter's table.

#### Scenario: Chapter's total disagrees with its own table

- **WHEN** a chapter's prose total differs from the number of rows validation derives from the tree
- **THEN** validation SHALL fail and report both numbers

#### Scenario: Chapter routes to the tree instead of counting

- **WHEN** a chapter describes the component set without stating a total
- **THEN** validation SHALL pass and the table SHALL remain the authority

### Requirement: A security posture is stated once per subject
Where the developer guide characterises a subsystem's security posture, that characterisation SHALL agree across every chapter that describes the same subsystem. A chapter SHALL NOT describe a subsystem as requiring no permission enforcement while another chapter documents the enforcement it receives.

#### Scenario: Two chapters characterise one subsystem

- **WHEN** one chapter classifies a subsystem as needing no permission system and another documents its sandbox, trust, and integrity controls
- **THEN** the classification SHALL be corrected to the enforced posture
- **AND** the distinction between a declarative artifact and an executable one SHALL be stated where both exist
