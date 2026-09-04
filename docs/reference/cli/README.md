# CLI reference

The reference material for the CLI Agents VaneHub AI drives. Know which file is which before editing:

| File | Type | Canonical source | How to update |
| --- | --- | --- | --- |
| [parameter-matrix.md](parameter-matrix.md) | **Generated — never edit by hand** | `src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json` | Change the catalog, then run `npm run docs:matrix:generate` |
| [maintenance.md](maintenance.md) | Authored runbook | The parameter registry workflow itself | Edit directly when the workflow changes |
| [builtin-cli-reference.md](builtin-cli-reference.md) | Authored, **point-in-time audit** | Upstream `--help` output and official vendor documentation, audited 2026-08 | Re-audit against each CLI's current `--help` and official docs before relying on any flag; the file's own timeliness warning applies |

The point-in-time audit is a working baseline for the PTY adapter layer, not a permanent specification: coding CLIs add, rename, and remove flags monthly, so the installed CLI's own `--help` and official documentation always win over this file. In-app behavior (which parameters VaneHub actually renders and projects) is governed by the generated matrix and the registry, never by the audit document.
