import { createRuntimeAdapter } from "./runtime-adapter";
import type { SettingsService } from "./settings-service";
import { tauriSettingsClient } from "./tauri-settings-client";
import { webSettingsClient } from "./web-settings-client";

export const settingsService: SettingsService = createRuntimeAdapter<SettingsService>({
  tauri: tauriSettingsClient,
  webMock: webSettingsClient,
});
