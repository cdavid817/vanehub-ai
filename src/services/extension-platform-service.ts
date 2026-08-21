import type { FeatureGateOverview, SetFeatureGateRequest } from "../types/extension-platform";

export interface ExtensionPlatformService {
  getFeatureGates(): Promise<FeatureGateOverview>;
  setFeatureGate(request: SetFeatureGateRequest): Promise<FeatureGateOverview>;
}
