# OpenSpec Archive Governance

## Scope and Location

The online decision history for completed changes lives at:

`openspec/changes/archive/YYYY-MM-DD-<change-name>/`

Each archive preserves the complete OpenSpec change directory, including proposal, design, tasks, verification evidence when present, and delta specs. `openspec/specs/` remains the only source of truth for current requirements.

Do not replace archive directories with zip or tar files. Git preserves compact storage, diffs, searchability, and traceable history more effectively for these text artifacts.

## Archive Admission

For a normal implementation change:

1. Complete every task in `tasks.md`.
2. Record implementation verification when code changed.
3. Run `openspec validate <change-name> --strict` successfully.
4. Run `openspec archive <change-name>` and accept its main-spec update.
5. Regenerate the archive index:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1
   ```

6. Commit the updated main specs, archived directory, and generated index together.

`--no-validate` is not part of this workflow. `--skip-specs` is permitted only for a completed change that has no main-spec impact, such as repository tooling or documentation work.

Archive artifacts are historical records. Do not edit their proposal, design, tasks, or delta specs after archiving. Capture corrections or follow-up work in a new change.

## Index Maintenance

`openspec/changes/archive/README.md` and `openspec/changes/archive/archive-index.json` are generated. Run `scripts/Update-OpenSpecArchiveIndex.ps1` immediately after each archive operation and after any completed cold migration. Do not edit either generated file manually.

Use `archive-index.json` as the first lookup surface for archive date, change name, artifacts, and affected capabilities. Read a specific archived Markdown file only after the catalog identifies the relevant change.

## Retention and Cold Archive

Review online archives at least every six months. Archive directories remain online unless a maintainer has a reason to reduce the active repository footprint.

Before moving an archive to cold storage:

1. Verify that the full directory exists in a dedicated Git archive repository or on an immutable Git branch or tag.
2. Record the source path, destination repository, immutable commit or tag, verification date, and reviewer in `openspec/archive-cold-migrations.md`.
3. Regenerate the online archive index.
4. Only then remove the verified online copy in the same reviewed change.

Never perform age-based automatic deletion. A cold migration preserves decision history; it is not a deletion policy.
