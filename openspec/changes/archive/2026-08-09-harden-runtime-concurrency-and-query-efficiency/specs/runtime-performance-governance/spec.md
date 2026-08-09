## ADDED Requirements

### Requirement: Blocking workspace/git/log work moves off the Tauri main thread

A Tauri command whose call path runs blocking subprocess execution (git inspection), whole-log file reads, file-export dialogs, or directory walks SHALL be `async` and SHALL run that blocking work on the blocking thread pool via `spawn_blocking`, not on the Tauri main thread. The `spawn_blocking` call SHALL live in the runtime/api layer, not in the command adapter, so the command layer stays free of IO primitives.

#### Scenario: git inspection on a slow repository

- **WHEN** a workspace command runs `git status` or `git diff` against a repository that takes seconds to respond
- **THEN** the work SHALL run on the blocking pool and SHALL NOT freeze the UI

#### Scenario: A diff preflight for a single path

- **WHEN** a git diff command needs to decide whether one path is untracked
- **THEN** it SHALL use a single-path query rather than a full-directory `git status` walk followed by a second git spawn

### Requirement: Repository reads batch instead of per-row on hot paths

A repository method that loads detail for a list of parent entities (workspaces, runs, source ids, documents being reconciled) SHALL load them in one query (or one transaction for writes) rather than one round-trip per item. A per-entity fallback MAY remain for single-entity lookups. Batched results SHALL be asserted equal to per-entity results in tests.

#### Scenario: A list endpoint loads status for every workspace

- **WHEN** a command lists code-index workspaces with their status
- **THEN** the repository SHALL run one aggregated query with correlated counts and a window function for each workspace's latest failure, not one status query per workspace

#### Scenario: A reconcile diff applies many upserts and deletes

- **WHEN** a reconciliation pass produces a set of changed documents and orphaned source ids
- **THEN** the repository SHALL apply the whole diff inside one transaction with prepared statements, not one autocommit per row

### Requirement: Chat stream events apply in batched traversals

The frontend SHALL coalesce high-frequency stream events (token, thinking) and apply them as a batch in a single message-array traversal per animation frame, rather than rebuilding the array once per event. Terminal events (completed/failed/cancelled) SHALL flush immediately so the stop indicator is not delayed.

#### Scenario: A turn emits thousands of token events

- **WHEN** an agent turn emits a burst of token events
- **THEN** the message array SHALL be rebuilt at most once per animation frame, not once per token
- **AND** messages that no event touched SHALL keep their reference identity so memoized children skip re-rendering
