## Context

VaneHub stores each managed Skill under the selected scope and mounts it into a CLI-specific relative directory such as `.claude/skills` or `.codex/skills`. The filesystem adapter currently calls `create_dir_all` on that mount root before creating a per-Skill symlink or Windows junction. A legacy whole-directory link can therefore either fail with an opaque platform error when broken or redirect writes into another manager's source directory when live.

The binding transaction already rolls back SQLite and filesystem changes on failure. The missing boundary is an explicit mount-root ownership check before the transaction stages or creates any target.

## Goals / Non-Goals

**Goals:**

- Accept normal existing directories and safely create absent normal directory paths.
- Detect directory symlinks, junctions, and other reparse-point ancestors between the canonical scope root and configured mount root.
- Distinguish live externally managed links from broken or unavailable links in user-facing errors.
- Preserve the Skill source, current Agent assignments, external links, and external targets on failure.
- Record the stable Agent id in unified binding diagnostics without logging raw home paths.
- Keep React and runtime adapters unchanged unless presentation needs a focused error helper.

**Non-Goals:**

- Automatically unlink, migrate, repair, or replace an external CLI Skill root.
- Follow a live external root and create VaneHub links inside another repository.
- Add a mount-root migration wizard, database schema, or hard-coded provider-specific UI.
- Treat an externally linked root as evidence that every Skill in that target is assigned to VaneHub.

## Decisions

### Decision: Treat linked mount-root components as externally owned

Before `create_dir_all` or target staging, the native filesystem adapter walks existing components of the relative mount root below the canonical scope root using non-following metadata. A Unix symlink or Windows reparse point is rejected. If resolving the component reaches a directory, the error classifies it as an external directory link; otherwise it classifies it as broken or unavailable.

Rationale: following the link would make VaneHub write into storage owned by Clowder, CatCafe, or another tool. Automatically replacing it would be destructive. Checking every component also covers a linked `.claude` ancestor rather than only `.claude/skills`.

Alternative considered: accept live links and only reject broken links. This restores binding for some machines but silently crosses the configured scope boundary and makes ownership ambiguous.

### Decision: Keep failure atomic and assignment intent unchanged

Mount-root preflight runs before any target is created, backed up, or linked. Binding continues through the existing filesystem/SQLite transaction, so a rejected root does not create a `skill_agent_bindings` row and does not alter another Agent binding.

Rationale: the UI must continue showing the Skill as Available after the failed assignment instead of claiming a binding that cannot be realized.

### Decision: Return actionable stable-Agent errors

The domain/application error distinguishes external, broken, and non-directory mount roots and includes only the stable Agent id. The row-level mutation error remains attached to the affected Skill. Messages explain that the whole-directory link must be migrated or repaired before assignment, rather than exposing a raw OS error or private absolute path.

Rationale: the stable Agent id is safe and sufficient to identify the affected CLI. Absolute home and external target paths remain out of logs and frontend errors.

### Decision: Add Agent context to existing unified logs

CLI bind and unbind operations add `agentId` to the existing Skill log context. They continue using the unified logging adapter and existing error/info levels. No feature-local log is introduced.

Rationale: the current error evidence identifies the Skill but cannot distinguish which CLI operation failed.

### Decision: Preserve Web/mock determinism

The Web adapter has no host filesystem and continues to simulate a normal writable mount root. Its granular binding behavior remains Agent-specific and atomic. Native-only link classification is covered in Rust filesystem/application tests, while shared Settings tests cover row-level failure presentation.

## Risks / Trade-offs

- [Risk] A user intentionally configured a safe junction and expects VaneHub to write through it. → Prefer explicit safety; a future migration/ownership feature can opt in with reviewable policy.
- [Risk] Windows junction detection differs from symbolic-link detection. → Check both `FileType::is_symlink` and the Windows reparse-point file attribute, with targeted Windows-compatible tests.
- [Risk] A mount-root ancestor changes between preflight and link creation. → Keep preflight immediately before directory creation inside the serialized filesystem transaction; existing rollback remains the final safety net.
- [Risk] The actionable error is still a backend-provided string. → Keep it concise and stable for this change; structured localized error codes can be introduced separately across the command boundary.

## Migration Plan

No automatic data or filesystem migration runs. Existing normal roots keep working. Existing linked roots become explicit, non-destructive assignment failures. Users can repair a broken link or intentionally convert an externally managed whole-directory link outside this change, then retry the same assignment.

Rollback restores the previous preflight behavior and requires no data rollback because rejected operations do not persist binding changes.

## Open Questions

None. Automatic root-link conversion remains a separate destructive workflow requiring preview and confirmation.
