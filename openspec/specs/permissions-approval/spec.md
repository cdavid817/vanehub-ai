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

### Requirement: Agent policy list surfaces every eligible agent's current template
The system SHALL provide a settings surface listing every custom API agent, the built-in OnePiece agent, and the five stable managed CLI principals (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`, `antigravity-cli`), each showing its currently assigned policy template, without requiring the user to inspect storage directly.

#### Scenario: Custom agents and OnePiece appear in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display every agent with `agentOrigin` of `user`, plus the OnePiece agent, each with its current policy template

#### Scenario: An agent with no explicit assignment shows the effective default
- **WHEN** a listed agent has never been assigned a policy template
- **THEN** the system SHALL display the current default template as its effective template, rather than an empty or unknown state

#### Scenario: The claude-code CLI principal appears in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display the `claude-code` principal alongside custom agents and OnePiece, with its current policy template or effective default

#### Scenario: The codex-cli, gemini-cli, and opencode CLI principals appear in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display the `codex-cli`, `gemini-cli`, and `opencode` principals alongside `claude-code`, custom agents, and OnePiece, each with its current policy template or effective default

#### Scenario: The antigravity-cli principal appears in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display the `antigravity-cli` principal alongside the other managed CLI principals, custom agents, and OnePiece, with its current policy template or effective default

### Requirement: Enabling Claude Code hook management requires a distinct first-use confirmation
The system SHALL, before the first policy template assignment to the `claude-code` principal takes effect, present a confirmation identifying that the action installs a permission hook into the user's global Claude Code configuration and affects Claude Code usage outside VaneHub, and SHALL NOT install that hook or apply the template until the user confirms. This confirmation is independent of, and in addition to, the existing trusted/yolo confirmation.

#### Scenario: First template assignment requires the installation confirmation
- **WHEN** a user assigns any policy template to the `claude-code` principal for the first time
- **THEN** the system SHALL present a confirmation naming the global `settings.json` side effect before installing the hook or applying the template

#### Scenario: Subsequent template changes do not repeat the installation confirmation
- **WHEN** a user changes the `claude-code` principal's template after the hook has already been installed
- **THEN** the system SHALL NOT present the installation confirmation again
- **AND** SHALL still present the existing trusted/yolo confirmation when the new template is `trusted` or `yolo`

#### Scenario: Declining the confirmation leaves the hook uninstalled
- **WHEN** a user declines the first-use confirmation
- **THEN** the system SHALL NOT write to `~/.claude/settings.json`
- **AND** the `claude-code` principal SHALL remain without an active hook

### Requirement: Reading a principal's policy template never creates it
The system SHALL be able to report an agent principal's current policy template without creating a stored principal record as a side effect of that read.

#### Scenario: Listing agents does not write principal rows
- **WHEN** the agent policy settings surface lists agents that have never been evaluated or explicitly assigned a template
- **THEN** the system SHALL NOT create a stored principal record for any of them as a result of that listing

### Requirement: Approval waits project canonical state
A pending approval for executing work SHALL transition its canonical Run to waiting approval and an allow, deny, expiry, generation end, or cancellation decision SHALL leave that state through the guarded transition contract.

#### Scenario: Late approval follows cancellation
- **WHEN** an approval arrives after its Run was cancelled
- **THEN** it is rejected and cannot resume or execute the cancelled work

### Requirement: Skill tool approval provenance
An approval request caused by a Skill tool SHALL identify the parent agent, Skill, tool, effective revision, requested capability, delegated host operation, target resource, risk level, and bounded redacted input summary. Approval of one operation MUST NOT approve the Skill revision or future operations.

#### Scenario: Skill tool requires approval
- **WHEN** unified permission evaluation returns Ask for a delegated Skill tool operation
- **THEN** the approval surface shows both Skill provenance and the concrete operation awaiting a decision

#### Scenario: User approves one operation
- **WHEN** the user approves a pending Skill tool operation
- **THEN** only that immutable request proceeds and later requests receive independent evaluation

### Requirement: Approval invalidation and fail-closed resolution
Pending approval SHALL become invalid if the parent generation is cancelled, the effective Skill revision changes, the tool is disabled or quarantined, or the immutable request witness no longer matches. A late approval MUST NOT revive invalid work.

#### Scenario: Revision changes while approval is pending
- **WHEN** a Skill tool's effective revision changes before the user decides
- **THEN** the pending request is invalidated and cannot execute under either revision

#### Scenario: Desktop approval channel is unavailable
- **WHEN** a protected Skill tool operation requires approval but no supported approval channel is available
- **THEN** the system denies the operation rather than executing it silently

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

