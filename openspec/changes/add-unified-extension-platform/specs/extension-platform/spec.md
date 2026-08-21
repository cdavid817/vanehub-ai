## ADDED Requirements

### Requirement: Versioned extension manifest and namespaced identity

The system SHALL load extension metadata from a versioned `vanehub-extension.yaml` manifest and SHALL validate extension identity, publisher identity, semantic version, application compatibility, runtime declaration, activation events, dependencies, requested capabilities, and contributions before installation. External contribution ids SHALL be globally namespaced as `ext::<extension-id>::<kind>::<local-id>` and SHALL NOT overwrite native ids.

#### Scenario: Valid manifest is normalized

* WHEN a package contains a schema-version-1 manifest with valid namespaced identity and contribution-local ids
* THEN the system produces deterministic validated domain identities and a canonical manifest digest without executing package code

#### Scenario: Package claims a native tool id

* WHEN an external package attempts to register a contribution directly as a non-namespaced native id
* THEN validation rejects the package before snapshot publication or runtime activation

#### Scenario: Future schema is unsupported

* WHEN a manifest declares a schema version not supported by the running application
* THEN the system marks the package incompatible with an explicit diagnostic and does not guess at its security semantics

### Requirement: Declarative contributions are discoverable without runtime execution

The system SHALL index manifest-declared tools, Skills, MCP definitions, mode presets, Hooks, authorization rules, connectors, configuration schemas, and transforms without activating executable extension code. Runtime activation SHALL occur only for a matching activation event or explicit user operation.

#### Scenario: User inspects a disabled extension

* WHEN an installed extension is disabled or its runtime has never activated
* THEN its validated contribution summary, requested capabilities, dependencies, and eligibility reasons remain inspectable without executing its entrypoint

#### Scenario: Tool invocation triggers lazy activation

* WHEN an eligible extension tool is called and its owning runtime is cold
* THEN the system performs one single-flight activation and resumes all concurrent callers against the resulting generation or returns the same stable activation failure

### Requirement: Extension package extraction is bounded and path safe

The system SHALL inspect and extract `.vhext` archives only into application-owned quarantine paths using centrally enforced compressed-size, expanded-size, entry-count, per-entry, depth, normalized-path, and compression-ratio ceilings. It SHALL reject absolute paths, traversal, normalized duplicates, Unicode/case collisions, reserved names, alternate streams, links, devices, sockets, sparse surprises, unsupported entry kinds, and references outside the package root.

#### Scenario: Archive contains traversal

* WHEN any entry normalizes outside the quarantine root
* THEN the entire package is rejected, no entry is published, and the diagnostic identifies a safe archive-validation code

#### Scenario: Archive exceeds an expansion limit

* WHEN streamed extraction reaches any application ceiling
* THEN extraction is cancelled, partial quarantine content is safely reconciled, and the previous installed extension remains unchanged

### Requirement: Installed extension bytes are immutable and content-addressed

The system SHALL publish validated package bytes as immutable content-addressed snapshots. An installation SHALL point to one active snapshot and SHALL NOT mutate package files in place. Updates, reloads, rollbacks, and reinstalls SHALL create or select snapshots and atomically change pointers.

#### Scenario: Installed file drifts

* WHEN an active snapshot no longer matches its recorded content manifest
* THEN it becomes ineligible for new activation and recovery requires verified reinstall, rollback, or uninstall rather than drift adoption

#### Scenario: Update publication fails

* WHEN an update fails after quarantine or validation but before active-pointer commit
* THEN the previous active snapshot and contribution generation remain usable

### Requirement: Install and authority confirmation are witness-bound

The system SHALL produce an immutable install/update/uninstall preview witness covering package hash, signature state, publisher fingerprint, manifest digest, compatibility, installed state, dependency plan, contribution summary, requested capability diff, and selected trust profile. A mutation SHALL proceed only when the supplied witness still matches current critical state.

#### Scenario: Package changes after preview

* WHEN confirmation references a preview witness but the package hash, publisher trust, installed version, dependency state, or capability request changed
* THEN the operation fails as stale and requires a new preview

#### Scenario: Update expands authority

* WHEN a newer version requests broader filesystem, network, process, secret, Hook, rule, connector, or runtime authority
* THEN the preview shows the expansion and the update cannot inherit prior confirmation silently

### Requirement: Signature trust is separate from operational authority

The system SHALL verify signed external packages against explicitly trusted publisher keys and SHALL represent key revocation. A valid signature SHALL NOT auto-enable the extension, select a less restrictive trust profile, grant Agent operation permissions, trust Skill tools, or provide connector credentials.

#### Scenario: Signed package requests a network origin

* WHEN a package signature is valid but its manifest requests a new network origin
* THEN installation still requires capability review and runtime/operation policy continues to govern use of that origin

#### Scenario: Publisher key is revoked

* WHEN the active snapshot is signed only by a revoked publisher key
* THEN new activation is blocked or quarantined according to policy while bytes, provenance, logs, and recovery actions remain available

### Requirement: Unsigned packages require explicit Developer Mode containment

Unsigned external packages SHALL be rejected by default. Developer Mode MAY admit an unsigned package only as disabled with Strict trust, SHALL display and audit its unsigned state, SHALL prevent automatic startup activation, and SHALL NOT grant network, process, or secret authority.

#### Scenario: Unsigned install outside Developer Mode

* WHEN a user previews an unsigned `.vhext` while Developer Mode is disabled
* THEN installation is rejected before package publication

#### Scenario: Developer package is installed

* WHEN Developer Mode admits an unsigned package
* THEN it remains disabled and Strict until an explicit enable operation that cannot widen the Strict capability floor

### Requirement: Runtime kinds obey the trust matrix

The system SHALL support reviewed built-in Rust, capability-constrained WASM, and isolated sidecar runtime kinds. Built-in runtime SHALL be unavailable to external packages. WASM SHALL support Trusted, Standard, and Strict profiles. Sidecar SHALL be Trusted-only unless a platform sandbox provider is available, verified, and explicitly enables Standard; Strict sidecar SHALL be prohibited.

#### Scenario: External package declares built-in runtime

* WHEN an external manifest declares `runtime.kind: builtin`
* THEN manifest validation rejects the package

#### Scenario: Standard sidecar lacks a verified sandbox

* WHEN a Standard sidecar is enabled on a platform without a passing sandbox-provider self-test
* THEN activation fails closed with a platform capability diagnostic

### Requirement: Runtime calls are capability constrained and budgeted

Every runtime call SHALL use a call context containing installation, snapshot, runtime generation, contribution, session/operation correlation, cancellation, trust profile, and declared capabilities. The host SHALL enforce timeout, memory/resource, output, log, concurrency, scratch, filesystem, network, process, and secret limits before returning a result.

#### Scenario: Strict WASM requests network

* WHEN a Strict WASM runtime calls a network host function
* THEN the capability broker denies the call without opening a socket and records a redacted policy event

#### Scenario: Runtime exceeds callback budget

* WHEN a runtime call exceeds the maximum budget for its trust profile
* THEN it is interrupted/cancelled, returns a stable timeout error, and contributes to health/quarantine accounting

### Requirement: Sidecar protocol is bounded and process-isolated

Sidecars SHALL execute outside the Tauri process with scrubbed environment, application-owned working directory, bounded stdout/stderr, heartbeat, cancellation, process-tree termination, and length-prefixed versioned JSON-RPC. Undeclared or malformed host requests SHALL be rejected before side effects.

#### Scenario: Sidecar sends oversized frame

* WHEN a sidecar frame exceeds the configured maximum before complete decoding
* THEN the host terminates the protocol/runtime safely and records the bounded failure

#### Scenario: Sidecar spawns descendants and crashes

* WHEN a sidecar exits unexpectedly with child processes still running
* THEN the host terminates the owned process tree and updates crash-loop state

### Requirement: Contribution publication is atomic across adapters

The system SHALL prepare every affected contribution adapter, commit an immutable registry generation, and atomically swap the current generation. If any prepare or commit fails, the system SHALL compensate committed adapters and retain the previous generation; it SHALL NOT expose a partially active extension.

#### Scenario: MCP adapter commit fails after tool preparation

* WHEN one adapter fails during extension activation
* THEN no new extension contribution becomes visible to new calls and the previous generation remains current

#### Scenario: Activation succeeds

* WHEN all dependencies, runtime health, permissions, and adapter preparations succeed
* THEN all eligible contributions become visible together under one registry generation

### Requirement: In-flight calls pin their runtime and registry generation

Tool, Hook, and connector calls SHALL hold an immutable reference to the registry/runtime generation on which they began. Reload or disable SHALL route new calls to the new generation/state while the old generation drains within a bounded window.

#### Scenario: Reload occurs during a tool call

* WHEN a tool call is running while its extension reloads successfully
* THEN the call completes or is cancelled according to the old generation's drain policy and does not jump to the new runtime mid-call

#### Scenario: Drain window expires

* WHEN pinned old-generation calls remain after the configured drain deadline
* THEN they are cancelled, the old runtime is shut down, and the new generation remains authoritative

### Requirement: Hot reload preserves a known-good generation

Reload SHALL build and validate a shadow runtime generation, perform a health handshake, prepare contributions, atomically swap, drain the old generation, and retain rollback evidence. Failure before swap SHALL leave current behavior unchanged; failure during the stabilization window SHALL roll back to the prior known-good generation when safe.

#### Scenario: Shadow runtime fails initialization

* WHEN the target runtime cannot complete its version/health handshake
* THEN reload fails without changing the current registry or runtime generation

#### Scenario: New generation crashes during stabilization

* WHEN a newly swapped generation repeatedly fails within the stabilization window
* THEN the installation rolls back or enters quarantine with explicit evidence rather than repeatedly restarting indefinitely

### Requirement: Crash loops cause visible quarantine

The system SHALL count unexpected runtime exits, timeouts, protocol failures, and health failures using a bounded rolling policy. Crossing the policy SHALL move the installation to Quarantined, stop automatic activation, and expose reset, rollback, disable, inspect, and uninstall actions.

#### Scenario: Extension crosses crash threshold

* WHEN an extension reaches the configured crash-loop threshold
* THEN later activation requests fail fast with quarantine status until an authorized recovery action occurs

#### Scenario: User resets quarantine without fixing compatibility

* WHEN reset is requested but the package is incompatible or its publisher is revoked
* THEN reset does not make the extension eligible

### Requirement: Dependencies resolve deterministically

The system SHALL resolve required extension and Skill dependencies with semantic-version constraints, compatibility, installation/enablement, and deterministic topological ordering. Required cycles or missing dependencies SHALL block activation; optional dependency failure SHALL be visible without corrupting required contributions.

#### Scenario: Required extension dependency cycle exists

* WHEN two or more enabled extensions form a required cycle
* THEN activation is blocked for the affected strongly connected component with stable cycle diagnostics

#### Scenario: Optional Skill dependency is unavailable

* WHEN an optional Skill dependency is absent
* THEN contributions that explicitly require it are ineligible while unrelated eligible contributions MAY remain available according to the manifest contract

### Requirement: Existing domains remain authoritative through adapters

The extension platform SHALL integrate with Skills, Skill Tools, MCP, Agent Runtime, Permissions, Prompt Hooks, Communications, local extensions, tasks, credentials, and unified logging only through published APIs/ports. It SHALL NOT directly mutate another context's repositories or duplicate its authoritative store.

#### Scenario: Extension contributes a Skill

* WHEN a Skill contribution is activated
* THEN it is validated and projected through the Skill API as an immutable virtual Registry-layer package rather than written directly into Skill tables by extension infrastructure

#### Scenario: Extension contributes an MCP definition

* WHEN an MCP contribution is activated
* THEN MCP remains responsible for configuration, sessions, transport, credentials, bindings, and invocation

### Requirement: Capability gates separate build availability from runtime state

The system SHALL gate every Extension Platform capability through two independent layers: a compile-time build capability derived from Cargo features, and a persisted runtime desired state. Effective enablement SHALL be `build_available AND persisted_enabled AND prerequisites_satisfied AND NOT forced_disabled`. The gate set SHALL be a closed, strongly typed enumeration covering catalog, external packages, lifecycle Hooks, authorization rules, connectors, WASM module runtime, and sidecar runtime; arbitrary string keys SHALL NOT form the domain interface. Persistence SHALL store only desired state, revision, update time, actor, and optional reason; build availability SHALL be derived at evaluation time and SHALL NOT be persisted.

#### Scenario: Gate is compiled out but enabled in storage

* WHEN a persisted gate is enabled while its Cargo feature is absent from the running build
* THEN effective enablement is false and the reported state is a distinct not-compiled state rather than a runtime-disabled state

#### Scenario: Gate state cannot be read

* WHEN a gate record is missing, its identifier is unknown, or storage read or configuration parsing fails
* THEN evaluation fails closed to disabled and records a bounded safe diagnostic

#### Scenario: Caller enables an uncompiled gate

* WHEN an operator requests enablement of a gate whose build capability is unavailable
* THEN the operation fails with an explicit build-unavailability error and SHALL NOT report success or persist an enabled desired state as if it had taken effect

### Requirement: Gate authority is domain-owned and audited

Extension Platform SHALL own capability-gate authority inside its own bounded context and SHALL publish it through an application API or port together with an immutable cached snapshot. Hooks, Permissions, Connectors, Agent Runtime, Tauri commands, and the frontend SHALL query gates only through that contract and SHALL NOT read gate storage directly. Every gate mutation SHALL produce an audit record identifying gate, prior and new desired state, revision, actor, and reason. Gate state SHALL NOT be represented only in frontend or browser-local storage.

#### Scenario: Another context needs a gate value

* WHEN Agent Runtime or Permissions must know whether a gate is effective
* THEN it reads the published contract and never the gate repository or database rows

#### Scenario: Operator changes a gate

* WHEN a gate's desired state changes
* THEN the change is persisted with a new revision and an audit record, and stale revisions cannot silently overwrite it

### Requirement: Disabling a gate takes effect immediately and never silently restores state

Turning a gate off SHALL immediately reject new installation, activation, registration, and execution that depend on it, while work already running follows the existing lifecycle drain policy and a sidecar is terminated after its safe-shutdown timeout. Re-enabling a gate SHALL NOT automatically reactivate quarantined extensions or reverse a prior recovery decision.

#### Scenario: Gate is disabled while a runtime is active

* WHEN a capability gate is disabled while an extension runtime is serving pinned calls
* THEN new work is refused at once, in-flight work drains within its bounded window, and a sidecar that does not exit is terminated

#### Scenario: Gate is re-enabled after quarantine

* WHEN a gate is turned back on while an extension remains quarantined
* THEN the extension stays quarantined until an explicit authorized recovery action

### Requirement: Capability gates do not disable existing subsystems

Every gate SHALL be scoped to new Extension Platform behavior only. The lifecycle-Hook gate SHALL NOT disable existing Prompt Hooks; the authorization-rule gate SHALL NOT disable the existing Permissions decision point, immutable safety floors, or approval brokering; the connector gate SHALL NOT disable existing IM connectors; the WASM module runtime gate SHALL NOT change existing Skill Tool enablement; and the external-package gate SHALL NOT affect the existing built-in local OCR, ASR, or TTS capability pages.

#### Scenario: Every gate is disabled

* WHEN all Extension Platform gates are off
* THEN current Prompt Hooks, Permissions decisions and floors, IM connectors, Skill tools, and local capability management continue to behave exactly as before

#### Scenario: Authorization-rule gate is off

* WHEN compiled authorization rules are disabled
* THEN no operation becomes more permissive as a result and existing policy evaluation remains authoritative

### Requirement: External provider registration remains prohibited

Version 1 external extensions SHALL NOT contribute model providers, CLI providers, native dynamic libraries, or executable orchestration strategies. The system SHALL reject unknown contribution kinds and attempts to address the internal provider SDK.

#### Scenario: Package declares a model-provider contribution

* WHEN a manifest attempts to register a model provider or CLI provider
* THEN validation rejects the unsupported contribution with a stable non-extensible-provider error
