import type { SessionTabId } from "./session-tab-bar";
import { evidenceTabOf } from "./workspace-evidence-reducer";
import { tabConsumesScope } from "./workspace-evidence-navigation";

/**
 * Whether a tab is about one participant's work or the whole session's.
 *
 * `required` is not the same as `optional` with a default. A Shell is one interactive channel with
 * one runtime owner, so a multi-Agent session must say whose it is; a Terminal History with no seat
 * chosen is a legitimate view of everyone's work.
 */
export type WorkspaceSeatMode = "none" | "optional" | "required";

/**
 * What a hidden tab is allowed to keep.
 *
 * `unmount` throws the panel away, `keep-state` keeps what the user typed and selected while its
 * background work stops, and `keep-live` additionally keeps a running attachment — a shell whose
 * process must outlive a glance at another tab.
 */
export type WorkspaceTabRetention = "unmount" | "keep-state" | "keep-live";

export interface WorkspaceTabCapability {
  id: SessionTabId;
  seatMode: WorkspaceSeatMode;
  /** Whether the tab has work that continues without the user: a subscription, a poll, a stream. */
  supportsLive: boolean;
  retention: WorkspaceTabRetention;
  /** Whether the tab reads the cross-panel evidence scope. Drives the filter chips. */
  consumesScope: boolean;
}

/**
 * One table instead of tab-id conditionals scattered across the workspace.
 *
 * The failure it replaces is quiet: a `tab === "shell" || tab === "logs"` written in one file and
 * a `["terminal", "shell", "logs"]` written in another disagree the moment a tab is added, and the
 * disagreement surfaces as a control that renders for the wrong tab rather than as an error.
 *
 * `satisfies` rather than an annotation so a tab added to the bar without a decision here fails to
 * compile, and so each entry keeps its literal type for callers that narrow on it.
 */
export const WORKSPACE_TAB_CAPABILITIES = {
  chat: {
    id: "chat",
    seatMode: "none",
    supportsLive: true,
    // The Agent CLI keeps running while the user reads another tab; tearing its terminal down
    // would end work the user started.
    retention: "keep-live",
    consumesScope: false,
  },
  changes: {
    id: "changes",
    seatMode: "none",
    supportsLive: false,
    retention: "keep-state",
    consumesScope: true,
  },
  documents: {
    id: "documents",
    seatMode: "none",
    supportsLive: false,
    retention: "keep-state",
    consumesScope: true,
  },
  files: {
    id: "files",
    seatMode: "none",
    supportsLive: false,
    retention: "keep-state",
    consumesScope: true,
  },
  terminal: {
    id: "terminal",
    seatMode: "optional",
    supportsLive: true,
    retention: "keep-state",
    consumesScope: true,
  },
  shell: {
    id: "shell",
    seatMode: "required",
    supportsLive: true,
    // The native shell is not the view. Hiding the tab detaches the xterm surface; the process,
    // its scrollback, and its working directory stay exactly as they were.
    retention: "keep-live",
    consumesScope: true,
  },
  logs: {
    id: "logs",
    seatMode: "optional",
    supportsLive: true,
    retention: "keep-state",
    consumesScope: true,
  },
  traces: {
    id: "traces",
    seatMode: "none",
    supportsLive: true,
    retention: "keep-state",
    consumesScope: true,
  },
  report: {
    id: "report",
    seatMode: "none",
    supportsLive: false,
    retention: "keep-state",
    consumesScope: true,
  },
} satisfies Record<SessionTabId, WorkspaceTabCapability>;

export function workspaceTabCapability(tab: SessionTabId): WorkspaceTabCapability {
  return WORKSPACE_TAB_CAPABILITIES[tab];
}

/**
 * The capability of a tab whose id is not known at compile time.
 *
 * Returns null rather than a permissive default. A dynamically registered tab — an OnePiece
 * surface, a plugin panel — that inherited "session-scoped, no live work, keep state" would look
 * correct and behave wrongly: its subscription would keep running while hidden, and the seat
 * switcher would refuse to appear for a panel that needed one.
 */
export function lookupWorkspaceTabCapability(tab: string): WorkspaceTabCapability | null {
  return Object.hasOwn(WORKSPACE_TAB_CAPABILITIES, tab)
    ? WORKSPACE_TAB_CAPABILITIES[tab as SessionTabId]
    : null;
}

/**
 * Whether the workspace-level seat switcher applies to this tab.
 *
 * A single-seat session has one option, so the control would be a statement with no alternative.
 */
export function showsWorkspaceSeatSwitcher(tab: SessionTabId, seatCount: number): boolean {
  return workspaceTabCapability(tab).seatMode !== "none" && seatCount > 1;
}

/**
 * Checks the two tables that could drift: a tab that consumes scope must have a destination, and a
 * destination must consume scope. Exported so a test can assert it rather than a comment claim it.
 */
export function capabilityScopeDisagreements(): string[] {
  return Object.values(WORKSPACE_TAB_CAPABILITIES).flatMap((capability) => {
    const destination = evidenceTabOf(capability.id);
    const consumes = destination !== null && tabConsumesScope(destination);
    return consumes === capability.consumesScope
      ? []
      : [`${capability.id}: consumesScope=${capability.consumesScope} but destination=${destination}`];
  });
}
