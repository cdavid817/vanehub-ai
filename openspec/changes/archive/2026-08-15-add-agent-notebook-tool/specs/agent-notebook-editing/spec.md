## ADDED Requirements

### Requirement: Reading a notebook returns cells, not notebook JSON
The system SHALL provide a notebook read that returns each cell's position, identifier, type, and source as text, rather than the notebook's underlying JSON. Cell source stored as a sequence of lines SHALL be returned as one text value. The read SHALL be bounded by the same tool-output limit other reads observe, and SHALL report how many cells it returned out of the notebook's total.

#### Scenario: Notebook is read as cells
- **WHEN** the native agent reads a notebook
- **THEN** the result SHALL contain each cell's position, identifier, type, and source as text
- **AND** it SHALL NOT require the caller to interpret notebook JSON

#### Scenario: Read is bounded
- **WHEN** a notebook is larger than the tool-output limit allows
- **THEN** the result SHALL be truncated to that limit
- **AND** it SHALL state how many cells were returned out of the total

### Requirement: Output bytes never reach the read result
A notebook read SHALL summarize each cell's outputs. An output carrying image or other binary data SHALL be reported by its media type and size only, and the read SHALL NOT include those bytes or any encoding of them. An error output SHALL retain its error name and value.

#### Scenario: Cell with an image output
- **WHEN** a read encounters a cell whose output carries image data
- **THEN** the result SHALL report that output's media type and size
- **AND** it SHALL NOT contain the image bytes or any encoding of them

#### Scenario: Cell with an error output
- **WHEN** a read encounters a cell whose output is an error
- **THEN** the result SHALL retain the error's name and value

#### Scenario: Cell with text output
- **WHEN** a read encounters a cell with text output
- **THEN** the result SHALL include that text up to a declared bound

### Requirement: A cell is edited without composing notebook JSON
The system SHALL support replacing a cell's source, inserting a new cell, and deleting a cell, with the cell addressed either by its identifier or by its position. Exactly one addressing form SHALL be supplied. The system SHALL reject a call that supplies both, supplies neither, or names a cell that does not exist, and SHALL NOT modify the notebook in those cases.

#### Scenario: Cell source is replaced
- **WHEN** the native agent replaces an addressed cell's source
- **THEN** that cell's source SHALL be the supplied text
- **AND** no other cell SHALL change

#### Scenario: Cell is inserted
- **WHEN** the native agent inserts a cell of a given type at an addressed position
- **THEN** the notebook SHALL contain that new cell at that position

#### Scenario: Cell is deleted
- **WHEN** the native agent deletes an addressed cell
- **THEN** the notebook SHALL no longer contain it
- **AND** every other cell SHALL remain

#### Scenario: Address is ambiguous or absent
- **WHEN** an edit supplies both addressing forms, neither, or names a cell that does not exist
- **THEN** the system SHALL reject the call with the reason
- **AND** the notebook SHALL be unchanged

### Requirement: An edit changes only what it edits
Writing a notebook back SHALL preserve the content of every cell the edit did not change, and the notebook's own metadata, exactly as they were stored, including the ordering of their fields. Only an edited or inserted cell SHALL be rewritten.

#### Scenario: Untouched cells are preserved exactly
- **WHEN** one cell of a notebook is edited
- **THEN** every other cell SHALL be byte-identical to how it was stored
- **AND** the notebook's own metadata SHALL be byte-identical

#### Scenario: Interrupted write
- **WHEN** writing an edited notebook is interrupted
- **THEN** the notebook on disk SHALL be either its original content or the fully written new content

### Requirement: Outputs do not survive the source that produced them
When a code cell's source is changed, the system SHALL clear that cell's outputs and its execution count, so the file does not report a result its source can no longer produce. A cell type that carries neither SHALL be unaffected.

#### Scenario: Code cell source changes
- **WHEN** a code cell's source is replaced
- **THEN** that cell's outputs and execution count SHALL be cleared

#### Scenario: Markdown cell source changes
- **WHEN** a markdown cell's source is replaced
- **THEN** the cell SHALL be updated without gaining or losing execution state

### Requirement: A file that is not a readable notebook is refused
The system SHALL refuse a notebook operation on a file that is not valid JSON, that has no cell sequence, or whose declared notebook format is unsupported, reporting which. It SHALL NOT write to such a file.

#### Scenario: File is not a notebook
- **WHEN** a notebook operation targets a file that is not valid JSON or has no cell sequence
- **THEN** the system SHALL refuse it with the reason
- **AND** the file SHALL be unchanged

#### Scenario: Unsupported notebook format
- **WHEN** a notebook operation targets a file declaring an unsupported notebook format
- **THEN** the system SHALL refuse it with the reason
- **AND** the file SHALL be unchanged

### Requirement: Notebook access observes the workspace boundary and plan mode
Notebook operations SHALL apply the same workspace-relative path, hidden-path, and file-size rules the file tools apply. In plan mode the system SHALL offer notebook reading only, and SHALL reject a notebook operation that would modify a file.

#### Scenario: Path outside the workspace
- **WHEN** a notebook operation names a path that resolves outside the session's workspace folder
- **THEN** the system SHALL reject it

#### Scenario: Plan mode
- **WHEN** a generation starts in plan mode
- **THEN** the notebook tool SHALL offer its read operation only
- **AND** a notebook operation that would modify a file SHALL be rejected
