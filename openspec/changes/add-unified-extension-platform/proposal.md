## Why

VaneHub currently has mature but separate extension-related capabilities: SKILL.md discovery and sandboxed Skill tools, MCP client management, prompt-template Hooks, permission policies and approvals, built-in CLI readiness integrations, local OCR/speech extensions, and five messaging connectors. These capabilities converge in the Agent runtime, but they do not share one package contract, contribution registry, lifecycle, runtime isolation model, policy source model, connector SPI, or unified management surface.

The result is structural duplication and an unsafe path for future third-party extensibility. Adding another integration today requires feature-specific discovery, configuration, permissions, lifecycle, diagnostics, and UI. Treating the existing `plugin_integrations` readiness catalog as a general plugin system would also be misleading: it does not load programmable packages or contribute tools, Skills, Hooks, modes, rules, or connector drivers.

VaneHub needs a first-class extension platform that composes the existing capabilities without replacing their bounded-context ownership. The platform must use declarative manifests and lazy activation, isolate executable third-party code from the Tauri main process, preserve the existing permissions safety floor, support atomic activation and rollback, and make every contributed capability inspectable by the user before it becomes eligible for Agent execution.

## What Changes

* Add a versioned `.vhext` extension package and `vanehub-extension.yaml` manifest with namespaced identity, semantic version, application compatibility, activation events, dependencies, requested capabilities, trust profile, runtime declaration, and declarative contributions.
* Add `tooling::extension_platform` as the package, installation, dependency, lifecycle, runtime-generation, and contribution-registry owner inside the existing Tooling bounded context.
* Add three runtime kinds with an explicit trust matrix: reviewed built-in Rust extensions, capability-constrained WASM extensions, and isolated JSON-RPC sidecars. Arbitrary Python is never imported into the Tauri process; the optional Python SDK runs behind the sidecar protocol and is Trusted-only in the first release.
* Add immutable content-addressed package snapshots, bounded archive validation, publisher signature policy, install previews bound to immutable witnesses, enable/disable/reload/uninstall operations, lazy activation, crash-loop quarantine, and generation-based hot reload with rollback.
* Add a transactional contribution registry for tools, Skills, MCP definitions, declarative interaction-mode presets, typed Hooks, authorization rules, connectors, configuration schemas, and system/message transforms. A package either publishes all eligible contributions or none.
* Add `tooling::lifecycle_hooks`, a typed Agent lifecycle event bus and handler engine supporting built-in, extension-runtime, command, HTTP, MCP-tool, prompt, and read-only Agent handlers with event-specific decisions, budgets, failure modes, circuit breakers, redaction, audit, and a versioned Claude Code compatibility catalog.
* Add `permissions::rules`, a compiled AuthorizationRule layer over the existing Policy Decision Point. Rules support structured operation types, safe regex/glob matching, risk, effect, allowed approval scopes, source provenance, priority, expiry, atomic project-file reload, last-known-good fallback, simulation, and an immutable safety floor.
* Add `tooling::connectors`, a general Connector SPI for installation/configuration, authentication, health, connect/disconnect/reconnect, capability discovery, and Agent-facing projection. Migrate the built-in GitHub CLI readiness integration as the first native connector adapter and project existing messaging connectors into the unified catalog without moving their communications ownership.
* Integrate extension-contributed Skills through immutable virtual Registry-layer snapshots rather than creating a competing Skill store; integrate extension MCP definitions through read-only namespaced definitions with separately supplied secrets and bindings.
* Allow extensions to contribute declarative mode presets that compose registered VaneHub strategies, policies, tool groups, Skills, and Hooks. Loading arbitrary orchestration code as a mode is excluded from the first release.
* Add a unified Settings → Extensions workspace with Installed, Contributions, Hooks, Rules, Connections, and Diagnostics tabs; an install and permission-review wizard; extension detail and runtime diagnostics; Hook trace/test UI; rule editor/simulator; and connector authentication/health controls.
* Preserve the current local OCR/ASR/TTS extension subsystem, Prompt Hooks subsystem, Skills page, MCP page, Agent Policies page, and IM connector runtime. They are adapted or projected into the unified platform rather than rewritten.
* Keep external model-provider and CLI-provider packages out of scope. The existing internal provider SDK remains static and reviewed.

## Capabilities

### New Capabilities

* `extension-platform`: Defines extension packages, manifests, signatures, trust profiles, runtime hosts, lifecycle, dependency resolution, atomic contribution publication, lazy activation, hot reload, quarantine, and compatibility adapters.
* `agent-lifecycle-hooks`: Defines typed lifecycle events, handler kinds, matching, event-specific decisions, execution budgets, failure behavior, compatibility mapping, trace, and audit.
* `authorization-rule-management`: Defines rule sources, schema, compilation, precedence, safety floors, project YAML, simulation, approval interaction, diagnostics, and last-known-good reload.
* `connector-platform`: Defines connector descriptors, driver SPI, authentication strategies, secret handling, lifecycle, health, capabilities, bindings, and migration adapters.
* `settings-extension-platform-ui`: Defines the unified Extensions workspace, installation review, contribution inspection, Hooks, Rules, Connections, diagnostics, Web/mock behavior, accessibility, responsive layout, and route compatibility.

### Modified Capabilities

* `plugin-integration-management`: Projects existing readiness integrations into Connector Platform and deprecates the old feature-specific catalog as a public extension abstraction.
* `prompt-hook-management`: Projects prompt-template Hooks into the generalized Hook catalog while retaining their non-executable template semantics and current authoring workflow.
* `permissions-core`: Evaluates compiled authorization rules and Hook escalations while preserving explicit-Deny-first behavior, approval broker semantics, grants, audit, and immutable safety floors.
* `mcp-client-management`: Accepts read-only extension-owned MCP definitions, keeps credentials outside extension packages, and removes definitions atomically from new runtime snapshots when an extension is disabled.
* `effective-skill-runtime`: Resolves extension-contributed Skills as immutable virtual Registry-layer definitions and keeps package provenance separate from Skill tool trust, configuration, Overlay, and permissions.
* `agent-tool-execution`: Registers extension tools through the native tool catalog and applies the same validation, permission, Hook, timeout, output, tracing, and cancellation lifecycle as built-in and MCP tools.
* `interaction-modes`: Accepts declarative extension mode presets that reference registered runtime strategies; executable third-party mode strategies remain prohibited.
* `im-connector-management`: Projects existing Feishu, Telegram, DingTalk, WeCom, and WeChat connector state and operations into the unified connector catalog while Communications remains the source of truth.
* `local-extension-management`: Projects built-in OCR/ASR/TTS capabilities into the unified Extensions workspace without changing their existing installation or process ownership.
* `software-supply-chain-security`: Extends package integrity, signature, archive, immutable-snapshot, provenance, revocation, and developer-mode requirements to `.vhext` packages.

## Impact

* Native architecture: adds Tooling subdomains `extension_platform`, `lifecycle_hooks`, and `connectors`, plus a `rules` subdomain inside Permissions. Existing contexts communicate only through published APIs and ports.
* Agent runtime: emits typed Hook events, consumes immutable contribution-registry generations, lazily activates runtime-backed contributions, and pins in-flight calls to the generation that started them.
* Storage: adds additive SQLite state for extension packages/installations/snapshots/contributions/dependencies/runtime generations, Hook definitions/bindings/executions, compiled rule sets/rules, connector definitions/instances, trusted publisher keys, and operation witnesses. Secret values remain in the credential store.
* Filesystem: adds application-owned quarantine, immutable package, runtime scratch, and sidecar roots with strict path ownership and startup reconciliation.
* Frontend: adds an Extension Platform service contract with matching Tauri and Web/mock adapters; React components never call `invoke()` directly and new production TS/TSX files remain under 300 lines.
* Security: adds executable-content isolation, capability review, signature verification, archive limits, fail-closed activation, crash quarantine, permission floors, secret handles, safe Hook decisions, and redacted unified logs.
* Compatibility: existing feature routes and commands remain available through adapters for at least one release. Disabling the new feature flag restores current behavior without data loss.
* Delivery: the change is implemented through gated phases. External package execution remains disabled by default until manifest, package, permissions, lifecycle, adapter, and desktop tests pass.
