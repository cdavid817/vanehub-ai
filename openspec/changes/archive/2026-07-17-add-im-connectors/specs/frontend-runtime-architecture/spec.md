## ADDED Requirements

### Requirement: IM frontend service boundary
The frontend SHALL define one typed IM service contract implemented by both the Tauri desktop adapter and Web/mock adapter.

#### Scenario: React manages IM settings
- **WHEN** an IM settings component loads data or performs an action
- **THEN** it SHALL call the frontend IM service interface and SHALL NOT import Tauri `invoke`, native credential APIs, or platform SDKs

#### Scenario: Tauri adapter performs native call
- **WHEN** the desktop IM service performs a native operation
- **THEN** only the Tauri-specific IM adapter SHALL invoke the declared Rust command and normalize its result

### Requirement: Tauri and Web IM contract parity
The Tauri and Web/mock IM adapters SHALL expose the same method signatures and normalized model shapes.

#### Scenario: Contract conformance test
- **WHEN** frontend contract tests run
- **THEN** they SHALL verify that both adapters implement connector listing, status, routing, configuration, lifecycle, testing, authorization, and binding-reset operations

### Requirement: Honest Web/mock behavior
The Web/mock IM adapter SHALL provide deterministic UI behavior without claiming to establish live platform connections or securely persist real secrets.

#### Scenario: Use IM page in browser mode
- **WHEN** the IM page runs through the Web/mock adapter
- **THEN** it SHALL display mock connector states and simulated actions with a localized runtime limitation

#### Scenario: Enter secret in browser mock
- **WHEN** a user submits a connector secret through the Web/mock adapter
- **THEN** the adapter SHALL not persist the plaintext secret to localStorage or another browser persistence mechanism

