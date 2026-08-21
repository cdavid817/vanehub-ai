## ADDED Requirements

### Requirement: Built-in plugin integrations are projected as connector compatibility adapters

Existing built-in product/CLI readiness integrations SHALL be exposed through Connector Platform while legacy integration ids, readiness semantics, service methods, and Tauri commands remain compatible for at least one release. The legacy subsystem SHALL NOT be presented as a programmable external plugin runtime.

#### Scenario: Legacy GitHub readiness is requested

* WHEN an existing caller invokes the GitHub plugin-integration readiness test
* THEN the request delegates to the built-in GitHub Connector driver and returns a legacy-compatible configured/readiness result

#### Scenario: User opens legacy settings route

* WHEN the Plugin Integrations route is opened
* THEN the application redirects to the unified Connections view and identifies the corresponding built-in connector

### Requirement: New programmable packages do not register through the legacy catalog

External `.vhext` packages and their contributions SHALL be managed only by Extension Platform. The legacy plugin-integration catalog SHALL reject or ignore attempts to persist external executable definitions and SHALL not duplicate extension installation, signature, capability, lifecycle, or runtime state.

#### Scenario: Caller attempts to add arbitrary integration definition

* WHEN an external client attempts to create a programmable integration through the legacy API
* THEN the operation is rejected with guidance to the Extension Platform contract
