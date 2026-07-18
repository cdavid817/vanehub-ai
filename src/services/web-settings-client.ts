import type { SettingsService, SettingsStateEvent } from "./settings-service";
import { defaultAppSettings, normalizeAppSettings, validateSettingValue } from "./settings-service";
import { i18n } from "../i18n";
import type { AppSettings, DataManagementInfo, NodeInfo } from "../types/settings";

const storageKey = "vanehub.appSettings";
const settingsSubscribers = new Set<(event: SettingsStateEvent) => void>();

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
    settingsSubscribers.forEach((handler) => handler({ kind: "settings-changed", key: input.key }));
    return nextSettings;
  },

  async setLaunchOnStartup(enabled) {
    void enabled;
    throw new Error(i18n.t("web.error.launchOnStartupDesktopOnly"));
  },

  async getNodeInfo(): Promise<NodeInfo> {
    return {
      available: false,
      path: null,
      version: null,
      reason: i18n.t("basic.nodeUnavailableReason"),
    };
  },

  async getDataManagementInfo(): Promise<DataManagementInfo> {
    return {
      databasePath: "localStorage:vanehub",
      databaseDirectory: i18n.t("basic.webDataStorage"),
      canOpenDirectory: false,
    };
  },

  async openDatabaseDirectory(): Promise<void> {
    throw new Error(i18n.t("web.error.openDatabaseDirectory"));
  },

  async openLogDirectory(): Promise<void> {
    throw new Error(i18n.t("web.error.openLogDirectory"));
  },

  async testNetworkProxy(): Promise<never> {
    throw new Error(i18n.t("web.error.networkProxyDesktopOnly"));
  },

  async scanNetworkProxies(): Promise<never> {
    throw new Error(i18n.t("web.error.networkProxyDesktopOnly"));
  },

  async reportClientLogEvent(): Promise<void> {
    return;
  },

  async subscribeSettingsEvents(handler) {
    settingsSubscribers.add(handler);
    return () => settingsSubscribers.delete(handler);
  },
};
