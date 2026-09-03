import type { SessionLifecycleState } from "../types/agent";

/**
 * Read-only projection assembled by `workspace-aggregation.ts` from several existing services
 * (design.md Decision 18). This is deliberately not identical to design.md's own sketch of
 * `WorkspaceSummary` — two fields were tightened after checking what the real services can
 * actually back, documented at each deviation below. Nothing here is written back anywhere;
 * write operations stay in their existing services/dialogs (§13 scope note).
 */
export type WorkspaceKind = "local" | "ssh";

/**
 * `"untrusted"` and `"revoked"` are structurally reachable (so rendering code must handle them)
 * but never produced by this increment's derivation logic: no service anywhere records "the user
 * explicitly distrusted this host" or "a previously-confirmed host key was superseded" —
 * `SshHostTrustMetadata` only ever holds the current fingerprint, nothing about a prior one it
 * replaced. Producing either value would mean guessing. This is what task 13.10 (persistent
 * trust/host-identity-change surfacing) is blocked on.
 */
export type WorkspaceTrust = "trusted" | "untrusted" | "unknown" | "revoked";

/**
 * `"missing"` is only ever produced for local rows (a remembered path that no longer resolves on
 * disk). Remote rows can only become `"available"` or `"disconnected"` — nothing in the SSH
 * service surface can confirm whether a specific *remote path* still exists without an active
 * connection, so `"missing"` would be a guess for `kind: "ssh"`.
 */
export type WorkspaceAvailability = "available" | "missing" | "disconnected";

export interface WorkspaceGitContext {
  repository: boolean;
  /**
   * Deliberately never populated in this increment. `ProjectInspection.gitRoot` (from
   * `inspectProject`) tells us a path *is* a repository, but branch/dirty need a further
   * git-status call this slice does not make — see `workspace-aggregation.ts` for why the
   * inspection call this increment already makes for availability was not also stretched to
   * cover these. Matches `WorkspaceSummary.git` being optional in design.md: the gap is expected,
   * not something to force with invented data.
   */
  branch?: string;
  dirty?: boolean;
  worktree?: string;
}

/**
 * A minimal, safe session projection for a workspace's "recent session" card — never the full
 * `Session` (no raw folder path duplicated beyond what `WorkspaceSummary.displayPath` already
 * shows, no seat/runtime detail). Design.md references a `SafeSessionSummary` type that does not
 * exist yet anywhere in the codebase; this is this increment's own definition of it.
 */
export interface SafeSessionSummary {
  id: string;
  title: string;
  lifecycleState: SessionLifecycleState;
  updatedAt: string;
}

export interface WorkspaceSummary {
  /**
   * For `kind: "local"`, the project's own filesystem path (already the natural unique id —
   * `KnownProject` has no separate id field). For `kind: "ssh"`, `RemoteWorkspace.uri`, which
   * `web-known-workspace-client.ts` already treats as this type's own de-facto unique key. Always
   * the raw canonical value, never display-normalized (task 13.11) — this is what a future
   * service call would need to receive.
   */
  workspaceId: string;
  kind: WorkspaceKind;
  displayName: string;
  /** `normalizeDisplayPath`-safe for rendering; `workspaceId` remains the canonical value. */
  displayPath: string;
  /** Present only for local rows with a resolvable inspection; absent (not fabricated) otherwise. */
  git?: WorkspaceGitContext;
  /**
   * Deviates from design.md's non-optional `trust` field: local filesystem access has no trust
   * concept at all (no field in `KnownProject` even gestures at one), so forcing a value onto
   * every row would misleadingly imply local paths carry the same "trust a remote host" semantics
   * SSH rows do. Present (and always `"trusted"` or `"unknown"` in this increment) for `kind:
   * "ssh"` rows only; absent for `kind: "local"` rows.
   */
  trust?: WorkspaceTrust;
  recentSession?: SafeSessionSummary;
  lastOpenedAt?: string;
  availability: WorkspaceAvailability;
  /**
   * The `SshConnection.id` `workspace-aggregation.ts`'s own `matchConnection` already found for
   * this row while deriving `trust`/`availability`/`recentSession` above -- surfaced here (task
   * 13.8's Reconnect action) rather than re-derived a second time, since no shared foreign key
   * exists between `KnownRemoteWorkspace` and `SshConnection` for a caller outside that function to
   * redo the match safely (see `matchConnection`'s own doc comment). Present only for `kind: "ssh"`
   * rows where a match was found; absent both for `kind: "local"` rows (no concept of a connection
   * at all) and for unmatched `kind: "ssh"` rows (a remembered path with no saved profile for its
   * host/port/user) -- callers must treat "absent" as "nothing to reconnect", never guess an id.
   */
  connectionId?: string;
  /**
   * design.md sketches a required `activeRuns: number`. Deliberately omitted here rather than
   * kept and hardcoded to 0: `MissionControlRunSummary.projectId` is hardcoded `null` in both the
   * Web mock (`web-mission-control-client.ts`) and the real Rust backend
   * (`src-tauri/src/contexts/operations/application/mission_control.rs`, confirmed by reading
   * both directly), so a per-workspace run count cannot be answered honestly by either backend
   * today. A `0` would read as "confirmed zero active runs", which is false — it would actually
   * mean "unknown". Whoever picks up 13.7/13.8 needs a real cross-domain join (or a backend
   * change) before this field can exist; this increment does not fabricate one.
   */
}
