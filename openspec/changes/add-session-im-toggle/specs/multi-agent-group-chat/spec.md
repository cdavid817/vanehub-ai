## ADDED Requirements

### Requirement: IM-originated human routing
An IM-originated user message in a multi-Agent session SHALL use the same stable seat identities and conversational routing semantics as a desktop-originated user message.

#### Scenario: IM user mentions one active seat
- **WHEN** an accepted IM message begins with exactly one valid active-seat mention
- **THEN** the system SHALL route the turn to that seat and preserve the IM origin for final-response delivery

#### Scenario: IM user does not mention a seat
- **WHEN** an accepted IM message has no valid line-leading seat mention
- **THEN** the system SHALL route it to the current conversational owner or the first active seat when no owner exists

#### Scenario: Agent handoff follows an IM turn
- **WHEN** the receiving Agent completes with a valid line-leading mention of another active seat
- **THEN** the existing bounded serial Agent-to-Agent handoff SHALL continue
- **AND** only the terminal response for the accepted external turn SHALL be delivered to its originating IM chat

#### Scenario: IM surface offers no dispatch override
- **WHEN** a multi-Agent session is enabled for IM
- **THEN** neither the information panel nor the external chat guidance SHALL offer a separate next-speaker selector

