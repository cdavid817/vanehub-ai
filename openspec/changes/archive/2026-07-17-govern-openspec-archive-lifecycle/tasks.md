## 1. Governance Documentation

- [x] 1.1 Add the OpenSpec archive lifecycle policy and cold-migration registry under `openspec/` with canonical path, admission checks, index refresh, and cold-migration rules.
- [x] 1.2 Correct the archive location and lifecycle guidance in `AGENTS.md`.

## 2. Archive Discoverability

- [x] 2.1 Add a PowerShell generator that derives the online archive index from date-prefixed archive directories.
- [x] 2.2 Generate `openspec/changes/archive/README.md` from the current archive contents.

## 3. Verification

- [x] 3.1 Run the archive index generator and confirm the generated entries match the archive directories.
- [x] 3.2 Run `openspec validate govern-openspec-archive-lifecycle --strict` and `openspec validate --specs --strict`.
