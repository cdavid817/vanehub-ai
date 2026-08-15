## ADDED Requirements

### Requirement: Chat file reference line ranges
A chat file reference SHALL be able to carry an optional line range naming the region of the file the user means. A reference without a range SHALL mean the whole file.

#### Scenario: Express a line range in the composer
- **WHEN** a user completes a mention as `@<path>:<start>-<end>`
- **THEN** the resulting reference SHALL carry that start and end line
- **AND** the range SHALL be interpreted as 1-based and inclusive of both bounds

#### Scenario: Express a single line
- **WHEN** a user completes a mention as `@<path>:<line>`
- **THEN** the resulting reference SHALL carry that line as both its start and its end

#### Scenario: Reference without a range
- **WHEN** a user completes a mention with no range suffix
- **THEN** the reference SHALL carry no range
- **AND** it SHALL behave exactly as file references did before ranges existed

#### Scenario: Candidate search ignores the range suffix
- **WHEN** the composer requests candidates for a mention token that already carries a range suffix
- **THEN** the query SHALL be the path portion only
- **AND** completion SHALL stay available while the range is being typed

#### Scenario: Reject a malformed range
- **WHEN** a reference carries only one of the two bounds, a bound below 1, or an end line before its start line
- **THEN** the system SHALL reject the reference with concise localized feedback and SHALL NOT send the message

#### Scenario: Range extends past the end of the file
- **WHEN** a reference names an end line beyond the file's last line
- **THEN** the system SHALL clamp the range to the last line rather than rejecting the reference
- **AND** a start line beyond the last line SHALL yield an empty region rather than an error

#### Scenario: Reference two regions of one file
- **WHEN** a user references the same path twice with different line ranges
- **THEN** both references SHALL be accepted as distinct
- **AND** the overall maximum number of file references per message SHALL still apply

#### Scenario: Reject an exact duplicate
- **WHEN** a user references the same path twice with the same range, or twice with no range
- **THEN** the system SHALL reject the duplicate with concise localized feedback

#### Scenario: Chips show the range
- **WHEN** a ranged reference is displayed in the composer or in message history
- **THEN** its chip SHALL show the referenced line range alongside the file name
- **AND** a reference without a range SHALL be displayed without range decoration

#### Scenario: Remove one of several references to a file
- **WHEN** a user removes one chip while another reference to the same path is attached
- **THEN** only the selected reference SHALL be removed and the other SHALL remain attached

#### Scenario: Persist and restore the range
- **WHEN** a message with ranged references is persisted and later restored into history
- **THEN** each restored reference SHALL retain its line range
- **AND** references persisted before ranges existed SHALL restore as whole-file references

### Requirement: Line-bounded file reference injection
Prompt assembly SHALL inline only the lines a reference names, so that referencing a region of a large file spends context proportional to the region rather than to the file.

#### Scenario: Inject only the requested lines
- **WHEN** a message is sent with a reference carrying a line range
- **THEN** the Agent prompt SHALL contain only the lines within that range
- **AND** the injected block SHALL identify the file and the range it covers

#### Scenario: Injected lines carry their positions
- **WHEN** a ranged reference is injected
- **THEN** each injected line SHALL be labelled with its 1-based position in the source file, so positions cited by the Agent match the user's editor

#### Scenario: Whole-file injection is unchanged
- **WHEN** a message is sent with a reference carrying no range
- **THEN** the Agent prompt SHALL contain the whole file exactly as it did before ranges existed

#### Scenario: Safeguards are not widened
- **WHEN** a ranged reference is resolved for injection
- **THEN** the existing path containment, oversize, and binary-file safeguards SHALL apply unchanged
- **AND** a range SHALL only narrow what is injected, never permit reading a file that would otherwise be refused
