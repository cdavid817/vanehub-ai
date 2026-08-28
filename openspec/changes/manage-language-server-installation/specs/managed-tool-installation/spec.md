## MODIFIED Requirements

### Requirement: Bounded archive extraction

The system SHALL extract a verified archive artifact only into a directory it owns, SHALL reject any entry whose path escapes that directory, and SHALL enforce declared limits on total extracted bytes and entry count. It MAY support more than one archive format, and every supported format SHALL enforce those bounds through the same containment and limit checks rather than through a copy of them.

#### Scenario: An archive entry escapes the destination

- **WHEN** an archive contains an entry whose path is absolute, contains a parent-directory component, or resolves outside the destination
- **THEN** extraction SHALL fail
- **AND** no entry from that archive SHALL be left in place

#### Scenario: An archive expands beyond its declared limits

- **WHEN** extraction would exceed the declared total byte ceiling or entry count
- **THEN** extraction SHALL fail before the limit is passed
- **AND** the partially extracted directory SHALL be removed

#### Scenario: Extraction succeeds

- **WHEN** a verified archive extracts within its declared limits
- **THEN** the resulting directory SHALL be reported to the caller
- **AND** the downloaded archive itself SHALL NOT be retained afterwards

#### Scenario: A second archive format is supported

- **WHEN** an artifact is published in a format other than the first one supported
- **THEN** its entries SHALL pass the same containment and limit checks as the first format's
- **AND** a format adapter SHALL NOT be able to write an entry the shared checks did not admit

#### Scenario: An archive entry is not a regular file or directory

- **WHEN** an archive contains a symbolic link, a hard link, or any other special entry
- **THEN** that entry SHALL be refused rather than recreated
- **AND** the refusal SHALL apply regardless of where the link points, because a link that resolves inside the destination today can resolve outside it after a later write
