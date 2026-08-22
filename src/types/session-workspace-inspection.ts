export type WorkspaceInspectionProviderKind = "local" | "ssh" | "simulated";

/**
 * A capability the provider either has or does not. `reasonCode` explains an absence in terms the
 * UI can localize; a blank panel with a generic error is the outcome this type exists to prevent.
 */
export interface CapabilityState {
  available: boolean;
  reasonCode?: string;
  remediation?: string;
}

/**
 * How change detection works for this provider. `none` is honest about a target that cannot report
 * changes at all, which is different from one that simply has not reported any yet.
 */
export type WorkspaceWatchMode = "native" | "polling" | "event-derived" | "none";

export interface WorkspaceInspectionCapabilities {
  provider: WorkspaceInspectionProviderKind;
  listFiles: CapabilityState;
  readTextFiles: CapabilityState;
  searchFiles: CapabilityState;
  gitStatus: CapabilityState;
  gitDiff: CapabilityState;
  watchMode: WorkspaceWatchMode;
}
