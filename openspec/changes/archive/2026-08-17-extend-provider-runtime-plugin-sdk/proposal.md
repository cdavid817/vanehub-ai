## Why

The existing provider runtime removes the first Session-level identity branches, but its contract still stops at invocation preparation and output-format selection. Adding another built-in CLI would therefore require coordinated edits across detection, parsing, cancellation, permission, usage, diagnostics, and tests instead of implementing one governed provider contract.

## What Changes

- Extend the internal, statically registered provider SDK to cover metadata, executable/version detection, side-effect-free readiness, capability negotiation, launch and prompt translation, incremental output parsing, resume, cancellation, permission mapping, model/reasoning options, usage extraction, and health diagnostics.
- Add a versioned, data-only provider manifest contract and strict parser. Manifests declare reviewed executable names and capabilities but cannot contain install hooks, scripts, shell fragments, arbitrary executable paths, or dynamic library entrypoints.
- Add a reusable provider conformance kit and run the five built-in CLI providers plus a test-only fixture provider through the same deterministic contract, parser, redaction, failure, and lifecycle cases.
- Keep provider-neutral Session orchestration independent of provider identity; callers resolve a provider and negotiate declared capabilities, receiving classified errors for unsupported behavior.
- Preserve existing Tauri commands, frontend service contracts, Web/mock behavior, persistence, CLI arguments, output semantics, and active Antigravity verification scope.
- Explicitly defer external package discovery/loading, installation, update, signature/source trust, quarantine, and marketplace behavior. Unreviewed external provider manifests remain non-loadable until a later OpenSpec change defines provider-package provenance and Sandbox/Trust integration.
- Add Provider SDK developer documentation covering the contract, manifest, fixture example, conformance testing, and security rules.

## Capabilities

### New Capabilities

- `provider-plugin-sdk`: Defines the internal provider SDK surface, versioned safe manifest, conformance kit, compatibility rules, documentation contract, and fail-closed boundary for external providers.

### Modified Capabilities

- `agent-provider-runtime`: Expands provider resolution into capability-negotiated execution, parsing, cancellation, permission, usage, version, readiness, and diagnostic behavior while retaining deterministic static registration.

## Impact

- Native `agent_runtime`: provider domain/application contracts, built-in compatibility adapters, parser/session capture, process gateway integration, composition-root registration, classified errors, and conformance/negative/benchmark tests.
- Native `tooling` and `permissions`: only existing published contracts may be consumed for executable detection, managed CLI metadata, permission mapping, and redacted diagnostics; no new bounded context or cross-context infrastructure import is introduced.
- Frontend/Web: no user-facing UI or service-contract change is planned. Contract checks prove the Web and Tauri adapters remain unchanged and compatible.
- Storage/migration: no SQLite or persisted-data migration.
- Dependencies/security: no runtime plugin loader and no new executable or package-install authority. A manifest parser dependency may be added only if its supply-chain review and strict data-only behavior are documented; otherwise existing serde support is used.
- Active changes: `verify-antigravity-cli-live-runtime` retains ownership of authenticated Antigravity capture and observed `step_update` mapping; this change only makes that adapter subject to the common SDK/conformance boundary.
