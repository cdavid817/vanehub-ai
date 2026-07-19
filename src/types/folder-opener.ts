export const folderOpenerIds = [
  "vscode",
  "file-explorer",
  "windows-terminal",
  "git-bash",
  "intellij-idea",
  "webstorm",
] as const;

export type FolderOpenerId = (typeof folderOpenerIds)[number];
export type FolderOpenerStatus = "available" | "not-installed" | "invalid-installation" | "unsupported-platform" | "detection-failed";
export type FolderOpenerCategory = "editor" | "file-manager" | "terminal" | "ide";

export interface FolderOpenerAvailability {
  id: FolderOpenerId;
  category: FolderOpenerCategory;
  status: FolderOpenerStatus;
  executablePath: string | null;
  version: string | null;
  edition: string | null;
  detectionSource: string | null;
  iconKey: FolderOpenerId;
  reason: string | null;
}
export interface FolderOpenerPreferences {
  configuredDefaultOpenerId: FolderOpenerId;
  effectiveDefaultOpenerId: FolderOpenerId | null;
  enabledOpenerIds: FolderOpenerId[];
  fallbackActive: boolean;
}

export interface SaveFolderOpenerPreferencesInput {
  configuredDefaultOpenerId: FolderOpenerId;
  enabledOpenerIds: FolderOpenerId[];
}

export interface OpenSessionFolderResult {
  status: "opened" | "unavailable";
  openerId: FolderOpenerId;
  reason: string | null;
}
