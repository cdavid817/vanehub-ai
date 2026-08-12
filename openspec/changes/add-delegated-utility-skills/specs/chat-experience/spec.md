## ADDED Requirements

### Requirement: Delegated Utility activity in chat
The chat experience SHALL render Utility delegation as a collapsible child activity attached to the parent assistant message, not as an independent conversation speaker. It SHALL show bounded lifecycle, Utility identity, status, elapsed time, tool and approval counts, result summary, and safe failure information.

#### Scenario: Delegation starts
- **WHEN** the frontend receives a Utility child-start event for the active parent message
- **THEN** it SHALL add or update one child activity identified by delegation attempt id without duplicating the parent assistant message

#### Scenario: Child tool activity updates
- **WHEN** bounded child tool or approval events arrive
- **THEN** the child activity SHALL update its counts and current state without rendering hidden prompts or full child transcripts

#### Scenario: Delegation completes
- **WHEN** the child returns a structured terminal result
- **THEN** the activity SHALL show its status, bounded summary, evidence references, limits, duration, and truncation state

#### Scenario: Parent response continues
- **WHEN** a Utility delegation returns success, denial, failure, limit, or cancellation
- **THEN** the parent generation MAY continue using that structured result unless the user stopped the parent

### Requirement: Delegation activity persistence
Completed parent messages SHALL persist their bounded Utility delegation activities and restore them in chronological attempt order when conversation history is loaded.

#### Scenario: Reload completed message
- **WHEN** a session containing delegated Utility work is reopened
- **THEN** the parent message SHALL restore each child activity with its terminal status and safe structured summary

#### Scenario: Interrupted delegation restored
- **WHEN** recovery marked an in-flight delegation interrupted after restart
- **THEN** chat history SHALL show the interrupted terminal state rather than a permanently running indicator

### Requirement: Child cancellation control
The chat experience SHALL let the user cancel a visible active Utility child independently from stopping the parent generation and SHALL keep the existing parent stop action able to cancel both.

#### Scenario: Cancel child only
- **WHEN** a user activates cancel on an active child activity
- **THEN** the frontend SHALL request child cancellation through the service boundary and keep the parent generation active

#### Scenario: Stop parent with active child
- **WHEN** a user activates the existing parent stop action while a child is active
- **THEN** the service SHALL cancel parent and child work and the UI SHALL retain already produced bounded activity

### Requirement: Web delegated activity parity
The Web/mock chat experience SHALL render the same Utility activity states, approval transitions, cancellation controls, persisted summaries, and errors from simulated adapter events without native side effects.

#### Scenario: Web child lifecycle
- **WHEN** the Web/mock adapter emits its deterministic Utility lifecycle
- **THEN** the chat UI SHALL render the same child activity contract used for desktop events

