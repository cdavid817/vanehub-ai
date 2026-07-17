import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SettingsService, SettingsStateEvent } from "./settings-service";
import { normalizeAppSettings } from "./settings-service";
import type { AppSettings, DetectedNetworkProxy, NetworkProxyTestResult, NodeInfo } from "../types/settings";

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

  async testNetworkProxy(input) {
    return invoke<NetworkProxyTestResult>("test_network_proxy", { input });
  },

  async scanNetworkProxies() {
    return invoke<DetectedNetworkProxy[]>("scan_network_proxies");
  },

  async reportClientLogEvent(event) {
    await invoke<void>("report_client_log_event", { event });
  },

  async subscribeSettingsEvents(handler) {
    return listen<SettingsStateEvent>("settings:event", (event) => handler(event.payload));
  },
};
