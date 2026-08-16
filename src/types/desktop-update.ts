export type UpdateChannel = "stable" | "preview";
export type UpdatePhase = "idle" | "queued" | "checking" | "available" | "up-to-date" | "downloading" | "ready-to-restart" | "failed";

export interface UpdatePreferences { automaticCheck: boolean; channel: UpdateChannel; }
export interface DesktopUpdateSnapshot {
  phase: UpdatePhase; currentVersion: string; channel: UpdateChannel;
  latestVersion?: string; releaseNotes?: string; checkedAt?: string; operationId?: string;
  downloadedBytes?: number; totalBytes?: number; error?: string;
}
export interface UpdateOperationReceipt { operationId: string; snapshot: DesktopUpdateSnapshot; }
export interface UpdateManifestCandidate { version: string; channel: UpdateChannel; notes?: string; signature: string; url: string; }
