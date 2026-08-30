## ADDED Requirements

### Requirement: Approval resolution has one claimant and one immutable decision

The native pending-approval broker SHALL atomically claim a pending request before resolution. One request id SHALL produce at most one immutable resolution id and decision; concurrent clicks, retries, timeout sweep, cancellation, and stale-generation handling SHALL reconcile to that same result rather than creating competing decisions.

#### Scenario: Two frontends resolve the same request concurrently

- **WHEN** two callers submit different decisions for the same pending request at the same time
- **THEN** exactly one caller SHALL claim and commit the resolution
- **AND** the other caller SHALL receive the existing resolving or terminal status without changing the decision, grant, audit, or execution outcome

#### Scenario: Retry after an ambiguous response

- **WHEN** a caller retries resolution because it did not receive the first response
- **THEN** the system SHALL return the resolution associated with the request id
- **AND** it SHALL NOT deliver the effect or write the audit a second time

### Requirement: Allow delivery is commit-before-effect and acknowledgement-gated

Before an approval can resume native Agent execution or release a Claude hook response with Allow, the system SHALL reserve the originating live waiter, commit the immutable resolution and audit, and only then deliver the effect with its resolution id. A remembered grant SHALL NOT become active until the waiter acknowledges delivery.

#### Scenario: Database fails before an approved action is delivered

- **WHEN** the user approves a pending action but the atomic resolution transaction fails
- **THEN** the Agent or hook waiter SHALL NOT receive Allow
- **AND** no active remembered grant SHALL exist for that attempt
- **AND** the pending claim SHALL be safely retryable or reported with a typed failure

#### Scenario: Delivery fails after durable commit

- **WHEN** the resolution and audit commit but the reserved waiter cannot acknowledge delivery
- **THEN** the resolution SHALL remain durable with a delivery-failed state
- **AND** any remembered grant intent SHALL remain inactive
- **AND** the system SHALL NOT convert the failed delivery into a second decision

#### Scenario: Stale generation is detected before commit

- **WHEN** the originating generation or hook waiter is no longer current when delivery is reserved
- **THEN** the system SHALL commit a stale-generation outcome without delivering the user's decision
- **AND** it SHALL NOT create or activate a remembered grant

### Requirement: Restart reconciliation never revives pre-restart work

The system SHALL reconcile committed or delivery-failed approval resolutions found at startup as aborted or delivery-unknown evidence. It MUST NOT recreate a pending approval, deliver the old effect to a new generation, or activate a grant whose original delivery was not acknowledged.

#### Scenario: Application stops after commit and before acknowledgement

- **WHEN** the next application launch finds a committed approval resolution without a recorded delivery acknowledgement
- **THEN** the system SHALL mark the delivery as aborted-by-restart or delivery-unknown
- **AND** it SHALL leave the related grant inactive
- **AND** no current generation SHALL be resumed from that record

### Requirement: Timeout storage failure remains an emergency fail-closed denial

Timeout and other non-human resolution paths SHALL use the same single-winner resolution flow. If durable storage is unavailable and waiting would violate the bounded approval timeout, the system MAY deliver only a Deny through an emergency fail-closed path, SHALL emit a redacted unified diagnostic, and MUST NOT create a grant or execute the action.

#### Scenario: Approval times out while SQLite is unavailable

- **WHEN** a pending approval reaches its timeout and the resolution transaction cannot be committed
- **THEN** the waiting provider SHALL receive a fail-closed denial rather than Allow or an unbounded wait
- **AND** the system SHALL write a bounded redacted diagnostic through unified logging
- **AND** a later retry SHALL NOT reinterpret the emergency denial as an approval

### Requirement: Approval UI and Web runtime expose resolution state safely

The frontend permission service SHALL expose a typed resolving and delivery result through both Tauri and Web/mock adapters. While a request is claimed or committed, approval controls SHALL be disabled or reconciled by pull so the user cannot submit a second competing decision. Web/mock SHALL simulate the same idempotency and grant-activation rules without native execution.

#### Scenario: Approval is being committed

- **WHEN** the frontend observes that one pending request is resolving
- **THEN** it SHALL prevent another Approve or Deny submission for that request
- **AND** it SHALL retain enough context to reconcile the terminal status through the service

#### Scenario: Web duplicate resolve

- **WHEN** Web/mock mode resolves one simulated request twice with the same request id
- **THEN** it SHALL return one simulated resolution and one delivery outcome
- **AND** it SHALL not create two simulated grants or execute two simulated actions
