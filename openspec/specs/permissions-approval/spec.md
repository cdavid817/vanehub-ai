# permissions-approval Specification

## Purpose
TBD - created by archiving change add-permissions-core. Update Purpose after archive.
## Requirements
### Requirement: Pending approval state is Rust-side authoritative
The system SHALL hold the set of pending approval requests in the native runtime as the single source of truth, independent of whether any frontend event about them was received.

#### Scenario: Pending list is complete even after a missed event
- **WHEN** the frontend queries the current pending approval list at any time
- **THEN** the system SHALL return every approval request that is still pending in the native runtime, regardless of whether its creation event was ever delivered to that frontend instance

### Requirement: New pending approvals are pushed and reconciled by pull
The system SHALL emit an event when a new approval request becomes pending, and the frontend SHALL additionally fetch the full pending list when it mounts, so a missed event cannot leave a generation silently waiting forever.

#### Scenario: Event notifies of a new pending approval
- **WHEN** a tool call evaluation resolves to `Ask`
- **THEN** the system SHALL emit an event carrying the principal, action, resource, and risk level of the new pending approval

#### Scenario: Mounting reconciles missed events
- **WHEN** the frontend mounts or reconnects after having missed a pending-approval event
- **THEN** it SHALL fetch and display every still-pending approval request through the pending-list query, not rely on the missed event alone

### Requirement: Approval resolution accepts an explicit remembered-scope choice
The system SHALL let a user resolve a pending approval with both an approve/deny decision and a memory scope of `Once`, `Session`, `Project`, or `Global`, replacing the prior single-decision-only resolution.

#### Scenario: User approves with a remembered scope
- **WHEN** a user approves a pending request and selects `Session`, `Project`, or `Global` as the scope
- **THEN** the system SHALL execute the approved action and persist a grant at the selected scope

#### Scenario: User approves without remembering
- **WHEN** a user approves a pending request and selects `Once`
- **THEN** the system SHALL execute the approved action and SHALL NOT persist a grant

#### Scenario: User denies a pending request
- **WHEN** a user denies a pending request
- **THEN** the system SHALL NOT execute it
- **AND** SHALL report the denial back to the provider as the tool's result, allowing the generation to continue

### Requirement: Unresolved approvals expire as a fail-closed denial
The system SHALL resolve a pending approval that is not answered within its timeout window as a denial, report that denial as the tool's result, and record the timeout as the deciding mechanism.

#### Scenario: Pending approval times out
- **WHEN** a pending approval request is not resolved within the system's timeout window
- **THEN** the system SHALL resolve it as `Deny`
- **AND** SHALL report the denial back to the provider as the tool's result, allowing the generation to continue
- **AND** SHALL record the audit entry's deciding mechanism as timeout-based, not as a human decision

### Requirement: Resolution against an ended generation is rejected, not applied
The system SHALL reject an approval resolution that arrives after the generation it was raised during has already ended, and SHALL record that rejection distinctly from a normal decision.

#### Scenario: Late resolution after generation stopped
- **WHEN** a user resolves a pending approval after its originating generation has already ended (for example, because generation was stopped)
- **THEN** the system SHALL NOT execute the requested action as a result of that resolution
- **AND** SHALL record the audit entry as a stale-generation rejection rather than as the user's decision

### Requirement: Approval presentation shows principal, action, resource, and risk level
The system SHALL present every pending approval with the requesting principal, the requested action, the target resource, and the computed risk level, plus a memory-scope choice, before accepting a decision.

#### Scenario: Pending approval card shows required context
- **WHEN** a pending approval is displayed to the user
- **THEN** it SHALL show the requesting agent, the action, the resource, and the risk level
- **AND** SHALL offer a `Once`/`Session`/`Project`/`Global` scope choice alongside the approve/deny decision

### Requirement: Increasing a principal's trust requires explicit confirmation; decreasing it does not
The system SHALL require a distinct, explicit confirmation step before assigning a policy template that increases a principal's standing auto-allow surface (`trusted` or `yolo`), describing what is being granted, and SHALL NOT require confirmation when assigning a template that only narrows it (`standard` or `readonly`).

#### Scenario: Assigning trusted or yolo requires confirmation
- **WHEN** a user assigns the `trusted` or `yolo` template to an agent principal
- **THEN** the system SHALL present a confirmation describing that the agent will run shell commands and modify files without per-call approval
- **AND** SHALL NOT apply the template unless the user confirms

#### Scenario: Assigning standard or readonly takes effect immediately
- **WHEN** a user assigns the `standard` or `readonly` template to an agent principal
- **THEN** the system SHALL apply it without requiring confirmation

### Requirement: Pending-approval visibility uses the existing notification system
The system SHALL surface the existence of pending approvals to the user through the existing notification system rather than a separate, purpose-built indicator.

#### Scenario: New pending approval is visible without opening the session
- **WHEN** a new approval request becomes pending
- **THEN** the system SHALL make it visible through the existing notification system

### Requirement: Web runtime approval parity
The Web/mock runtime SHALL simulate the pending-approval queue, event-and-pull reconciliation, scoped resolution, and confirmation-on-increase behavior through the same service contracts the desktop runtime uses, without native side effects.

#### Scenario: Mock pending approval resolves through the same contract
- **WHEN** a user exercises the simulated tool-call sequence in Web/mock mode and a simulated call resolves to `Ask`
- **THEN** the Web adapter SHALL simulate a pending approval, its scoped resolution, and (when applicable) a confirmation step through the same event and service contracts the desktop runtime uses

