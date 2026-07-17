## Context

The generated archive README is useful for browsing but contains only directory names. Finding a completed change by capability currently requires scanning archived proposals or specs repeatedly. The archive generator already enumerates archive directories, so it can build a compact query surface at generation time.

## Goals / Non-Goals

**Goals:**

- Produce a deterministic `archive-index.json` for direct programmatic and text queries.
- Include stable archive metadata: date, change name, relative path, available artifacts, and affected capabilities.
- Keep the README as the human-readable companion and document catalog-first lookup.

**Non-Goals:**

- Build a search service, database, UI, or runtime dependency.
- Extract arbitrary prose, requirements, or confidential content from planning artifacts.
- Modify archived source artifacts.

## Decisions

### Index structural metadata only

The generator derives capabilities from `specs/<capability>/spec.md` paths and derives artifact names from top-level known Markdown files. This avoids full-content parsing and preserves a deterministic mapping from archive layout to catalog fields.

Alternative considered: index proposal and design text for full-text search. Rejected because it reintroduces the expensive, noisy scan the catalog is intended to avoid and would make indexing sensitive to prose changes.

### Commit JSON beside the README

`openspec/changes/archive/archive-index.json` is checked in and regenerated together with the README after an archive operation. Consumers can filter this file with PowerShell or `rg` without traversing archived change directories.

Alternative considered: build the index at query time. Rejected because repeated queries would repeat archive traversal.

## Risks / Trade-offs

- [Catalog becomes stale] -> The existing post-archive generator step creates both index files together.
- [Historic archives use nonstandard layouts] -> Empty artifact or capability arrays remain valid and truthfully represent the on-disk layout.

## Migration Plan

1. Extend the generator to write the JSON catalog and enrich the README with indexed capabilities.
2. Regenerate both outputs for all existing online archives.
3. Verify every archive directory has exactly one catalog entry and that catalog capabilities match delta spec paths.
