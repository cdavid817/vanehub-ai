## ADDED Requirements

### Requirement: Mechanically enforced frontend runtime boundary
Production React components and pages SHALL access runtime behavior only through frontend service interfaces and MUST NOT import Tauri APIs, call Tauri invocation functions, import native adapter implementations, or branch on native-runtime globals.

#### Scenario: Component imports a Tauri API
- **WHEN** a production React component or page imports an `@tauri-apps` module
- **THEN** frontend architecture fitness SHALL fail with the runtime-boundary rule id and exact source location

#### Scenario: Component invokes a native command
- **WHEN** a production React component or page calls an invocation function or reaches a Tauri-specific service adapter directly
- **THEN** frontend architecture fitness SHALL fail with the runtime-boundary rule id and repair guidance to use the service interface

#### Scenario: Component branches on native runtime
- **WHEN** a production React component or page reads a Tauri runtime global or imports a runtime detection helper to select native behavior
- **THEN** frontend architecture fitness SHALL fail with the runtime-branch rule id and repair guidance to move behavior behind adapters

### Requirement: Mechanically enforced runtime adapter parity
The desktop/Tauri and Web/mock runtime adapters SHALL both conform to the same declared frontend service contract, and architecture fitness SHALL reject removal of either conformance boundary.

#### Scenario: Service contract changes
- **WHEN** a frontend service operation is added, removed, or changes type
- **THEN** static contract validation SHALL require both the Tauri and Web/mock adapters to conform before architecture fitness passes

#### Scenario: Adapter conformance annotation is removed
- **WHEN** either runtime adapter is no longer checked against the shared service contract
- **THEN** frontend architecture fitness SHALL fail with the adapter-parity rule id and affected adapter file

