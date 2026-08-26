## MODIFIED Requirements

### Requirement: Server capabilities are negotiated before use
The client SHALL advertise only implemented capabilities, complete `initialize` followed by `initialized`, record the selected position encoding and text-document synchronization mode, and record which of the semantic methods it implements the server advertises. It SHALL issue a semantic request only when that record reports support for the method. The record SHALL be a list of negotiated methods rather than a fixed set of fields, so a method added to the client appears in it without any consumer being told the method's name in advance. Protocol readiness SHALL remain distinct from optional background indexing progress.

#### Scenario: Server selects no position encoding
- **WHEN** the initialize result omits a selected position encoding
- **THEN** the client SHALL use UTF-16 position semantics

#### Scenario: Hover is unsupported
- **WHEN** a configured server reports no hover capability
- **THEN** a hover query SHALL return an unavailable status without sending an unsupported request

#### Scenario: Server reports indexing progress
- **WHEN** a server publishes work-done progress after initialization
- **THEN** server status SHALL expose bounded warming or indexing detail
- **AND** protocol-ready requests SHALL remain eligible to run

#### Scenario: A server advertises a method the client does not implement
- **WHEN** an initialize result advertises a capability outside the set of methods the client implements
- **THEN** the negotiated record SHALL omit it
- **AND** the client SHALL NOT report it as available anywhere

#### Scenario: A method the client implements is absent from the initialize result
- **WHEN** an initialize result omits a capability for a method the client implements
- **THEN** the negotiated record SHALL report that method as unsupported rather than omitting it
- **AND** a request for it SHALL return unavailable without being sent

#### Scenario: Negotiated methods are reported in a stable order
- **WHEN** two servers negotiate the same set of methods
- **THEN** their negotiated records SHALL list those methods in the same order
- **AND** a consumer rendering the list SHALL NOT have to sort it to be deterministic
