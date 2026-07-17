import { invoke } from "@tauri-apps/api/core";
import type { ExtensionService } from "./extension-service";
import type { ExtensionInstallPreview, ExtensionOverview } from "../types/extension";
import type { OperationTask } from "../types/operation";

export const tauriExtensionClient: ExtensionService = {
  getOverview() {
    return invoke<ExtensionOverview>("get_extension_overview");
  },
  refreshHealth() {
    return invoke<ExtensionOverview>("refresh_extension_health");
  },
  getInstallPreview(request) {
    return invoke<ExtensionInstallPreview>("get_extension_install_preview", { request });
  },
  install(request) {
    return invoke<OperationTask>("install_extension", { request });
  },
  uninstall(request) {
    return invoke<OperationTask>("uninstall_extension", { request });
  },
  setEnabled(request) {
    return invoke<OperationTask>("set_extension_enabled", { request });
  },
  start(request) {
    return invoke<OperationTask>("start_extension", { request });
  },
  stop(request) {
    return invoke<OperationTask>("stop_extension", { request });
  },
  selfTest(request) {
    return invoke<OperationTask>("test_extension", { request });
  },
};
