## MODIFIED Requirements

### Requirement: Chat file references
The chat composer SHALL allow users to reference files under the active session root by typing `@`. Candidate discovery for this completion SHALL be independent of the session document listing that backs the Documents tab, so that document-viewer type bounds do not constrain which files can be referenced.

#### Scenario: Show file candidates
- **WHEN** a user types `@` in the active-session chat composer
- **THEN** the composer SHALL request bounded file candidates through the frontend service boundary and show only files inside the active session root
- **AND** the candidate set SHALL NOT be derived from the Documents tab document listing

#### Scenario: Select file reference
- **WHEN** a user selects a file candidate
- **THEN** the composer SHALL show a visible file-reference chip and keep the reference associated with the draft until it is removed or sent

#### Scenario: Send message with references
- **WHEN** the user sends a message with one or more file references
- **THEN** the frontend service SHALL submit the text and file references together and the native runtime SHALL inject bounded file content into the Agent prompt

#### Scenario: Reject unsafe reference
- **WHEN** a referenced file is outside the session root, binary, oversized, or unavailable
- **THEN** the system SHALL reject or omit that reference with concise localized feedback without sending unrelated local files

#### Scenario: Persist reference metadata
- **WHEN** a message is sent with file references
- **THEN** the persisted user message SHALL retain safe reference metadata for history display and export

## ADDED Requirements

### Requirement: Chat file reference candidate search
The system SHALL resolve chat file-reference candidates by searching the active session root for a caller-supplied query, returning a ranked and bounded result set rather than an unranked prefix of the workspace.

#### Scenario: Match source files
- **WHEN** the composer requests candidates with a query that matches a source or configuration file under the session root
- **THEN** the result SHALL include that file
- **AND** eligibility SHALL NOT be restricted to Markdown and plain-text documents

#### Scenario: Rank by match quality
- **WHEN** candidates are returned for a query
- **THEN** they SHALL be ordered so that an exact filename match ranks above a filename prefix match, a filename prefix match ranks above a filename substring match, and a filename substring match ranks above a match found only across path segments

#### Scenario: Exclude vendored and generated trees
- **WHEN** the session root contains dependency installs, build outputs, compiler caches, or virtual environments
- **THEN** candidate search SHALL NOT descend into those directories
- **AND** the result budget SHALL be spent on first-party files

#### Scenario: Documents tab listing is unaffected
- **WHEN** the Documents tab lists documents for the same session root
- **THEN** its listing SHALL retain its existing Markdown and text bounds and its existing traversal behavior
- **AND** the exclusions applied to candidate search SHALL NOT change what the Documents tab shows

#### Scenario: Bound the search
- **WHEN** candidate search runs
- **THEN** it SHALL enforce a traversal depth limit and return no more than the requested number of results
- **AND** it SHALL resolve only paths contained within the active session root

#### Scenario: No match
- **WHEN** no file under the session root matches the query
- **THEN** the system SHALL return an empty candidate set without error
- **AND** the composer SHALL NOT present a file completion list

#### Scenario: Session root unavailable
- **WHEN** candidate search is requested for a session that has no resolvable root
- **THEN** the system SHALL return an empty candidate set with concise localized feedback rather than raw native diagnostics

#### Scenario: Web runtime parity
- **WHEN** candidate search is requested in Web/mock mode
- **THEN** the Web adapter SHALL serve the same service contract so the composer remains usable in browser mode
