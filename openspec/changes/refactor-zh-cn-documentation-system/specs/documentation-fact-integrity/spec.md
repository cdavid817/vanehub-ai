## ADDED Requirements

### Requirement: Repository facts in prose are checked against their source
A repository fact that appears in documentation prose — an invocable script name, or a total that counts something the repository owns — SHALL be verified against the artifact that owns it rather than maintained by hand in parallel. Documentation validation SHALL fail when a stated fact disagrees with its source, and SHALL accept documentation that omits the fact and routes the reader to the owning artifact instead.

#### Scenario: Documented total disagrees with the tree it counts

- **WHEN** a documentation chapter states a total for a set of components the repository owns, and that total differs from the number of components present
- **THEN** documentation validation SHALL fail
- **AND** the failure SHALL report the stated total, the actual total, and the file

#### Scenario: Documentation omits a volatile total

- **WHEN** a documentation chapter describes the same set of components without stating a total
- **THEN** documentation validation SHALL pass

#### Scenario: A new component is added without updating prose

- **WHEN** a component is added to the tree and a chapter's stated total is not updated
- **THEN** documentation validation SHALL fail on the same run that would already fail for a missing table row

### Requirement: Documented commands are invocable
A command that documentation instructs a reader to run through the repository's package manager SHALL resolve to a script the repository defines. Documentation validation SHALL fail when a documented script name is absent from the manifest.

#### Scenario: README documents a script that does not exist

- **WHEN** a README instructs the reader to run a package script that the manifest does not define
- **THEN** documentation validation SHALL fail and SHALL name both the file and the unresolved script

#### Scenario: A script is renamed

- **WHEN** a package script is renamed and a README still instructs the reader to run the previous name
- **THEN** documentation validation SHALL fail

#### Scenario: Documented script exists

- **WHEN** every package script named in a README is defined by the manifest
- **THEN** documentation validation SHALL pass

### Requirement: A capability has one definition across the documentation set
A capability that is described in more than one document SHALL carry one definition of its scope, its isolation semantics, and its delivery status. Where two documents describe the same capability, they SHALL NOT state conditions that cannot both be true.

#### Scenario: Two guides describe the same capability differently

- **WHEN** a user guide and a developer guide describe the same runtime capability
- **THEN** neither SHALL assert a limitation that the other asserts is absent
- **AND** the description that disagrees with the implementation SHALL be the one corrected

#### Scenario: A capability's status is unresolved

- **WHEN** a capability's specification and its implementation disagree and neither has been chosen as authoritative
- **THEN** the documentation SHALL state both positions and SHALL NOT present the unimplemented one as delivered
