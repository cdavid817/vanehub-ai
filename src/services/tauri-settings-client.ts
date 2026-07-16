import { invoke } from "@tauri-apps/api/core";
import type { SettingsService } from "./settings-service";
import { normalizeAppSettings } from "./settings-service";
import type { AppSettings, NodeInfo } from "../types/settings";

export const tauriSettingsClient: SettingsService = {
  async getSettings() {
    const settings = await invoke<AppSettings>("get_settings");
    return normalizeAppSettings(settings);
  },

  async saveSetting(input) {
    const settings = await invoke<AppSettings>("save_setting", { input });
    return normalizeAppSettings(settings);
  },

  async getNodeInfo() {
    return invoke<NodeInfo>("get_node_info");
  },

  async openLogDirectory() {
    await invoke<void>("open_log_directory");
  },

  async reportClientLogEvent(event) {
    await invoke<void>("report_client_log_event", { event });
  },
};
