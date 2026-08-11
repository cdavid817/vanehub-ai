## Why

VaneHub AI currently treats every Skill as a mutable Global or Workspace directory and injects every bound API Skill eagerly into the system prompt. That model cannot safely support immutable system packages, on-demand role activation, delegated Utility metadata, deterministic layer precedence, or progressive resource loading without breaking existing bindings and user-edited built-ins.

## What Changes

- Introduce separate Skill type, delivery, layer, origin, trust, and availability metadata instead of overloading scope or source.
- Resolve an effective Skill catalog using `Project > User > Registry > System` precedence, canonical IDs, aliases, deterministic shadowing, and current binding context.
- Package built-in Skills as immutable System packages and migrate the six currently materialized built-ins without overwriting user edits or resurrecting intentional deletions.
- Add fixed-schema `list_skills`, `load_skill`, and `read_skill_resource` tools for native API agents, including bounded inline content, logical resource URIs, resource indexes, traversal protection, and usage tracking.
- Preserve existing eager API-agent behavior for legacy Skills while allowing explicit Role Skills to use on-demand delivery; recognize Utility metadata but fail closed until delegated execution is implemented by a later change.
- Replace direct editing of immutable System Skill content with enablement, binding, preview, layer, shadowing, and compatibility presentation. Overlay-based customization is intentionally deferred to a separate change.
- Keep desktop and Web/mock service contracts aligned and preserve the frontend service boundary.
- **BREAKING**: Built-in Skills cease to use mutable Global files as their authoritative source. Existing unchanged built-in files migrate to immutable System packages; modified files are preserved as higher-priority User overrides.

## Capabilities

### New Capabilities

- `effective-skill-runtime`: Defines Skill classification, four-layer effective resolution, aliases, immutable System packages, bounded loading/resource tools, compatibility states, and usage tracking.

### Modified Capabilities

- `skill-management`: Replaces mutable built-in seeding as the authoritative model with System packages and safe migration, and extends lifecycle responses with effective-layer and compatibility information.
- `agent-skill-injection`: Adds delivery-aware prompt assembly so legacy eager behavior is preserved while on-demand Role and unavailable Utility Skills are excluded from eager injection.
- `agent-tool-execution`: Adds fixed-schema Skill discovery, loading, and resource-reading tools with bounded, read-only execution semantics.
- `agent-chat-configuration`: Keeps Skill discovery and reading available in Plan mode while preserving the mode's prohibition on mutating tools.
- `settings-skill-management-ui`: Presents Skill type, delivery, layer, origin, shadowing, resource summaries, and unavailable reasons, and prevents direct mutation of immutable System package content.

## Impact

- Native Skill domain, filesystem/package loading, SQLite migrations, API-agent tool catalog, prompt assembly, usage persistence, unified diagnostic integration, and startup reconciliation are affected.
- Frontend Skill contracts, `agent-service.ts`, Tauri and Web/mock adapters, Settings Skills components, and localization resources are affected.
- Existing Global/Workspace records, built-in tombstones, bindings, enabled state, and user-modified built-in files require an idempotent migration.
- No arbitrary Skill script execution, remote registry network client, Overlay mutation, Utility delegation, Curator, or self-evolution behavior is introduced by this change.
