## ADDED Requirements

### Requirement: Connector-scoped session authorization
Every built-in IM connector SHALL require explicit enabled access for the target session before pairing or admitting an ordinary inbound message, and access granted to one connector SHALL NOT authorize another connector.

#### Scenario: Pair a non-Feishu connector with enabled access
- **WHEN** a valid unexpired pairing command from Telegram, DingTalk, WeCom, or personal WeChat targets a session with enabled access for that connector
- **THEN** the connector SHALL consume the code and create or replace the binding according to the existing binding rules

#### Scenario: Reject pairing without matching connector access
- **WHEN** a pairing command targets a session whose access is missing, disabled, or enabled only for a different connector
- **THEN** the connector SHALL reject the pairing without consuming the code, creating a binding, or revealing session metadata

#### Scenario: Reject inbound delivery after access is revoked
- **WHEN** an ordinary direct message resolves to a binding whose connector-specific session access is disabled
- **THEN** the message SHALL NOT append a session turn or start Agent work
- **AND** the connector SHALL return the localized disabled-state response

#### Scenario: Preserve access isolation during connector replacement
- **WHEN** a session replaces a binding from one connector with another connector
- **THEN** the new connector SHALL require its own enabled session access
- **AND** the previous connector's access state SHALL NOT authorize the replacement connector
