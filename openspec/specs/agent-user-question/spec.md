# agent-user-question Specification

## Purpose
TBD - created by archiving change add-agent-user-question. Update Purpose after archive.
## Requirements
### Requirement: Structured clarification round trip
The system SHALL expose an `ask_user_question` tool that submits one question with a bounded set of offered options, publishes it to the session's chat surface, and blocks that tool call until the user answers. The answer SHALL be returned to the model as the tool's result. The tool SHALL NOT return a placeholder, a default option, or a guessed answer if no answer has been given.

#### Scenario: Question is presented and answered
- **WHEN** the native agent calls `ask_user_question` in an interactive session
- **THEN** the system SHALL publish the question and its offered options to that session's chat surface
- **AND** the tool call SHALL remain unresolved until the user answers
- **AND** the answer SHALL be returned as the tool result

#### Scenario: Free-text answer is always accepted
- **WHEN** a user answers with text that matches none of the offered options
- **THEN** the system SHALL accept that text as the answer and return it unchanged to the model

#### Scenario: No default is substituted while waiting
- **WHEN** a question has been published and no answer has been given
- **THEN** the system SHALL NOT resolve the tool call with an offered option, an empty answer, or any other substitute

### Requirement: Question bounds
Each call SHALL carry exactly one question and between two and four offered options. The system SHALL reject a call with no question, with fewer than two options, with more than four options, with an empty or whitespace-only question or option, or whose question or option text exceeds its declared maximum length. A rejected call SHALL NOT publish anything to the chat surface and SHALL NOT block.

#### Scenario: Too few or too many options
- **WHEN** the native agent submits fewer than two or more than four options
- **THEN** the system SHALL reject the call with an explicit error
- **AND** it SHALL NOT publish a question or wait for an answer

#### Scenario: Empty or oversized text
- **WHEN** the question or any option is empty, whitespace-only, or longer than the declared maximum
- **THEN** the system SHALL reject the call with an explicit error

#### Scenario: A valid question at the bounds is accepted
- **WHEN** the native agent submits one question with exactly two options, or with exactly four
- **THEN** the system SHALL accept the call and publish the question

### Requirement: Distinct awaiting-input presentation
A tool call waiting on a user answer SHALL report a lifecycle status distinct from the status used for a call waiting on approval. The chat surface SHALL render a choice affordance for a waiting question and SHALL continue to render the allow/deny affordance for a waiting approval.

#### Scenario: Waiting question is distinguishable from waiting approval
- **WHEN** a tool call is waiting on a user answer
- **THEN** its reported status SHALL differ from the status reported by a call waiting on approval
- **AND** the chat surface SHALL present the offered options rather than an allow/deny control

#### Scenario: Approval presentation is unchanged
- **WHEN** a tool call is waiting on approval
- **THEN** the chat surface SHALL present the existing approval control unchanged

### Requirement: Non-interactive execution refuses to ask
The system SHALL NOT allow a question to block an execution context that has no interactive user, including Loop worker and verifier runs, scheduled-task runs, Plan attempt and repair sessions, and delegated Utility attempts. In those contexts the tool SHALL either be excluded from the offered catalog or fail immediately with an explicit non-interactive error.

#### Scenario: Loop or scheduled execution asks a question
- **WHEN** a non-interactive execution context invokes `ask_user_question`
- **THEN** the system SHALL return an immediate explicit error identifying the context as non-interactive
- **AND** it SHALL NOT publish a question or wait

#### Scenario: Delegated attempt asks a question
- **WHEN** a delegated Utility attempt invokes `ask_user_question`
- **THEN** the system SHALL refuse it without blocking the parent run

### Requirement: Cancellation and generation lifetime
A waiting question SHALL be cancelled when its generation is cancelled or its session ends, and the affected tool call SHALL terminate rather than remain blocked. A pending question SHALL be runtime state only: it SHALL NOT be persisted and SHALL NOT be answerable after a desktop restart.

#### Scenario: Generation is cancelled while a question waits
- **WHEN** a user cancels a generation that has a question waiting
- **THEN** the system SHALL stop waiting and terminate that tool call
- **AND** it SHALL NOT leave the generation blocked

#### Scenario: Answer for an unknown question
- **WHEN** an answer arrives for a question that never existed, that already resolved, or that belongs to a previous desktop run
- **THEN** the system SHALL reject it without resolving another tool call

### Requirement: Web runtime question parity
The Web/mock runtime SHALL expose the same question contract through the shared frontend service boundary with deterministic simulated behavior, and SHALL NOT claim that a real generation is blocked on the answer.

#### Scenario: Web mock question
- **WHEN** a question is presented in Web/mock mode
- **THEN** the adapter SHALL render it through the same service contract the desktop runtime uses
- **AND** it SHALL identify the round trip as simulated

### Requirement: User questions project canonical state
A valid interactive question SHALL transition its canonical Run to waiting user; an answer SHALL resume it, and cancellation or restart SHALL invalidate the ephemeral question without allowing a late answer.

#### Scenario: Restart interrupts a question
- **WHEN** the desktop restarts while a Run waits for a user answer
- **THEN** the question is invalidated and the Run records the owner-defined interrupted outcome rather than remaining waiting

