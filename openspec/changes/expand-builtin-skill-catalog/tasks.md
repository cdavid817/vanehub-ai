## 1. Prerequisite and Catalog Schema

- [ ] 1.1 Verify `establish-effective-skill-runtime` is implemented and its immutable System package, alias, dependency availability, logical resource, migration, usage, and Overlay association tests pass.
- [ ] 1.2 Add a failing exact-catalog snapshot test for the 28 canonical ids, 13 Role classifications, 15 Utility classifications, categories, delivery values, and `code-reviewer` alias.
- [ ] 1.3 Extend first-party metadata with localization keys, purpose key, related Skills, differentiation, dependencies, modalities, and delegation contract using validated typed models.
- [ ] 1.4 Implement the checked-in first-party catalog contract and reject missing, extra, duplicate, non-kebab-case, or misclassified packages.
- [ ] 1.5 Add validation tests for alias collisions, duplicate purpose keys, identical purpose fingerprints, missing relationship differentiation, invalid semantic versions, unknown categories, and invalid delegation subsets.

## 2. Package Tree and Deterministic Manifest

- [ ] 2.1 Create `src-tauri/resources/skills/` as the authoritative first-party source tree with one canonical directory per package.
- [ ] 2.2 Implement deterministic package discovery that scans only immediate canonical package directories and excludes hidden, linked, generated, and unsupported entries.
- [ ] 2.3 Implement manifest generation with sorted metadata, resource inventory, individual hashes, complete package hashes, dependency declarations, and fixture hashes.
- [ ] 2.4 Add reproducibility tests proving identical package trees generate byte-equivalent catalog ordering, resource inventories, and hashes.
- [ ] 2.5 Integrate the validated package tree and generated manifest with the immutable System provider and application packaging without a runtime network dependency.
- [ ] 2.6 Add installed-resource integrity tests for manifest version, missing resources, changed hashes, partial damage, and safe per-package unavailability.

## 3. Catalog Content Validator

- [ ] 3.1 Enforce the 2,000-character `SKILL.md` body limit, 128 KiB text resource limit, 1 MiB binary asset limit, 4 MiB package limit, 64-entry limit, depth four, and 240-character logical path limit.
- [ ] 3.2 Add safe link and reachability validation for missing targets, absolute paths, parent traversal, hidden components, case collisions, cycles, orphan resources, and cross-package references.
- [ ] 3.3 Add UTF-8, media type, magic signature, executable extension, shebang, link, and hidden-payload validation for first-party resources.
- [ ] 3.4 Add deterministic scans for credentials, private keys, account identifiers, prompt-authority overrides, unsafe placeholders, permission bypass, logging bypass, service-boundary bypass, and OpenSpec-governance bypass instructions.
- [ ] 3.5 Validate every required localization key and English fallback without localizing canonical ids, aliases, or resource paths.
- [ ] 3.6 Add positive, ambiguous, and negative selection-fixture schema and validation for every first-party package.
- [ ] 3.7 Expose one repository catalog-check command and include it in `npm run contracts:check` or its invoked validation chain.
- [ ] 3.8 Add Rust tests proving runtime parsing and generated manifest parsing agree with repository validation.

## 4. Existing Six Package Migration

- [ ] 4.1 Move `tdd-discipline` into the package tree with explicit Role/Eager metadata, existing purpose preservation, concise instructions, references, localization, and fixtures.
- [ ] 4.2 Move `code-review` into the package tree with explicit Role/Eager metadata, `code-reviewer` alias, existing purpose preservation, references, localization, and fixtures.
- [ ] 4.3 Move `code-security-scan` into the package tree with explicit Role/Eager metadata, existing purpose preservation, safe security examples, localization, and fixtures.
- [ ] 4.4 Move `api-doc-generation` into the package tree with explicit Role/Eager metadata, existing purpose preservation, templates, localization, and fixtures.
- [ ] 4.5 Move `unit-test-generation` into the package tree with explicit Role/Eager metadata, existing purpose preservation, references, localization, and fixtures.
- [ ] 4.6 Move `readme-generation` into the package tree with explicit Role/Eager metadata, existing purpose preservation, templates, localization, and fixtures.
- [ ] 4.7 Add behavioral snapshots proving the six existing ids, eager delivery, established outcomes, bindings, enabled state, deletion intent, usage, aliases, overrides, and Overlays survive migration.

## 5. Development and Code-Quality Packages

- [ ] 5.1 Author `developer` as an on-demand Role with bounded implementation workflow, test discipline, permission awareness, references, localization, relationships, and fixtures.
- [ ] 5.2 Author `code-explorer` as a read-only Utility with file-read and search capabilities, structured repository findings, references, localization, relationships, and fixtures.
- [ ] 5.3 Author `code-modification-mr` as a Utility with explicit mutation and version-control capability requirements, approval-aware workflow, references, templates, localization, and fixtures.
- [ ] 5.4 Author `fix-vulnerability` as a Utility with evidence-first remediation, security validation, bounded mutation capabilities, no automatic remote publication, references, localization, and fixtures.
- [ ] 5.5 Author `mcp-builder` as an on-demand Role with protocol guidance, implementation references, permission boundaries, localization, relationships, and fixtures.
- [ ] 5.6 Author `plugin-creator` as an on-demand Role limited to VaneHub-compatible plugin structures and current contribution boundaries, with templates, localization, relationships, and fixtures.
- [ ] 5.7 Validate differentiation among developer, explorer, reviewer, modification, vulnerability, TDD, test generation, and security packages.

## 6. Architecture and Planning Packages

- [ ] 6.1 Author `plan` as a read-only Utility producing bounded analysis, root-cause findings, dependencies, risks, acceptance criteria, and implementation plans without executing them.
- [ ] 6.2 Author `specification-architect` as an on-demand coaching Role with structured questions, decision records, non-goals, references, localization, relationships, and fixtures.
- [ ] 6.3 Author `coach` as an on-demand Role for goal clarification and mode selection without impersonating specialist Skills, with localization, relationships, and fixtures.
- [ ] 6.4 Author `general-assistant` as an on-demand Role for general coordination and explicit delegation recommendations, with bounded scope, localization, relationships, and fixtures.
- [ ] 6.5 Validate positive, ambiguous, and negative selection boundaries among plan, specification architect, coach, general assistant, and developer.

## 7. Project and Configuration Packages

- [ ] 7.1 Author `project-initializer` as a Utility with explicit read/write requirements, safe AGENTS and project-knowledge workflows, templates, localization, relationships, and fixtures.
- [ ] 7.2 Author `vanehub-expert` as a Utility grounded only in current VaneHub specs, settings, adapters, commands, and troubleshooting boundaries, with no undocumented configuration claims.
- [ ] 7.3 Author `skill-creator` as a Utility that creates or improves valid Role and Utility packages using the first-party schema, validator, progressive disclosure, templates, localization, and fixtures.
- [ ] 7.4 Validate that product configuration, project initialization, and Skill creation requests select distinct packages and never embed credentials or user-specific paths.

## 8. Document-Production Packages

- [ ] 8.1 Author `md2word` as a Utility requiring the Word artifact capability, with bounded conversion contract, Chinese typography references, safe presets, templates, localization, and fixtures.
- [ ] 8.2 Author `pptx-craft` as a Utility requiring the presentation artifact capability, with planning/design pipeline, templates, asset rules, localization, and fixtures.
- [ ] 8.3 Author `codewiki-api` as a Utility requiring an explicitly configured documentation-service integration, with credential-free request guidance, bounded results, localization, and fixtures.
- [ ] 8.4 Validate that `api-doc-generation`, `readme-generation`, `md2word`, `pptx-craft`, and `codewiki-api` have non-overlapping artifact and integration purposes.
- [ ] 8.5 Add dependency-unavailable fixtures proving document packages remain visible but do not claim output or trigger integrations when required capabilities are absent.

## 9. Research and Analysis Packages

- [ ] 9.1 Author `deepresearch` as a Utility requiring configured web-research capability, with bounded depth levels, source verification, citation contract, privacy rules, localization, and fixtures.
- [ ] 9.2 Author `image-analyzer` as an on-demand Role requiring image modality, limited to software-engineering diagrams, screenshots, logs, and interfaces, with localization and fixtures.
- [ ] 9.3 Author `version-bug-analysis` as a read-only Utility for issue quality, severity, type, root-cause, and bounded aggregate analysis, with templates, localization, relationships, and fixtures.
- [ ] 9.4 Add modality and dependency fixtures proving unavailable research or image inputs are reported rather than fabricated.

## 10. Specification Workflow and Specialized Packages

- [ ] 10.1 Author `sdd-design-story` as a Utility for local proposal, delta-spec, design, and task artifacts with state, re-entry, OpenSpec validation, templates, localization, and fixtures.
- [ ] 10.2 Author `sdd-cloud-desktop-manager` as a Utility requiring an explicitly configured compatible workspace integration, with bounded item, upload, and review operations and no embedded account details.
- [ ] 10.3 Author `specification-architect-skill-creator` as a Utility for team-specific specification-coach packages, differentiated from general `skill-creator`, with templates, localization, and fixtures.
- [ ] 10.4 Validate boundaries among local specification design, configured workspace integration, specification coaching, and specification-coach generation.

## 11. Dependency Availability and Reconciliation

- [ ] 11.1 Add a read-only dependency resolver for runtime capabilities, configured integrations, and modalities that performs no process launch, network request, installation, account flow, or model call.
- [ ] 11.2 Map all 28 packages to core, optional capability, integration, and modality requirements and validate delegation tool declarations are permitted subsets.
- [ ] 11.3 Add availability tests for core-only, delegation-missing, artifact-missing, web-research-missing, image-missing, and integration-unconfigured packages.
- [ ] 11.4 Extend System catalog reconciliation from six to 28 entries while preserving existing and pre-existing higher-layer definitions for new canonical ids.
- [ ] 11.5 Add upgrade tests for idempotency, partial resource damage, new-package tombstones, existing overrides, aliases, no automatic assignment, restoration, and per-package unified diagnostics.
- [ ] 11.6 Keep the 22 new packages enabled but unassigned by default and prove they do not affect Agent prompts or delegated tools until explicit assignment.

## 12. Frontend Contracts and Adapters

- [ ] 12.1 Extend shared TypeScript Skill contracts and `agent-service.ts` with first-party category, purpose, dependency, modality, validation, resource, alias, and catalog-summary models without `any`.
- [ ] 12.2 Update `tauri-agent-client.ts` mappings for expanded catalog overview, detail, dependency status, aliases, resources, and summaries while keeping all native invocation inside the adapter.
- [ ] 12.3 Update `web-agent-client.ts` with representative Role, Utility, overridden, assigned, unassigned, dependency-unavailable, modality-unavailable, and alias cases plus exact 28-entry summary fixtures.
- [ ] 12.4 Add desktop mapping and Web/mock adapter-parity tests for canonical ids, counts, categories, classification, dependencies, overrides, and unavailable reasons.

## 13. Skills Settings Experience

- [ ] 13.1 Add composable first-party, Role/Utility, delivery, category, dependency, assignment, and override filters while preserving search and selected stable Agent context.
- [ ] 13.2 Add bounded catalog statistics for 28 first-party packages, 13 Roles, 15 Utilities, categories, availability, assignments, dependencies, and overrides.
- [ ] 13.3 Update Skill rows and details with localized first-party metadata, canonical id, aliases, dependencies, modalities, resources, immutable base, and effective override state.
- [ ] 13.4 Add actionable unavailable explanations without triggering installation, account access, process launch, or network calls.
- [ ] 13.5 Add localized names, descriptions, categories, dependency reasons, validation messages, filter labels, summaries, and English fallbacks for all 28 packages.
- [ ] 13.6 Add component and interaction tests for counts, combined filters, alias display, dependency status, one-row-per-effective-id behavior, responsive details, accessibility, and Web parity.
- [ ] 13.7 Keep new production TS/TSX modules below 300 lines by separating catalog filters, statistics, metadata, dependencies, and resource details.
- [ ] 13.8 Run `npx playwright test` and resolve expanded-catalog regressions in filtering, assignment, preview, unavailable state, and responsive behavior.

## 14. Verification and Documentation

- [ ] 14.1 Document the exact first-party catalog, Role/Utility semantics, dependencies, progressive disclosure, assignment defaults, aliases, and content contribution workflow without external-product comparisons.
- [ ] 14.2 Run the first-party catalog validator and reproducible-manifest check.
- [ ] 14.3 Run `npm run docs:check` and `npm run contracts:check`.
- [ ] 14.4 Run `npm run lint:ci`.
- [ ] 14.5 Run `npm run test` and `npm run test:coverage`.
- [ ] 14.6 Run `npm run coverage:policy:test` and `npm run version:unit:test`.
- [ ] 14.7 Run `npm run build`.
- [ ] 14.8 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [ ] 14.9 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [ ] 14.10 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 14.11 Run `openspec validate expand-builtin-skill-catalog --strict` and `openspec validate --specs --strict`.

