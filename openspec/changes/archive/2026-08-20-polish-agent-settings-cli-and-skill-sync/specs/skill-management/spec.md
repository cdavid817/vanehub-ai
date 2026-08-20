## ADDED Requirements

### Requirement: Convergent built-in Skill drift synchronization
Synchronizing drift for an immutable built-in Skill SHALL reconcile supported legacy registry hashes and derived-cache revisions to the currently shipped package, and a successful synchronization SHALL leave that Skill absent from a fresh drift detection result.

#### Scenario: Upgrade with legacy built-in snapshots
- **WHEN** an existing installation contains legacy registry or cache state for a built-in Skill whose shipped package is authoritative
- **AND** drift detection reports that `SKILL.md` differs from the registry snapshot
- **THEN** synchronization SHALL materialize or adopt the current immutable package and atomically update the registry witness
- **AND** a subsequent drift detection SHALL not report the same metadata-change issue

#### Scenario: Repair the affected shipped Skill set
- **WHEN** legacy state affects `api-doc-generation`, `code-review`, `code-security-scan`, or `readme-generation`
- **THEN** each Skill SHALL follow the same convergent reconciliation rule without id-specific repair behavior

#### Scenario: Source cannot be safely reconciled
- **WHEN** an item cannot be repaired because its identity, package revision, path, or filesystem state is unsafe or unavailable
- **THEN** synchronization SHALL preserve the existing source and registry state for that item
- **AND** the result SHALL include a bounded per-Skill failure reason instead of reporting it as restored

### Requirement: Synchronization persists post-repair drift truth
The atomic synchronization record SHALL represent the state after successful repairs rather than retaining the pre-repair issue list as the latest persisted drift snapshot.

#### Scenario: Persist a successful synchronization
- **WHEN** all detected drift issues are repaired successfully
- **THEN** the committed drift snapshot SHALL contain no resolved issues and SHALL use the drift hash for the empty post-repair state

#### Scenario: Persist a partially successful synchronization
- **WHEN** only a subset of detected drift issues can be repaired
- **THEN** the committed drift snapshot SHALL retain only issues that remain observable after the committed repairs
- **AND** successful repairs and remaining failures SHALL be distinguishable in the synchronization result
