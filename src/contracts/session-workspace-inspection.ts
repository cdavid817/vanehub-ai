import { z } from "zod";
import type {
  WorkspaceInspectionCapabilities,
  WorkspaceInvalidationNotice,
  WorkspaceContentSearchResult,
  WorkspacePathSearchResult,
} from "../types/session-workspace-inspection";

export const capabilityStateSchema = z.object({
  available: z.boolean(),
  reasonCode: z.string().optional(),
  remediation: z.string().optional(),
});

export const workspaceInspectionCapabilitiesSchema = z.object({
  provider: z.enum(["local", "ssh", "simulated"]),
  listFiles: capabilityStateSchema,
  readTextFiles: capabilityStateSchema,
  searchFiles: capabilityStateSchema,
  gitStatus: capabilityStateSchema,
  gitDiff: capabilityStateSchema,
  watchMode: z.enum(["native", "polling", "event-derived", "none"]),
});

export function parseWorkspaceInspectionCapabilities(value: unknown): WorkspaceInspectionCapabilities {
  return workspaceInspectionCapabilitiesSchema.parse(value);
}

/**
 * Parsed rather than trusted, even though it comes from this application's own native side.
 *
 * The drift this guards against is a native and a frontend vocabulary growing apart. A scope token
 * nobody recognises would otherwise fall through to "refresh nothing", which on screen is
 * indistinguishable from a workspace where nothing happened.
 */
export const workspacePathSearchResultSchema = z.object({
  coverage: z.object({
    state: z.enum(["complete", "partial", "unavailable"]),
    reasonCode: z.string().optional(),
  }),
  matches: z.array(
    z.object({
      name: z.string(),
      path: z.string(),
      kind: z.enum(["file", "directory"]),
    }),
  ),
  nextCursor: z.string().optional(),
});

export function parseWorkspacePathSearchResult(value: unknown): WorkspacePathSearchResult {
  return workspacePathSearchResultSchema.parse(value);
}

export const workspaceContentSearchResultSchema = z.object({
  coverage: z.object({
    state: z.enum(["complete", "partial", "unavailable"]),
    reasonCode: z.string().optional(),
  }),
  matches: z.array(
    z.object({
      path: z.string(),
      // Positive rather than merely non-negative: line and column are 1-based, and a zero would be
      // a position no editor can go to.
      line: z.number().int().positive(),
      column: z.number().int().positive(),
      snippet: z.string(),
      snippetTruncated: z.boolean(),
    }),
  ),
});

export function parseWorkspaceContentSearchResult(value: unknown): WorkspaceContentSearchResult {
  return workspaceContentSearchResultSchema.parse(value);
}

export const workspaceInvalidationNoticeSchema = z.object({
  sessionId: z.string().min(1),
  source: z.enum(["watch", "poll", "execution-evidence"]),
  scope: z.enum(["path", "directory", "workspace"]),
  relativePath: z.string().optional(),
  change: z.enum(["created", "modified", "removed", "unknown"]).optional(),
  sequence: z.number().int().nonnegative(),
  occurredAt: z.string().min(1),
  coalesced: z.number().int().positive().optional(),
});

/**
 * Null rather than a throw: an event handler has no caller to reject to, and one malformed notice
 * must not tear down a live subscription.
 */
export function safeParseWorkspaceInvalidationNotice(
  value: unknown,
): WorkspaceInvalidationNotice | null {
  const parsed = workspaceInvalidationNoticeSchema.safeParse(value);
  if (!parsed.success) return null;
  // A scope that names a path without carrying one cannot be acted on: the reader would have to
  // guess which directory, and a guessed path refreshes the wrong query while leaving the right one
  // stale. Refused here so it never reaches the router.
  if (parsed.data.scope !== "workspace" && !parsed.data.relativePath) return null;
  return parsed.data;
}
