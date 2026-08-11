## Context

See `proposal.md` for motivation and the delta specifications for behavior. The current catalog is a Rust constant containing six inline Skill definitions that are materialized as mutable Global files. `establish-effective-skill-runtime` replaces that authority model with immutable System packages, aliases, explicit type and delivery, dependency availability, logical resources, and four-layer resolution. This change supplies the larger first-party content set for that runtime.

The catalog is product content and build input, but mistakes can affect prompts, delegated permissions, external integrations, package upgrades, and every Agent using a first-party Skill. Content therefore needs deterministic validation, reviewable identities, dependency honesty, and runtime fixtures rather than a loose directory of Markdown files.

## Goals / Non-Goals

**Goals:**

- Ship exactly 28 coherent VaneHub-native first-party packages with stable identities.
- Preserve the six existing ids, behavior, state, overrides, usage, and Overlays.
- Make Role, Utility, delivery, dependency, alias, and resource metadata explicit.
- Keep primary instructions concise through progressive disclosure.
- Fail packaging when content, resources, metadata, or fixtures violate governance.
- Present unavailable dependencies honestly in desktop and Web/mock experiences.
- Make future package additions follow the same manifest and validation pipeline.

**Non-Goals:**

- Implementing capabilities that a Skill describes, such as document rendering, image input, network integrations, or cloud accounts.
- Executing bundled scripts or dynamically registering package functions.
- Automatically assigning all new packages to every Agent.
- Copying System packages into User scope for customization.
- Adding remote registry installation, Skill configuration UI, evolution candidates, or Curator behavior.
- Treating localized names as identities or creating duplicate reviewer packages for aliases.

## Decisions

### 1. Use one checked-in package tree and generated manifest

First-party source packages live under a dedicated application resource tree:

```text
src-tauri/resources/skills/
├─ developer/
│  ├─ SKILL.md
│  └─ references/...
├─ code-explorer/
│  ├─ SKILL.md
│  └─ references/...
└─ ...
```

A build-time catalog generator scans only immediate package directories, validates them through the same parser and rules used by the runtime, sorts by canonical id, and emits a deterministic manifest containing metadata, resource inventory, individual hashes, complete package hash, dependency declarations, and selection-fixture hash. The Tauri application packages the validated tree as immutable resources.

The checked-in source tree is reviewable; the generated manifest is build output, not hand-edited authority. Runtime startup verifies the manifest version and package hashes before exposing the System catalog. Tests can use the source tree directly while installed builds use application resources through the System package provider.

Alternatives considered:

- Keep adding large inline Rust string constants. Rejected because multi-file resources, reviews, localization, and content linting become unmanageable.
- Check in a manually maintained manifest with hashes. Rejected because content and hashes drift and create noisy manual updates.
- Download first-party content after installation. Rejected because core availability would depend on a network registry and weaken reproducibility.

### 2. Fix the identity, category, type, and delivery matrix

The manifest generator compares every package against this checked-in catalog contract:

| Category | Skill | Type | Delivery |
|---|---|---|---|
| Development | `developer` | Role | On-demand |
| Development | `code-explorer` | Utility | On-demand |
| Development | `code-review` | Role | Eager |
| Development | `code-modification-mr` | Utility | On-demand |
| Development | `fix-vulnerability` | Utility | On-demand |
| Development | `mcp-builder` | Role | On-demand |
| Development | `plugin-creator` | Role | On-demand |
| Development | `tdd-discipline` | Role | Eager |
| Development | `code-security-scan` | Role | Eager |
| Development | `unit-test-generation` | Role | Eager |
| Architecture | `plan` | Utility | On-demand |
| Architecture | `specification-architect` | Role | On-demand |
| Architecture | `coach` | Role | On-demand |
| Architecture | `general-assistant` | Role | On-demand |
| Project | `project-initializer` | Utility | On-demand |
| Project | `vanehub-expert` | Utility | On-demand |
| Project | `skill-creator` | Utility | On-demand |
| Documentation | `md2word` | Utility | On-demand |
| Documentation | `pptx-craft` | Utility | On-demand |
| Documentation | `codewiki-api` | Utility | On-demand |
| Documentation | `api-doc-generation` | Role | Eager |
| Documentation | `readme-generation` | Role | Eager |
| Research | `deepresearch` | Utility | On-demand |
| Research | `image-analyzer` | Role | On-demand |
| Research | `version-bug-analysis` | Utility | On-demand |
| Specification | `sdd-design-story` | Utility | On-demand |
| Specification | `sdd-cloud-desktop-manager` | Utility | On-demand |
| Specification | `specification-architect-skill-creator` | Utility | On-demand |

This yields 13 Roles and 15 Utilities. Existing packages keep eager delivery for behavioral compatibility. New Roles are on-demand to protect prompt budgets. Utilities are never eagerly injected.

`code-reviewer` is an alias on `code-review`, not a second package. Exact canonical matching still takes precedence if a higher layer later defines a canonical `code-reviewer` Skill.

Alternatives considered:

- Preserve a third `Meta` type. Rejected because the effective runtime deliberately has only Role and Utility; generator Skills execute as Utilities.
- Make all first-party Roles eager. Rejected because thirteen instruction bodies would consume context before they are relevant.
- Rename the six existing ids for consistency. Rejected because bindings, usage, tombstones, Overlays, and user expectations depend on them.

### 3. Treat package purposes as bounded contracts

Each package has a one-sentence `purpose` and stable `purpose_key` in metadata. The purpose describes an outcome, not a broad personality. Trigger fixtures contain representative positive, ambiguous, and negative requests. Validation ensures ids and purpose keys are unique, aliases do not collide, required fixtures exist, and related packages declare `related_skills` plus a differentiation note.

Important boundaries include:

- `developer` implements; `code-explorer` only investigates; `code-review` evaluates existing changes.
- `plan` produces analysis and a plan; `general-assistant` coordinates a broad conversation; `coach` clarifies intent; `specification-architect` coaches specification decisions.
- `skill-creator` builds ordinary Skill packages; `specification-architect-skill-creator` builds a narrower team specification-coach package.
- `api-doc-generation` writes API documentation from code; `codewiki-api` uses an explicitly configured documentation integration.
- `version-bug-analysis` analyzes issue quality and root causes; it does not patch code automatically.

The duplicate-purpose validator is deterministic: it rejects duplicate ids, purpose keys, normalized aliases, identical trigger fixtures, and identical purpose/description fingerprints. Semantic overlap remains a human review responsibility supported by the checked-in differentiation notes; no model call is required during build.

Alternatives considered:

- Use embedding similarity during packaging. Rejected because builds must be deterministic and offline.
- Allow broad overlapping “expert” packages. Rejected because selection becomes unpredictable and evidence cannot be attributed reliably.

### 4. Keep `SKILL.md` under a strict progressive-disclosure budget

Each package uses the standard frontmatter plus catalog fields:

```yaml
id: code-explorer
name_key: skills.codeExplorer.name
description_key: skills.codeExplorer.description
category: development
version: 1.0.0
type: utility
delivery: on-demand
aliases: []
purpose_key: repository-exploration
related_skills: [developer, code-review]
requires:
  capabilities: [file-read, content-search, filename-search]
  integrations: []
  modalities: [text]
delegation:
  tools: [file-read, content-search, filename-search]
  max_rounds: 6
```

The instruction body hard limit is 2,000 Unicode characters excluding frontmatter. It contains purpose, when to use, non-goals, compact workflow, result contract, and links to detailed references. Reusable checklists, examples, schemas, templates, and domain material live in resources.

Catalog-specific limits are stricter than general imported-Skill limits:

```text
SKILL.md body              2,000 Unicode characters
text resource             128 KiB
binary asset              1 MiB
package total             4 MiB
resource entries          64
resource depth            4
logical path length       240 characters
```

Every resource must be reachable from metadata, `SKILL.md`, or another reachable validated document. Every reference must remain package-relative and logical. Resources are ordered and hashed deterministically.

Alternatives considered:

- Put complete workflows in `SKILL.md`. Rejected because Role loading and Utility child prompts would waste context on unused detail.
- Allow unrestricted package size because content is shipped locally. Rejected because application size, indexing, tool output, and review costs still need bounds.

### 5. Declare dependencies separately from delegated tools

`requires` describes what must exist for advertised behavior:

- `capabilities`: VaneHub runtime functions such as file operations, web research, document artifacts, presentation artifacts, image input, or Utility delegation.
- `integrations`: configured service contracts; never credentials or endpoints embedded in the Skill.
- `modalities`: text, image, or artifact input/output needs.

`delegation.tools` is a least-privilege child tool request and must be a subset of capabilities that map to delegatable tools. Availability resolves requirements through runtime feature and integration registries without starting a process, network request, account flow, or model generation.

Core examples:

- `code-explorer`, `plan`, `vanehub-expert`, and `version-bug-analysis` can use core read-only capabilities.
- `code-modification-mr`, `fix-vulnerability`, `project-initializer`, `skill-creator`, and specification-writing Utilities declare bounded mutating capabilities and remain subject to delegation and tool approvals.
- `md2word` and `pptx-craft` require corresponding artifact capabilities.
- `deepresearch` requires configured web-research capability.
- `image-analyzer` requires image input modality.
- `codewiki-api` and `sdd-cloud-desktop-manager` require explicitly configured integrations.

Unavailable packages remain discoverable, previewable, overrideable, and explainable. The UI does not present an execute action. A higher-layer override may change dependencies but must still pass runtime validation.

Alternatives considered:

- Let the Skill attempt a tool and discover failure. Rejected because availability would be misleading and could trigger account or network side effects.
- Encode provider credentials in package templates. Rejected because credentials belong behind existing Settings and native service boundaries.
- Treat dependencies as installation instructions only. Rejected because runtime needs machine-readable eligibility.

### 6. Keep all bundled scripts inert

The general package shape reserves `scripts/`, but this change does not execute any file from it. First-party packages should prefer references and templates. If a script example is essential, it must be UTF-8 documentation content, use a non-executable example suffix, be referenced explicitly, pass secret and injection scanning, and never appear in a delegation tool catalog.

The validator rejects executable extensions, executable signatures, shebang-bearing runnable files, package links, and hidden paths. It also rejects instructions that tell an Agent to bypass VaneHub approval, sandboxing, unified logging, service boundaries, or OpenSpec governance.

Alternatives considered:

- Ship working helper scripts early for convenience. Rejected because dynamic Skill tooling and sandbox lifecycle are a separate security change.
- Allow shell snippets without review constraints. Rejected because examples can become executable instructions when loaded by a model.

### 7. Generate catalog content in reviewable waves

Implementation authors packages by category rather than writing all 28 in one opaque change:

1. migrate and validate the six existing packages without changing purpose;
2. development and code-quality;
3. architecture and planning;
4. project and configuration;
5. document production;
6. research and analysis;
7. specification workflows and specialized generation.

Each wave adds package content, resources, translations, purpose relationships, dependency declarations, and fixtures together. A catalog snapshot test prevents accidental additions, removals, type changes, or delivery changes. Content review verifies that referenced VaneHub behavior matches current main specs and user documentation rather than copying obsolete examples.

No first-party package may include comparison history, source-product terminology, undocumented proprietary workflows, or identity that belongs to another product. `vanehub-expert` is authored from VaneHub's own specs, commands, adapters, settings, and troubleshooting boundaries.

Alternatives considered:

- Generate all Markdown once from an LLM and accept it as source. Rejected because identities, safety boundaries, links, and product facts require deterministic review.
- Create one OpenSpec change per package. Rejected because the catalog matrix, aliases, migration, and validator must land coherently.

### 8. Expand the System manifest without assigning new behavior silently

After the effective runtime migration, the six existing packages already have immutable System identities. This change adds 22 manifest entries. Reconciliation:

1. validates the complete packaged catalog;
2. preserves existing records, tombstones, enabled state, bindings, usage, and Overlays for six stable ids;
3. detects existing User or Project definitions using any new id and leaves them as higher-priority overrides;
4. adds new System definitions as enabled but unassigned;
5. records aliases without rewriting persisted canonical identity;
6. calculates dependency availability without triggering dependencies;
7. reports per-package results through unified logging.

New on-demand Roles and Utilities cannot affect an Agent until explicitly assigned. Existing Role bindings keep eager behavior. Restoring a first-party package clears deletion intent and reveals the System base or higher-layer winner; it never creates a mutable copy.

If a package is invalid, the rest of the catalog remains visible, but release packaging and CI fail. Runtime partial failure is only a defense for damaged installed resources, not permission to ship invalid content.

Alternatives considered:

- Auto-assign general Skills to OnePiece. Rejected because new instructions and delegated abilities require user intent.
- Reset all first-party state to catalog defaults. Rejected because upgrades must not erase user configuration or evolution history.

### 9. Reuse runtime-neutral management contracts

Rust Skill responses add first-party category, purpose key, dependency requirements and status, modality, validation summary, resource counts, alias list, and catalog-version metadata. Aggregate responses count one effective identity even when a higher layer shadows a System base.

`agent-service.ts` defines the TypeScript contract. `tauri-agent-client.ts` maps native data; `web-agent-client.ts` supplies a representative 28-entry catalog fixture or a compact behaviorally complete fixture plus exact aggregate contract tests. React components never read package files or invoke Tauri directly.

Settings adds composable filters and bounded summary cards rather than rendering 28 special cases. Rows remain compact; resources, dependencies, aliases, and base/effective information live in details. Localization keys provide names, descriptions, categories, reasons, and validation copy with English fallback; ids and resource paths are never localized.

Alternatives considered:

- Hard-code package names and categories in React. Rejected because Web parity, localization, overrides, and future catalog updates would drift.
- Render all 28 packages as separate dashboard statistics. Rejected because summary should remain useful as the catalog grows.

### 10. Make validation part of CI and packaging

The catalog validator is callable as a deterministic repository script and from Rust tests. It checks:

- exact catalog snapshot and unique stable identities;
- metadata schema, type, delivery, category, semantic version, aliases, purposes, relations, dependencies, and delegation subsets;
- body and package limits;
- resource reachability, links, paths, case collisions, UTF-8, media signatures, and hashes;
- secrets, authority overrides, executable content, unsafe examples, and governance bypass text;
- localization keys and English fallback;
- positive, ambiguous, and negative selection fixtures;
- unchanged behavioral snapshots for the existing six packages;
- reproducible manifest output.

`npm run contracts:check` or a dedicated catalog check invoked by it protects cross-language snapshots. Rust tests verify the generated manifest and runtime parser agree. Documentation checks validate Markdown links and CommonMark rules. Application packaging depends on successful catalog generation.

Alternatives considered:

- Rely on manual review only. Rejected because 28 packages and their resources will evolve frequently.
- Validate only at runtime. Rejected because invalid first-party content must not reach a release artifact.

## Risks / Trade-offs

- [Twenty-two new packages create a large review surface] → Author by category, require fixtures and deterministic validation, and keep primary instructions under 2,000 characters.
- [Packages promise unavailable behavior] → Require machine-readable dependencies and show unavailable reasons before assignment or execution.
- [Role and Utility purposes overlap] → Use unique purpose keys, related-skill differentiation, trigger fixtures, and aliases instead of near-duplicate packages.
- [Existing built-ins change behavior accidentally] → Preserve ids and eager delivery, add behavioral snapshots, and review content migration separately from new packages.
- [Catalog growth increases application size] → Enforce per-resource and per-package limits and reject orphaned assets.
- [Integration Skills expose credentials or trigger network calls] → Keep credentials behind native settings, make availability read-only, and prohibit embedded endpoints or account identifiers.
- [First-party instructions become stale as VaneHub evolves] → Link product packages to stable specs and commands, validate references, and update package versions with reviewed changes.
- [Aliases collide with higher-layer Skills] → Preserve canonical-id-first resolution and surface alias shadowing without rewriting user identities.
- [Utility packages are visible before delegation lands] → Show them as previewable but unavailable with a clear runtime dependency; never reinterpret them as Roles.

## Migration Plan

1. Complete the effective Skill runtime prerequisite and confirm six System packages migrate safely.
2. Add the catalog source tree, schema extensions, validator, deterministic generator, and exact 28-entry snapshot with placeholder-free minimal packages.
3. Move the six existing package content into the resource tree and verify byte/behavior compatibility, state preservation, and restore behavior.
4. Author and validate the 22 new packages in category waves with resources, dependencies, aliases, translations, relationships, and fixtures.
5. Add the generated manifest to application packaging and extend reconciliation for new System entries without automatic Agent assignment.
6. Extend dependency availability, management responses, unified diagnostics, and runtime tests.
7. Update shared frontend contracts, both adapters, filters, statistics, details, localization, component tests, and E2E coverage.
8. Run catalog, contracts, docs, frontend, Rust, packaging, and strict OpenSpec validation before enabling the expanded manifest.

Rollback returns catalog exposure to the prior six-entry manifest while leaving the 22 package resources and all additive state untouched. New-package assignments, tombstones, usage, Overlays, and higher-layer definitions remain preserved but dormant so re-enabling does not lose intent. The six existing ids retain their records and behavior. Rollback never deletes User or Project content and never rewrites first-party Overlay history.

