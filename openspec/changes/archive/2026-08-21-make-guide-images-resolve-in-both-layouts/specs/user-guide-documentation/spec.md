## ADDED Requirements

### Requirement: Guide media resolves from the authored source

An image reference in a guide chapter SHALL resolve from that chapter's committed location, not only from a location produced by a build step. A reference that resolves solely in an assembled-site layout SHALL be treated as broken.

Documentation validation MUST NOT rewrite an authored path to a location the path does not name in order to make it resolve. Where an authored path cannot resolve as written, validation SHALL report it.

#### Scenario: A chapter image is read from the repository

- **WHEN** a reader opens a guide chapter's Markdown at its committed path
- **THEN** every image the chapter references SHALL resolve to a committed file
- **AND** it SHALL do so without depending on a directory that a build step creates

#### Scenario: Validation compensates for a path instead of reporting it

- **WHEN** documentation validation resolves an authored media path by substituting a different directory from the one the path names
- **THEN** that substitution SHALL be treated as a defect in the authored path rather than as validation behavior to preserve

#### Scenario: A media path is authored incorrectly

- **WHEN** a chapter references an image by a path that resolves in neither the repository nor the assembled site
- **THEN** documentation validation SHALL fail and name the chapter and the unresolved path

#### Scenario: A locale's media is scoped to that locale

- **WHEN** a capture exists for one guide locale only
- **THEN** it SHALL be stored under that locale's book rather than in a location shared with a locale that does not reference it
