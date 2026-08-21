import { invoke } from "@tauri-apps/api/core";
import type { ExtensionPlatformService } from "./extension-platform-service";
import type { FeatureGateOverview } from "../types/extension-platform";

export const tauriExtensionPlatformClient: ExtensionPlatformService = {
  getFeatureGates() {
    return invoke<FeatureGateOverview>("get_extension_feature_gates");
  },
  setFeatureGate(request) {
    return invoke<FeatureGateOverview>("set_extension_feature_gate", { request });
  },
};
