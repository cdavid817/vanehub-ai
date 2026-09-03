import { normalizeDisplayPath } from "../../lib/session-path";
import type { SafeSessionSummary, WorkspaceSummary } from "../../projects/workspace-summary";
import {
  DEFAULT_SEED, FIXTURE_RANGE_END_MS, FIXTURE_RANGE_START_MS, type SeededRandom,
  createSeededRandom, offsetTimestamp,
} from "./seeded-random";

/**
 * Task 13.13: deterministic `WorkspaceSummary` fixtures for the Projects and Workspaces
 * destination (`src/projects/`), one named builder per real state the type can honestly take —
 * see `workspace-summary.ts`'s own field comments, which this file follows rather than invents.
 * Matches this directory's established one-file-per-domain-type convention (compare
 * `session-fixtures.ts`/`goal-fixtures.ts`), and is shaped for new callers to reuse (13.14's own
 * Playwright/accessibility/privacy tests, in particular). Deliberately does not replace the
 * pre-existing, differently-shaped ad hoc `workspace(overrides)` helpers already local to
 * `workspace-filter.test.ts`/`workspace-detail.test.tsx`: several of their call sites depend on
 * *that* helper's bare, field-absent default (no `git`, no `recentSession`) to exercise empty-
 * state branches, which this file's named builders intentionally do not default to, so swapping
 * them in would silently change what those existing tests cover rather than just deduplicate.
 *
 * Every builder returns a fresh object per call (never a shared literal) and accepts
 * `Partial<WorkspaceSummary>` overrides, the same shape as this codebase's existing ad hoc
 * per-test `workspace(overrides)` helpers — deliberately not the `generate(count, seed)`-only
 * shape every other file here uses, because 13.13 asks for named, individually reachable
 * scenarios ("local Git", "missing", "revoked", ...), not a randomised distribution. A small
 * `generateWorkspaceSummaries` bulk generator at the bottom still follows that established
 * convention, for parity and for any future scale/pagination test.
 */

function session(overrides: Partial<SafeSessionSummary> = {}): SafeSessionSummary {
  return {
    id: "workspace-fixture-session-1", lifecycleState: "idle", title: "Fix flaky upload retry",
    updatedAt: "2026-08-18T09:30:00.000Z", ...overrides,
  };
}

/** A local Git repository, available, with a recent idle session -- the common case. */
export function localGitWorkspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  const path = "D:/workspace/vanehub-ai";
  return {
    availability: "available", displayName: "vanehub-ai", displayPath: normalizeDisplayPath(path),
    git: { repository: true }, kind: "local", lastOpenedAt: "2026-08-20T08:00:00.000Z",
    recentSession: session(), workspaceId: path, ...overrides,
  };
}

/** A local, available, plain folder -- `git.repository: false`, never `git: undefined` (that shape is SSH-only). */
export function nonGitWorkspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  const path = "D:/workspace/scratch-notes";
  return {
    availability: "available", displayName: "scratch-notes", displayPath: normalizeDisplayPath(path),
    git: { repository: false }, kind: "local", lastOpenedAt: "2026-07-02T14:15:00.000Z",
    workspaceId: path, ...overrides,
  };
}

/**
 * A remembered local path whose last `inspectProject` call rejected. `git.repository: true` here
 * is the historical `KnownProject.isGit` carried through, not a fabricated "checked and it is
 * one" claim -- `buildLocalWorkspaceSummary`'s own documented fallback for exactly this case.
 */
export function missingWorkspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  const path = "D:/workspace/deleted-prototype";
  return {
    availability: "missing", displayName: "deleted-prototype", displayPath: normalizeDisplayPath(path),
    git: { repository: true }, kind: "local", lastOpenedAt: "2026-03-11T11:45:00.000Z",
    workspaceId: path, ...overrides,
  };
}

/** An SSH workspace with a matched, successfully-tested connection -- trusted and reachable. */
export function remoteConnectedWorkspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  const uri = "ssh://vane@build.example.com/srv/app";
  return {
    availability: "available", connectionId: "ssh-fixture-connected", displayName: "build.example.com:app",
    displayPath: "vane@build.example.com:/srv/app", kind: "ssh", lastOpenedAt: "2026-08-22T06:00:00.000Z",
    recentSession: session({ id: "workspace-fixture-session-2", lifecycleState: "running", title: "Deploy staging build" }),
    trust: "trusted", workspaceId: uri, ...overrides,
  };
}

/** An SSH workspace with a matched connection that has never tested successfully -- trust stays honestly "unknown", not "untrusted". */
export function remoteDisconnectedWorkspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  const uri = "ssh://vane@staging.example.com/srv/app";
  return {
    availability: "disconnected", connectionId: "ssh-fixture-disconnected", displayName: "staging.example.com:app",
    displayPath: "vane@staging.example.com:/srv/app", kind: "ssh", lastOpenedAt: "2026-05-14T19:20:00.000Z",
    trust: "unknown", workspaceId: uri, ...overrides,
  };
}

/**
 * `untrustedWorkspace`/`revokedWorkspace` (below): `"untrusted"`/`"revoked"` are structurally
 * reachable in `WorkspaceTrust` and rendered by `workspace-card.tsx`/`workspace-detail.tsx`'s own
 * tone maps, but never produced by any real derivation path today -- confirmed by 13.10's own
 * audit (`workspace-summary.ts`'s doc comment on `WorkspaceTrust`): no service anywhere records
 * "the user explicitly distrusted this host" or "a previously-confirmed host key was superseded".
 * These two exist for defensive UI coverage of the type's full state space -- rendering,
 * accessibility, and privacy tests that must not crash or misrender if that data ever becomes
 * real -- not because either is an observed, real-service-produced scenario. Do not read their
 * presence here as evidence that trust revocation works end to end; it does not yet.
 */
export function untrustedWorkspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  const uri = "ssh://vane@legacy.example.com/srv/app";
  return {
    availability: "available", connectionId: "ssh-fixture-untrusted", displayName: "legacy.example.com:app",
    displayPath: "vane@legacy.example.com:/srv/app", kind: "ssh", lastOpenedAt: "2026-02-01T10:00:00.000Z",
    trust: "untrusted", workspaceId: uri, ...overrides,
  };
}

/** See `untrustedWorkspace`'s own doc comment directly above -- the same honest caveat applies here. */
export function revokedWorkspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  const uri = "ssh://vane@rotated.example.com/srv/app";
  return {
    availability: "available", connectionId: "ssh-fixture-revoked", displayName: "rotated.example.com:app",
    displayPath: "vane@rotated.example.com:/srv/app", kind: "ssh", lastOpenedAt: "2026-01-15T10:00:00.000Z",
    trust: "revoked", workspaceId: uri, ...overrides,
  };
}

/** The "no projects or workspaces yet" empty state (`projects.empty.*`) -- named so call sites read as intent, not a bare `[]`. */
export const EMPTY_WORKSPACE_LIST: readonly WorkspaceSummary[] = [];

const NAMED_SCENARIOS = [
  localGitWorkspace, nonGitWorkspace, missingWorkspace, remoteConnectedWorkspace,
  remoteDisconnectedWorkspace, untrustedWorkspace, revokedWorkspace,
] as const;

/**
 * `count` deterministic rows cycling through every named scenario above, each repeat given a
 * distinct `workspaceId`/`displayName`/`lastOpenedAt` so a caller can render many rows without
 * key collisions or identical timestamps. Bulk/scale convention shared by every other file in
 * this directory (e.g. `generateSessions`); unlike those, this still only ever emits the 7
 * states this file documents above, never a fabricated one.
 */
export function generateWorkspaceSummaries(count: number, seed: number = DEFAULT_SEED): WorkspaceSummary[] {
  const rng: SeededRandom = createSeededRandom(seed);
  return Array.from({ length: count }, (_unused, index) => {
    const base = NAMED_SCENARIOS[index % NAMED_SCENARIOS.length]();
    const suffix = `-${index}`;
    return {
      ...base,
      displayName: `${base.displayName}${suffix}`,
      lastOpenedAt: base.lastOpenedAt
        ? offsetTimestamp(base.lastOpenedAt, -1000 * 60 * 60 * 24 * 20, 1000 * 60 * 60 * 24 * 20, rng)
        : undefined,
      workspaceId: `${base.workspaceId}${suffix}`,
    };
  });
}

// Re-exported so `FIXTURE_RANGE_START_MS`/`FIXTURE_RANGE_END_MS` stay reachable from this module
// alone -- callers building their own overrides (e.g. a custom `lastOpenedAt`) should not also
// need a second import from `seeded-random.ts` just for the shared fixture time bounds.
export { FIXTURE_RANGE_END_MS, FIXTURE_RANGE_START_MS };
