## 1. Domain Model and Compatibility Parsing

- [x] 1.1 Add failing Rust domain tests for independent Skill type, delivery, layer, origin, trust, availability, aliases, and compatibility-default metadata.
- [x] 1.2 Implement the new Skill value types and extend `SKILL.md` parsing and serialization without weakening existing required-field validation.
- [x] 1.3 Add failing tests for canonical-id-first alias resolution, ambiguous aliases, and unsupported Utility availability.
- [x] 1.4 Implement canonical identity and alias resolution with structured conflict and unavailable outcomes.
- [x] 1.5 Extend application and API response models with effective metadata, shadow summaries, and compatibility state while preserving existing fields.

## 2. Layer Providers and Effective Catalog

- [x] 2.1 Define package descriptor, package reader, layer provider, and effective catalog ports in the Skill application boundary.
- [x] 2.2 Add failing resolver tests for `Project > User > Registry > System`, workspace omission, shadowing, invalid winners, and deterministic same-layer collisions.
- [x] 2.3 Implement the effective resolver and deterministic ordering independently of Agent binding logic.
- [x] 2.4 Implement bounded Project and User filesystem providers with canonical workspace boundaries, depth limits, excluded directories, and link-escape protection.
- [x] 2.5 Implement an empty local Registry provider and verify that the layer is represented without exposing install or network behavior.
- [x] 2.6 Add catalog cache and invalidation tests keyed by canonical workspace, package inventory fingerprint, and persisted state revision.
- [x] 2.7 Implement bounded effective catalog caching and invalidate it after managed Skill mutations, drift synchronization, and migration.

## 3. Immutable System Packages

- [x] 3.1 Create the versioned System package resource layout and manifest for the six existing built-in Skills, preserving their current instruction content and resources.
- [x] 3.2 Add manifest validation tests for canonical ids, versions, hashes, aliases, classification, and deterministic resource entries.
- [x] 3.3 Implement the immutable System layer provider and package reader behind the common provider ports.
- [x] 3.4 Add application tests proving System content can be listed and previewed but cannot be directly edited, deleted, or overwritten by import.
- [x] 3.5 Enforce layer mutability checks before filesystem transactions while keeping enablement and binding state mutable.
- [x] 3.6 Implement and test an application-managed read-only derived cache for CLI mount bindings that require a physical representation of a System package.

## 4. Built-in Migration and Persistence

- [x] 4.1 Add additive SQLite migrations for reconciliation version, effective metadata, and preserved deletion and enablement state, including upgrade tests from pre-change databases.
- [x] 4.2 Add failing migration tests for unchanged legacy built-ins, divergent user edits, missing records, tombstones, disabled state, invalid content, partial failure, crash recovery, and repeat execution.
- [x] 4.3 Implement content-aware reconciliation that converts unchanged built-ins to System authority and preserves divergent valid content as User-layer overrides.
- [x] 4.4 Implement recoverable cleanup of redundant unchanged legacy sources only after persisted migration state commits successfully.
- [x] 4.5 Change built-in restore to clear deletion intent and reveal the effective package without recreating a mutable System copy.
- [x] 4.6 Route bounded per-Skill migration outcomes and summaries through unified logging with redacted paths and no instruction bodies.
- [x] 4.7 Extend drift and synchronization behavior to distinguish intentional deletion, immutable System content, mutable overrides, and shadowed definitions.

## 5. Effective Bindings and Eager Prompt Assembly

- [x] 5.1 Add failing application tests showing existing canonical bindings follow the effective winner without being rewritten when layers change.
- [x] 5.2 Update Global and Workspace list, preview, enablement, restore, binding, and drift operations to use the effective catalog and return layer-aware results.
- [x] 5.3 Add prompt regression tests for legacy eager compatibility, explicit on-demand Role exclusion, Utility exclusion, shadowed-definition exclusion, workspace isolation, unreadable definitions, and deterministic budgets.
- [x] 5.4 Update native API prompt assembly to include only enabled, available, effective eager Role Skills while preserving the 8,000/16,000-character whole-body limits.
- [x] 5.5 Record one best-effort use event for each Skill included in the final generation prompt and verify tracking failure does not fail generation.

## 6. Usage Sidecars

- [x] 6.1 Define the versioned usage sidecar schema with layer-qualified keys, view/use counters, timestamps, revision witness, and reserved future counters.
- [x] 6.2 Add filesystem tests for Project and non-project sidecar placement, atomic replacement, concurrent updates, corrupt-file backup, backup retention, and write failure.
- [x] 6.3 Implement the usage repository using existing filesystem transaction primitives and bounded recoverable backups.
- [x] 6.4 Implement `bump_view` and `bump_use` application operations with best-effort unified logging and no package mutation.
- [x] 6.5 Expose bounded usage summaries in Skill management response models without making telemetry required for catalog availability.

## 7. Progressive Skill Loading and Resource Security

- [x] 7.1 Define bounded input and output models for list, load, and resource-read operations, including logical URIs, revisions, truncation, resource indexes, and structured refusals.
- [x] 7.2 Add security tests for absolute paths, parent traversal, hidden components, unindexed resources, stale revisions, escaping links, binary files, oversized files, excessive indexes, and path-length limits.
- [x] 7.3 Implement logical `skill://` URI generation and resolution without returning unrestricted host paths.
- [x] 7.4 Implement `list_skills` application behavior with bounded filters and metadata-only results.
- [x] 7.5 Implement `load_skill` with canonical id and alias lookup, Role-only enforcement, 12,000-character inline truncation, `{skill_base_dir}` replacement, resource indexing, and successful-view tracking.
- [x] 7.6 Implement `read_skill_resource` through the package reader with effective-revision checks and bounded text-only output.
- [x] 7.7 Add unified diagnostics for discovery and read refusals using safe identity, layer, operation, size, and reason fields only.

## 8. Native Agent Tool Integration

- [x] 8.1 Add provider-agnostic fixed schemas for `list_skills`, `load_skill`, and `read_skill_resource` to the native tool catalog.
- [x] 8.2 Add translation tests for Anthropic and OpenAI-compatible request shapes and prove inventory changes do not alter the fixed schemas.
- [x] 8.3 Add tool-loop tests for valid calls, malformed inputs, unavailable Skills, stale resources, bounded results, round limits, cancellation, and completed-message persistence.
- [x] 8.4 Dispatch the three tools through the effective Skill application service without granting generic filesystem or mutation authority.
- [x] 8.5 Update Plan mode catalog and execution enforcement so all three read-only Skill tools remain available while mutating Skill operations remain impossible.
- [x] 8.6 Verify existing shell, file, search, edit, memory, MCP, approval, and permission-mode behavior remains unchanged outside the additive Skill tools.

## 9. Frontend Contracts and Runtime Adapters

- [x] 9.1 Extend `src/types/skill.ts`, Skill contracts, and `agent-service.ts` with effective classification, layer, availability, shadowing, immutable state, usage, and bounded resource models without using `any`.
- [x] 9.2 Update `tauri-agent-client.ts` mappings for effective Skill inventory, preview, enablement, binding, restore, usage, and resource summaries; keep all `invoke()` calls inside the adapter.
- [x] 9.3 Update `web-agent-client.ts` with behaviorally representative System, User, Project, shadowed, unavailable Utility, truncated load, and resource-read cases using the same service contract.
- [x] 9.4 Add contract and adapter tests that assert desktop payload mapping and Web/mock behavior stay aligned.

## 10. Skills Settings Experience

- [x] 10.1 Update Skills page hooks and derived state to use canonical ids and one effective row per Skill while retaining selected-Agent and management-scope behavior.
- [x] 10.2 Update inventory rows with bounded type, delivery, layer, origin, version, compatibility, availability, usage, and shadowing labels.
- [x] 10.3 Add an accessible details presentation for precedence and shadowed definitions without rendering them as independently active cards.
- [x] 10.4 Update preview, create, edit, import, delete, and restore dialogs so System content is preview-only and mutable User definitions retain conflict-aware operations.
- [x] 10.5 Add localized explanations for immutable System packages, unavailable Utility execution, compatibility defaults, stale resources, and migration outcomes.
- [x] 10.6 Add component and interaction tests for desktop-equivalent Web data, keyboard focus, narrow and wide layouts, row-scoped mutation failures, and immutable controls.
- [x] 10.7 Run `npx playwright test` and resolve regressions in Skills settings and API-agent Skill interactions.

## 11. Verification and Documentation

- [x] 11.1 Update relevant developer documentation and contract fixtures to describe management scopes, runtime layers, fixed Skill tools, logical URIs, compatibility defaults, and deferred capabilities without external-product comparisons.
- [x] 11.2 Run `npm run lint:ci`.
- [x] 11.3 Run `npm run test` and `npm run test:coverage`.
- [x] 11.4 Run `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 11.5 Run `npm run build`.
- [x] 11.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 11.7 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 11.8 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 11.9 Run `openspec validate establish-effective-skill-runtime --strict` and `openspec validate --specs --strict`.
