import type { SessionSurfaceId } from "./session-surface-registry";

/**
 * The nine flat tab ids the workspace exposed before design.md Decision 7 — still the vocabulary
 * slash commands, Mission Control/Loop cross-navigation, and any deep link written before this
 * migration use. New internal code must use `SessionSurfaceId` directly; this type exists only to
 * name what a legacy caller is allowed to hand the adapter below.
 */
export type LegacySessionTabId =
  | "chat"
  | "changes"
  | "documents"
  | "files"
  | "terminal"
  | "shell"
  | "logs"
  | "traces"
  | "report";

/**
 * `documents` and `files` both land on the merged Files surface (design.md Decision 7's mapping
 * table); `chat` becomes `work`; `terminal` becomes the Runtime Panel's `terminal-history`. Every
 * other id is unchanged.
 */
const LEGACY_SESSION_SURFACE_MAP: Record<LegacySessionTabId, SessionSurfaceId> = {
  chat: "work",
  changes: "changes",
  documents: "files",
  files: "files",
  terminal: "terminal-history",
  shell: "shell",
  logs: "logs",
  traces: "traces",
  report: "report",
};

/** For a caller that already has a statically known legacy id — always succeeds. */
export function legacySessionSurfaceAdapter(id: LegacySessionTabId): SessionSurfaceId {
  return LEGACY_SESSION_SURFACE_MAP[id];
}

/**
 * For a caller holding an id only as a string — a stored preference or a deep link written before
 * this migration, say. Returns null rather than guessing so an unrecognized id fails visibly
 * instead of silently landing on whatever surface happens to be first in the map.
 */
export function resolveLegacySessionSurface(id: string): SessionSurfaceId | null {
  if (Object.hasOwn(LEGACY_SESSION_SURFACE_MAP, id)) {
    return LEGACY_SESSION_SURFACE_MAP[id as LegacySessionTabId];
  }
  if (import.meta.env.DEV) {
     
    // surface loudly in development, not a state the workspace should recover from silently.
    console.error(`legacySessionSurfaceAdapter: no target surface registered for legacy tab id "${id}"`);
  }
  return null;
}
