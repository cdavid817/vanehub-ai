import {
  folderOpenerIds,
  type FolderOpenerAvailability,
  type FolderOpenerCategory,
  type FolderOpenerId,
  type FolderOpenerPreferences,
  type FolderOpenerStatus,
} from "../types/folder-opener";

const statuses: FolderOpenerStatus[] = ["available", "not-installed", "invalid-installation", "unsupported-platform", "detection-failed"];
const categories: FolderOpenerCategory[] = ["editor", "file-manager", "terminal", "ide"];

export function isFolderOpenerId(value: unknown): value is FolderOpenerId {
  return typeof value === "string" && folderOpenerIds.includes(value as FolderOpenerId);
}

export function normalizeFolderOpeners(value: unknown): FolderOpenerAvailability[] {
  if (!Array.isArray(value)) throw new Error("Invalid folder opener response.");
  return value.map((entry) => {
    if (!entry || typeof entry !== "object") throw new Error("Invalid folder opener entry.");
    const item = entry as Partial<FolderOpenerAvailability>;
    if (!isFolderOpenerId(item.id) || !statuses.includes(item.status as FolderOpenerStatus) || !categories.includes(item.category as FolderOpenerCategory)) {
      throw new Error("Invalid folder opener entry.");
    }
    return {
      id: item.id,
      category: item.category as FolderOpenerCategory,
      status: item.status as FolderOpenerStatus,
      executablePath: typeof item.executablePath === "string" ? item.executablePath : null,
      version: typeof item.version === "string" ? item.version : null,
      edition: typeof item.edition === "string" ? item.edition : null,
      detectionSource: typeof item.detectionSource === "string" ? item.detectionSource : null,
      iconKey: isFolderOpenerId(item.iconKey) ? item.iconKey : item.id,
      reason: typeof item.reason === "string" ? item.reason : null,
    };
  });
}

export function normalizeFolderOpenerPreferences(value: unknown): FolderOpenerPreferences {
  if (!value || typeof value !== "object") throw new Error("Invalid folder opener preferences.");
  const item = value as Partial<FolderOpenerPreferences>;
  const enabled = Array.isArray(item.enabledOpenerIds) ? item.enabledOpenerIds.filter(isFolderOpenerId) : [];
  if (!isFolderOpenerId(item.configuredDefaultOpenerId) || !enabled.includes("file-explorer") || !enabled.includes(item.configuredDefaultOpenerId)) {
    throw new Error("Invalid folder opener preferences.");
  }
  return {
    configuredDefaultOpenerId: item.configuredDefaultOpenerId,
    effectiveDefaultOpenerId: isFolderOpenerId(item.effectiveDefaultOpenerId) ? item.effectiveDefaultOpenerId : null,
    enabledOpenerIds: [...new Set(enabled)],
    fallbackActive: item.fallbackActive === true,
  };
}

