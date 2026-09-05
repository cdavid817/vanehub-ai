## ADDED Requirements

### Requirement: A documented boundary describes what is enforced
Where a guide describes a feature as providing isolation, separation, or protection, it SHALL describe what that feature actually constrains and SHALL state what it does not constrain. A guide SHALL NOT present a workspace or directory arrangement as a substitute for permission approval, command restriction, process isolation, or credential governance.

#### Scenario: Reader evaluates an isolation feature

- **WHEN** a guide describes a feature that separates work into distinct directories or branches
- **THEN** it SHALL state that the separation does not restrict a process's access to files outside it, to the network, or to credentials
- **AND** it SHALL point to the controls that do restrict those

#### Scenario: Guide would imply a security guarantee

- **WHEN** a guide would state that a feature removes the need for a separate safety control
- **THEN** that statement SHALL be replaced by the boundary the feature actually provides

### Requirement: A user-facing capability description matches the runtime
A user guide SHALL NOT state a limitation the runtime does not impose, nor a capability the runtime does not provide. Where a guide answers whether something is possible, that answer SHALL agree with the code path that decides it.

#### Scenario: Guide states an isolation limit that the runtime does not impose

- **WHEN** a guide answers that a per-Agent or per-workspace restriction is unavailable
- **AND** the runtime evaluates that restriction before a record can reach a prompt
- **THEN** the guide's answer SHALL be corrected to the runtime's behaviour

#### Scenario: Guide names a settings entry point

- **WHEN** a guide directs a reader to a settings page
- **THEN** the page name SHALL match the label the settings registry resolves for the reader's locale

#### Scenario: Guide states a capability count

- **WHEN** a guide states how many tools or capabilities a feature exposes
- **THEN** every statement of that count within the chapter SHALL agree

### Requirement: Inbound integration scope is stated where a reader would act on it
Where a guide describes an integration that accepts inbound messages, it SHALL state which message kinds reach Agent execution and which are acknowledged without executing.

#### Scenario: Reader plans an inbound integration

- **WHEN** a reader configures an inbound messaging integration from the user guide
- **THEN** the guide SHALL state the message kinds that trigger Agent execution
- **AND** it SHALL state that other kinds are acknowledged rather than executed
