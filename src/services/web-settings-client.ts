import type { SettingsService } from "./settings-service";
import { defaultAppSettings, normalizeAppSettings, validateSettingValue } from "./settings-service";
import type { AppSettings, NodeInfo } from "../types/settings";

const storageKey = "vanehub.appSettings";

function readStoredSettings(): AppSettings {
  if (typeof window === "undefined") return defaultAppSettings;
  const raw = window.localStorage.getItem(storageKey);
  if (!raw) return defaultAppSettings;
  try {
    return normalizeAppSettings(JSON.parse(raw) as Partial<AppSettings>);
  } catch {
    return defaultAppSettings;
  }
}

function writeStoredSettings(settings: AppSettings) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(storageKey, JSON.stringify(settings));
}

export const webSettingsClient: SettingsService = {
  async getSettings() {
    return readStoredSettings();
  },

  async saveSetting(input) {
    validateSettingValue(input.key, input.value);
    const nextSettings = { ...readStoredSettings(), [input.key]: input.value };
    writeStoredSettings(nextSettings);
    return nextSettings;
  },

  async getNodeInfo(): Promise<NodeInfo> {
    return {
      available: false,
      path: null,
      version: null,
      reason: "Node.js information is only available in the desktop runtime.",
    };
  },
};
