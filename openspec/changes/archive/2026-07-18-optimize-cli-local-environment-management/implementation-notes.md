# CLI Local Environment Management: Implementation and Optimization Notes

## First-Version Implementation

- Scope is limited to Claude Code, Codex CLI, Gemini CLI, and OpenCode using their existing stable agent ids.
- Desktop detection enumerates PATH results plus a bounded catalog of common local install locations, deduplicates executable targets, and probes each candidate with a timeout.
- Cached `CliToolStatus` remains the single frontend service model. It includes installation distribution, active entry, source, environment, runnable state, and conflict state.
- The active installation is the first PATH-resolved candidate. A discovered executable whose version command fails is represented as installed but broken.
- Remote stable-version lookup remains npm-backed and can fail independently from local detection.
- Automatic versioned package mutation is allowed only for a missing tool or an active installation positively classified as npm-managed. Other sources receive a safe manual/source-native guidance state.
- Multiple installations require explicit confirmation before any eligible mutation. The backend recalculates the plan and never accepts command text from React.
- Global CLI package mutations are serialized. Detection, navigation, log expansion, and cached reads remain responsive.
- CLI Management owns detailed lifecycle controls. About displays only a shared environment summary and navigation affordance.
- Tauri and Web adapters preserve one contract; Web reports native inspection as unsupported and does not fake host installations.

## Known First-Version Limitations

- WSL installations and distro-specific shell selection are not detected.
- Source classification is based on bounded path patterns and can label unfamiliar layouts as `unknown`.
- WinGet, Homebrew, Volta, fnm, nvm, pnpm, bun, and vendor-native update commands are not automatically executed.
- The known-path catalog does not attempt recursive disk discovery or system-wide package database inspection.
- Repair actions for broken installations are guidance-only except where an existing safe npm operation applies.
- Remote version metadata is npm-centric; non-npm release channels can differ from the displayed latest stable npm version.

## Prioritized Optimization Paths

1. Add source-native lifecycle providers with explicit capability metadata, command previews, elevation handling, cancellation, and rollback guidance.
2. Add WSL distro discovery, per-tool distro binding, shell selection, and Linux-side probing behind the same normalized contract.
3. Split local and remote refresh scheduling so cached local health can refresh quickly while registry metadata uses a longer TTL.
4. Add filesystem watchers or PATH/config change signals to invalidate only affected CLI cache entries.
5. Improve source detection through package-manager metadata queries rather than path heuristics alone.
6. Add repair plans for broken platform binaries, unsupported Node versions, stale shims, and orphaned launchers.
7. Add richer install provenance history and before/after operation snapshots for support diagnostics.
8. Extend the catalog only through backend-owned definitions and contract fixtures, without display-name branching in React.

## Extension Contract

- New discovery sources belong in isolated Rust helpers and must return the existing normalized installation record.
- New lifecycle providers must be selected from backend-owned metadata, validate stable ids and versions, use explicit process arguments, and write through unified logging.
- Any new user-visible state must be added to both zh-CN and en resources and verified in both registered visual styles.
- Any new desktop capability must retain an honest Web adapter response and may not introduce direct Tauri calls in React components.

