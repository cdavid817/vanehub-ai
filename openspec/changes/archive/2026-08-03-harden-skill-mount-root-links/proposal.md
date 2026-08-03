## Why

Skill binding currently assumes each CLI Skill mount root is a normal directory. On Windows, a legacy whole-directory symlink or junction—especially a broken link left by another Skill manager—can make binding fail with raw `os error 183`, while a live external link risks writing VaneHub-managed entries into another tool's source directory.

## What Changes

- Preflight the configured CLI Skill mount root before creating or repairing a per-Skill managed link.
- Continue normally for absent roots that can be created and existing normal directories.
- Reject broken directory links and live externally managed directory links without deleting, replacing, following, or writing through them.
- Return concise, actionable Agent-specific errors instead of raw Windows filesystem messages, while leaving the Skill assignment unchanged on failure.
- Include the safe stable Agent id in unified Skill binding diagnostics without persisting raw home-directory paths.
- Keep Web/mock behavior deterministic and non-destructive while preserving the same assignment-success and assignment-failure contract.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `skill-management`: Define safe mount-root preflight behavior for normal directories, live external directory links, and broken directory links.
- `settings-skill-management-ui`: Present failed Agent assignment as an actionable row-level error without falsely moving the Skill into Assigned.

## Impact

- Affects the shared Skill domain/application error contract, the Rust managed filesystem adapter, Skill operation logging, and Settings Skill interaction tests.
- Uses the existing React → `AgentService` → Tauri/Web adapter boundary; React will not inspect the filesystem or call Tauri directly.
- Does not automatically migrate, unlink, delete, or overwrite existing CLI Skill root links and introduces no SQLite schema change or dependency.
- Desktop runtime performs real mount-root inspection; Web/mock simulates equivalent outcomes where filesystem-only behavior is represented.
