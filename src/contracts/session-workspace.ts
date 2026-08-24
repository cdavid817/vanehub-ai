export type {
  BoundedResult,
  DirectoryEntry,
  DirectoryEntryKind,
  DirectoryListing,
  DocumentKind,
  DocumentListing,
  FileContent,
  FileContentStatus,
  FileSearchListing,
  FileSearchMatch,
  GitChangeKind,
  GitDiffFile,
  GitDiffHunk,
  GitDiffLine,
  GitDiffLineKind,
  GitDiffResult,
  GitDiffSource,
  GitStatusEntry,
  GitStatusResult,
  SessionDocument,
  SessionLogEntry,
  SessionLogExportResult,
  SessionLogExportStatus,
  SessionLogLevel,
  SessionLogPage,
  SessionLogQuery,
  SessionWorkspaceContext,
  ShellRuntimeDescriptor,
  ShellRuntimeKind,
  WorkspaceAvailability,
} from "../types/session-workspace";

import type { ShellRuntimeDescriptor as ShellRuntimeDescriptorType } from "../types/session-workspace";

/**
 * Capabilities are derived from the runtime kind instead of being read off the wire. A transport
 * that claimed `simulated` with `supportsResize: true` would otherwise let the Shell view send a
 * resize to a runtime that has no PTY to resize.
 */
export function normalizeShellRuntimeDescriptor(value: unknown): ShellRuntimeDescriptorType {
  const invalid = new Error("Invalid shell runtime descriptor.");
  if (!value || typeof value !== "object") throw invalid;
  const descriptor = value as Record<string, unknown>;
  switch (descriptor.kind) {
    case "native":
      return { kind: "native", supportsResize: true, supportsReplay: true, supportsReconnect: false };
    case "remote": {
      if (typeof descriptor.connectionId !== "string" || descriptor.connectionId.length === 0) throw invalid;
      if (typeof descriptor.profileRevision !== "number" || !Number.isFinite(descriptor.profileRevision)) throw invalid;
      return {
        kind: "remote",
        connectionId: descriptor.connectionId,
        profileRevision: descriptor.profileRevision,
        supportsResize: true,
        supportsReplay: true,
        supportsReconnect: descriptor.supportsReconnect === true,
      };
    }
    case "simulated":
      return { kind: "simulated", supportsResize: false, supportsReplay: true, supportsReconnect: false };
    case "unavailable": {
      if (typeof descriptor.reasonCode !== "string" || descriptor.reasonCode.length === 0) throw invalid;
      return typeof descriptor.remediation === "string"
        ? { kind: "unavailable", reasonCode: descriptor.reasonCode, remediation: descriptor.remediation }
        : { kind: "unavailable", reasonCode: descriptor.reasonCode };
    }
    default:
      throw invalid;
  }
}
