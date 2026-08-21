## ADDED Requirements

### Requirement: Extensions may contribute read-only namespaced MCP definitions

An enabled extension MAY contribute an immutable namespaced MCP server definition containing supported transport and non-secret configuration metadata. MCP SHALL validate the definition and remain the owner of credentials, environment/header values, Agent bindings, connection/session lifecycle, tool discovery, limits, invocation, and shutdown.

#### Scenario: Extension MCP definition requires a secret header

* WHEN the manifest declares a credential field/reference
* THEN the package stores no secret value and the user configures the credential through the existing secure MCP flow

#### Scenario: Extension is disabled

* WHEN the owning extension is disabled
* THEN the definition becomes ineligible for new sessions atomically while active sessions follow current MCP shutdown/drain policy

### Requirement: Extension MCP definitions retain existing security floors

Tools discovered from extension-owned MCP definitions SHALL use existing namespacing, schema/result limits, call-time visibility validation, explicit approval floor, Hook lifecycle, audit, and cancellation. Package signature or Trusted runtime status SHALL NOT auto-approve MCP calls.

#### Scenario: Trusted extension supplies MCP server

* WHEN the server exposes a tool and the Agent invokes it
* THEN the normal MCP approval and execution path is used
