## ADDED Requirements

### Requirement: Feishu binding honors session opt-in
The Feishu connector SHALL accept pairing and ordinary inbound delivery only for a session whose IM enablement state is on, while connector configuration and health remain independently manageable.

#### Scenario: Pair with an enabled session
- **WHEN** a valid unexpired Feishu pairing command targets a session whose IM access is enabled
- **THEN** the connector SHALL consume the code and create or replace the binding according to the existing binding rules

#### Scenario: Pair with a disabled session
- **WHEN** a Feishu pairing command targets a session whose IM access is disabled
- **THEN** the connector SHALL reject the pairing without consuming the code, creating a binding, or revealing session metadata

#### Scenario: Manage connector while all sessions are disabled
- **WHEN** every session has IM access disabled
- **THEN** the user SHALL still be able to configure, test, enable, disable, and inspect the Feishu connector independently

#### Scenario: Re-enable a previously bound session
- **WHEN** a user re-enables IM access for a session whose Feishu binding was paused by session opt-out
- **THEN** the binding SHALL resume only if the connector is healthy
- **AND** it SHALL remain non-delivering with an explicit connector condition otherwise

