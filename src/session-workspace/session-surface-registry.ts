/**
 * The declarative registry design.md Decision 7 calls for: one table instead of tab-id
 * conditionals scattered across the workspace.
 *
 * Evolves `workspace-tab-capability.ts` (the nine-flat-tab predecessor) rather than replacing its
 * vocabulary from scratch — `seatMode`/`retention` already existed there under different value
 * spellings (`optional`/`keep-live`) for exactly the same distinctions this registry's `scope` and
 * `retention` make. What's new here: the `region` split (primary vs. Runtime Panel) design.md
 * Decision 7 introduces, and the `chat` -> `work` rename / `documents` + `files` merge that moves
 * nine ids down to eight.
 */
export type SessionPrimarySurfaceId = "work" | "changes" | "files" | "report";
export type SessionRuntimeSurfaceId = "terminal-history" | "shell" | "logs" | "traces";
export type SessionSurfaceId = SessionPrimarySurfaceId | SessionRuntimeSurfaceId;

/**
 * Whether a surface is about one participant's work or the whole session's.
 *
 * `seat-required` is not the same as `seat-optional` with a default. A Shell is one interactive
 * channel with one runtime owner, so a multi-Agent session must say whose it is; a Terminal
 * History with no seat chosen is a legitimate view of everyone's work.
 */
export type SessionSurfaceScope = "session" | "seat-optional" | "seat-required";

/**
 * What a hidden surface is allowed to keep.
 *
 * `unmount` throws the panel away, `cache` keeps what the user typed and selected while its
 * background work stops, and `keep-mounted-while-active-run` additionally keeps a running
 * attachment alive — a shell whose process must outlive a glance at another tab.
 */
export type SessionSurfaceRetention = "unmount" | "cache" | "keep-mounted-while-active-run";

/**
 * Whether a surface has work that continues without the user watching it, and whether that work
 * is allowed to keep running while the surface itself is not the one on screen.
 *
 * `background-terminal` is strictly stronger than `visible`: a live PTY or CLI process the user
 * started does not stop because they looked at another tab, but a live *query* (Logs' notice
 * subscription, Traces' polling) has no reason to keep running for nobody to read.
 */
export type SessionSurfaceLiveUpdates = "none" | "visible" | "background-terminal";

export interface SessionSurfaceDefinition {
  id: SessionSurfaceId;
  region: "primary" | "runtime";
  labelKey: string;
  scope: SessionSurfaceScope;
  retention: SessionSurfaceRetention;
  liveUpdates: SessionSurfaceLiveUpdates;
}

/**
 * `satisfies` rather than an annotation so a surface added to either region without a decision
 * here fails to compile, and so each entry keeps its literal type for callers that narrow on it.
 */
export const SESSION_SURFACE_REGISTRY = {
  work: {
    id: "work",
    region: "primary",
    labelKey: "sessionTabs.tab.work",
    scope: "session",
    // The Agent CLI/live chat keeps running while the user reads another tab; tearing it down
    // would end work the user started.
    retention: "keep-mounted-while-active-run",
    liveUpdates: "background-terminal",
  },
  changes: {
    id: "changes",
    region: "primary",
    labelKey: "sessionTabs.tab.changes",
    scope: "session",
    retention: "cache",
    liveUpdates: "none",
  },
  files: {
    id: "files",
    region: "primary",
    labelKey: "sessionTabs.tab.files",
    scope: "session",
    retention: "cache",
    liveUpdates: "none",
  },
  report: {
    id: "report",
    region: "primary",
    labelKey: "sessionTabs.tab.report",
    scope: "session",
    retention: "cache",
    liveUpdates: "none",
  },
  "terminal-history": {
    id: "terminal-history",
    region: "runtime",
    labelKey: "sessionTabs.tab.terminal-history",
    scope: "seat-optional",
    retention: "cache",
    liveUpdates: "visible",
  },
  shell: {
    id: "shell",
    region: "runtime",
    labelKey: "sessionTabs.tab.shell",
    scope: "seat-required",
    // The native shell is not the view. Hiding the tab detaches the xterm surface; the process,
    // its scrollback, and its working directory stay exactly as they were.
    retention: "keep-mounted-while-active-run",
    liveUpdates: "background-terminal",
  },
  logs: {
    id: "logs",
    region: "runtime",
    labelKey: "sessionTabs.tab.logs",
    scope: "seat-optional",
    retention: "cache",
    liveUpdates: "visible",
  },
  traces: {
    id: "traces",
    region: "runtime",
    labelKey: "sessionTabs.tab.traces",
    scope: "session",
    retention: "cache",
    liveUpdates: "visible",
  },
} satisfies Record<SessionSurfaceId, SessionSurfaceDefinition>;

export const SESSION_PRIMARY_SURFACE_IDS: readonly SessionPrimarySurfaceId[] = Object.freeze([
  "work",
  "changes",
  "files",
  "report",
]);

export const SESSION_RUNTIME_SURFACE_IDS: readonly SessionRuntimeSurfaceId[] = Object.freeze([
  "terminal-history",
  "shell",
  "logs",
  "traces",
]);

export function sessionSurfaceDefinition(id: SessionSurfaceId): SessionSurfaceDefinition {
  return SESSION_SURFACE_REGISTRY[id];
}

/**
 * The definition of a surface id not known at compile time. Returns null rather than a permissive
 * default — a dynamically arriving id that inherited "session-scoped, no live work, cached" would
 * look correct and behave wrongly.
 */
export function lookupSessionSurfaceDefinition(id: string): SessionSurfaceDefinition | null {
  return Object.hasOwn(SESSION_SURFACE_REGISTRY, id)
    ? SESSION_SURFACE_REGISTRY[id as SessionSurfaceId]
    : null;
}

export function isRuntimeSurface(id: SessionSurfaceId): id is SessionRuntimeSurfaceId {
  return sessionSurfaceDefinition(id).region === "runtime";
}

export function isPrimarySurface(id: SessionSurfaceId): id is SessionPrimarySurfaceId {
  return sessionSurfaceDefinition(id).region === "primary";
}

/**
 * Whether the workspace-level seat switcher applies to this surface.
 *
 * A single-seat session has one option, so the control would be a statement with no alternative.
 */
export function showsSessionSeatSwitcher(id: SessionSurfaceId, seatCount: number): boolean {
  return sessionSurfaceDefinition(id).scope !== "session" && seatCount > 1;
}
