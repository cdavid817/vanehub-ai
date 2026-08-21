## ADDED Requirements

### Requirement: A link resolves at its anchor, not only its file

Where a documentation link carries a fragment, the fragment SHALL identify a heading that exists in the target document. Documentation validation SHALL verify the fragment, not only the file, using the heading-identifier rules of the documentation toolchain.

A link SHALL be authored for the surface that entry points direct readers to. Where a project publishes no assembled site, a link that resolves only in an assembled site SHALL be treated as broken.

#### Scenario: A fragment names no heading

- **WHEN** a link's fragment does not match any heading identifier in the target document
- **THEN** documentation validation SHALL fail and name the linking file, the target, and the fragment

#### Scenario: A fragment is only checked as a file today

- **WHEN** documentation validation strips a fragment before checking a target
- **THEN** that SHALL be treated as missing coverage rather than as intended behavior

#### Scenario: Authored links follow the read surface

- **WHEN** entry points direct readers to documentation in one form and the project publishes no other form
- **THEN** cross-document links SHALL resolve in that form
- **AND** any transformation needed by a generated form SHALL be applied when generating it, not by authoring against it
