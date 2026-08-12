## Why

VaneHub AI's six built-in Skills cover only a narrow set of code-quality and documentation tasks, leaving the new Role, Utility, progressive-disclosure, and delegation infrastructure without a useful first-party catalog. A curated VaneHub-native set is needed so users can immediately apply the Skill runtime across development, planning, initialization, research, document production, and specification workflows.

## What Changes

- Expand the immutable System catalog from 6 to 28 first-party Skill packages while retaining all existing canonical ids and user state.
- Add 6 development and code-quality Skills: `developer`, `code-explorer`, `code-modification-mr`, `fix-vulnerability`, `mcp-builder`, and `plugin-creator`; add alias `code-reviewer` to the existing `code-review` package.
- Add 4 architecture and planning Skills: `plan`, `specification-architect`, `coach`, and `general-assistant`.
- Add 3 project and product-configuration Skills: `project-initializer`, `vanehub-expert`, and `skill-creator`.
- Add 3 document-production Skills: `md2word`, `pptx-craft`, and `codewiki-api`.
- Add 2 research and analysis Skills: `deepresearch` and `image-analyzer`.
- Add 2 specification-driven workflow Skills: `sdd-design-story` and `sdd-cloud-desktop-manager`.
- Add 2 specialized generation and analysis Skills: `specification-architect-skill-creator` and `version-bug-analysis`.
- Retain the 6 current Skills as first-party System packages: `tdd-discipline`, `code-review`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`.
- Classify every package explicitly as Role or Utility, set eager or on-demand delivery, declare aliases and Utility delegation capabilities, and provide concise metadata suitable for discovery.
- Standardize every package as `SKILL.md` plus only required `references/`, `templates/`, `assets/`, and non-executing `scripts/` resources, using progressive disclosure rather than oversized instruction bodies.
- Add manifest, frontmatter, link, resource, safety, token-budget, duplicate-purpose, and behavioral fixture validation for all first-party packages.
- Preserve higher-layer overrides, aliases, enabled state, deletion intent, Agent assignments, usage, Overlay state, and immutable System-package migration when the catalog expands.
- Extend Skills settings with first-party categories, Role/Utility filters, alias visibility, resource summaries, compatibility and dependency status, and catalog-level statistics through existing service adapters.
- Exclude arbitrary executable tools, bundled credentials, provider-specific secrets, automatic external account access, and undocumented product-specific assumptions from first-party packages.

## Capabilities

### New Capabilities

- `builtin-skill-catalog`: Defines the exact 28-package first-party catalog, classification, content and resource quality, safety, progressive disclosure, dependency availability, aliases, and compatibility guarantees.

### Modified Capabilities

- `skill-management`: Expands built-in reconciliation and restore behavior from six to 28 immutable System packages while preserving existing and higher-layer user state.
- `settings-skill-management-ui`: Adds catalog categories, Role/Utility and first-party filtering, alias and dependency presentation, resource summaries, and expanded catalog statistics.

## Impact

- Depends on `establish-effective-skill-runtime`; Utility execution additionally depends on `add-delegated-utility-skills`, while packages remain visible with safe unavailable reasons until dependencies are ready.
- Affects System package resources and manifest generation, built-in reconciliation tests, Skill metadata validation, aliases, resource indexing, usage and Overlay association, frontend contracts, both runtime adapters, Skills settings, localization, and documentation.
- Adds substantial Markdown and supporting content but no new frontend state library, direct React-to-Tauri calls, remote registry client, dynamic script execution, credential storage, or alternative package manager.
