## Context

The repository stores completed OpenSpec changes at `openspec/changes/archive/YYYY-MM-DD-<change-name>/`. The directory contains complete Markdown planning artifacts and is tracked by Git, but `AGENTS.md` references a non-existent `openspec/archive/` location. There is also no authoritative archive policy or index for discovering completed changes.

This change is repository governance only. It does not cross the React service boundary, invoke Tauri commands, alter the Web adapter, or affect the desktop runtime.

## Goals / Non-Goals

**Goals:**

- Make the archive location and archive prerequisites unambiguous for contributors and agents.
- Keep complete, diffable Markdown artifacts in the repository's online archive.
- Generate a small index from the archive directory so history remains discoverable without duplicating artifact content.
- Define a conservative cold-archive process that preserves Git history and does not replace repository artifacts with opaque zip files.

**Non-Goals:**

- Reformat, deduplicate, or delete existing archived changes.
- Add automated retention deletion, cloud storage, Git LFS, or a new runtime dependency.
- Change application behavior, frontend adapters, native commands, or product log retention.

## Decisions

### Retain complete Markdown artifacts in the online archive

`openspec archive <change-name>` remains the sole normal archival operation. It updates the main specs and moves the complete change directory under `openspec/changes/archive/`. Git already stores repository history efficiently and preserves reviewable diffs, so individual archive directories will not be converted to zip or tar files.

Alternative considered: replace completed directories with compressed files. Rejected because it prevents direct repository search, review, and spec-history tracing while offering negligible savings for the current small text-only archive.

### Add a generated archive index

Add a PowerShell generator under `scripts/` that enumerates date-prefixed archive directories and produces `openspec/changes/archive/README.md`. The generated index contains the archive path and each archived change name, allowing maintainers to review changes and detect an incomplete archive without duplicating proposal or spec content. A separate `openspec/archive-cold-migrations.md` registry records immutable Git destinations after an online archive is moved.

Alternative considered: maintain the README manually. Rejected because the index will inevitably drift during routine archive operations.

### Use a two-tier retention model

The repository retains the current online archive in Git. At a six-month review interval, maintainers may move older, complete archive directories to a dedicated Git archive repository or an immutable Git branch/tag while retaining an index record and the commit reference in this repository. The transfer must be verified before removing online copies.

Alternative considered: time-based automatic deletion. Rejected because OpenSpec archives are decision records and deleting them without a verified historical reference is irreversible.

### Make validation and archive evidence mandatory

Normal archival requires completed tasks, successful strict OpenSpec validation, and an implementation verification record when code changed. `--skip-specs` is limited to changes with no main-spec impact. `--no-validate` is prohibited in the normal workflow.

## Risks / Trade-offs

- [Generated index is not refreshed] -> The policy requires running the generator immediately after `openspec archive`; the generator derives all entries from disk and can repair drift.
- [Cold archive loses traceability] -> Move only complete directories after verifying the destination Git commit/tag, and keep the source path and destination reference in the index.
- [Archive volume grows] -> Review every six months before migration; do not introduce automatic deletion based on directory age.

## Migration Plan

1. Add the governance specification, policy, cold-migration registry, index generator, and generated index.
2. Correct the archive path documented in `AGENTS.md`.
3. Generate the index from the existing archive directories and verify it matches the directory list.
4. For future completed changes, validate, archive with the OpenSpec CLI, regenerate the index, and commit the resulting artifacts together.

Rollback is limited to reverting the governance documentation, generator, and index; no existing archive data is altered by this change.

## Open Questions

- The destination repository or immutable Git branch for six-month cold archives will be selected when the first review is due.
