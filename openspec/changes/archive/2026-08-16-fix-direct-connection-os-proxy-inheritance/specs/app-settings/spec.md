## ADDED Requirements

### Requirement: Direct connection does not inherit external proxy configuration
VaneHub-managed native outbound requests SHALL be routed only according to the application's own persisted network proxy setting. When no proxy URL is configured, those requests SHALL connect directly and MUST NOT adopt proxy configuration discovered from the operating system, the environment, or any other source outside that setting.

#### Scenario: Operating system proxy is configured but VaneHub is not
- **WHEN** the host operating system or environment declares a proxy and no VaneHub proxy URL is persisted
- **THEN** VaneHub-managed native outbound requests SHALL connect directly
- **AND** the system SHALL NOT route them through the externally declared proxy

#### Scenario: VaneHub proxy is configured
- **WHEN** a VaneHub proxy URL is persisted
- **THEN** VaneHub-managed native outbound requests SHALL use that proxy rather than any externally declared one

### Requirement: Proxy bypass applies in every routing mode
The configured proxy bypass list SHALL apply to VaneHub-managed native outbound requests regardless of whether a VaneHub proxy URL is configured, so that loopback and bypassed destinations are never routed through a proxy.

#### Scenario: Loopback request while a proxy is configured
- **WHEN** a VaneHub-managed native request targets a destination covered by the bypass list and a VaneHub proxy URL is persisted
- **THEN** the request SHALL connect directly to that destination

#### Scenario: Loopback request in direct connection mode
- **WHEN** a VaneHub-managed native request targets a destination covered by the bypass list and no VaneHub proxy URL is persisted
- **THEN** the request SHALL connect directly to that destination
- **AND** the outcome SHALL NOT depend on an operating system bypass list or its wildcard syntax

#### Scenario: Every native client constructor is covered
- **WHEN** any VaneHub-managed native HTTP client is constructed, whether asynchronous or blocking and whether or not it follows redirects
- **THEN** it SHALL apply the same routing and bypass decision
