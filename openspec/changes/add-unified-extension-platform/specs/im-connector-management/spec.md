## ADDED Requirements

### Requirement: IM connectors are projected into Connector Platform

Feishu, Telegram, DingTalk, WeCom, and WeChat connector metadata, lifecycle state, health, capabilities, and supported operations SHALL be projected through the published Communications API into the unified connector catalog. Communications SHALL remain the source of truth for configuration, credentials, transports, generation-safe transitions, inbound routing, deduplication, and outbound delivery.

#### Scenario: Unified page lists IM connectors

* WHEN Connections is loaded
* THEN every configured or available IM connector appears with source Communications and current authoritative status

#### Scenario: Unified reconnect is requested

* WHEN a reconnect action is initiated
* THEN Connector Platform delegates to Communications and reflects its stable operation/result

### Requirement: Projection does not duplicate messaging runtime

Connector Platform SHALL NOT start a second IM transport, copy raw credentials, persist an independent connection state, or reroute messages. Stale projected completions SHALL not overwrite newer Communications state.

#### Scenario: Communications state changes during unified operation

* WHEN a newer generation disables the connector before an older test/reconnect result returns
* THEN the stale result is ignored and Disabled remains authoritative
