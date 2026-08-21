import type { ExtensionPlatformService } from "./extension-platform-service";
import type {
  ExtensionPlatformFeature,
  FeatureGate,
  FeatureGateOverview,
  FeatureGateStatus,
} from "../types/extension-platform";

/**
 * Deterministic Web/mock gate state.
 *
 * The mock does not claim native capability. Both runtime-bearing gates report `not_compiled`,
 * which is what the browser build genuinely is — there is no Wasmtime engine and no sidecar host
 * here — rather than pretending they are merely switched off.
 */
const FEATURES: readonly ExtensionPlatformFeature[] = [
  "catalog",
  "external_packages",
  "lifecycle_hooks",
  "authorization_rules",
  "connectors",
  "wasm_module_runtime",
  "sidecar_runtime",
];

const BUILD_UNAVAILABLE: readonly ExtensionPlatformFeature[] = [
  "wasm_module_runtime",
  "sidecar_runtime",
];

function initialGates(): Map<ExtensionPlatformFeature, FeatureGate> {
  return new Map(
    FEATURES.map((feature) => [
      feature,
      {
        feature,
        status: statusFor(feature, false),
        buildAvailable: !BUILD_UNAVAILABLE.includes(feature),
        desiredEnabled: false,
        revision: 0,
        updatedAt: null,
        updatedBy: null,
        reason: null,
      },
    ]),
  );
}

function statusFor(feature: ExtensionPlatformFeature, desiredEnabled: boolean): FeatureGateStatus {
  if (BUILD_UNAVAILABLE.includes(feature)) return { kind: "not_compiled" };
  if (!desiredEnabled) return { kind: "runtime_disabled" };
  return { kind: "enabled" };
}

let gates = initialGates();

function overview(): FeatureGateOverview {
  return { gates: FEATURES.map((feature) => ({ ...readGate(feature) })) };
}

function readGate(feature: ExtensionPlatformFeature): FeatureGate {
  const gate = gates.get(feature);
  if (!gate) throw new Error(`Unknown capability gate: ${feature}`);
  return gate;
}

export const webExtensionPlatformClient: ExtensionPlatformService = {
  async getFeatureGates() {
    return overview();
  },
  async setFeatureGate({ feature, desiredEnabled, expectedRevision, reason }) {
    const current = readGate(feature);
    if (desiredEnabled && !current.buildAvailable) {
      throw new Error(`feature_unavailable_in_build: ${feature}`);
    }
    if (current.revision !== expectedRevision) {
      throw new Error(`stale_revision: ${feature}`);
    }
    const revision = current.revision + 1;
    gates.set(feature, {
      ...current,
      status: statusFor(feature, desiredEnabled),
      desiredEnabled,
      revision,
      updatedAt: new Date().toISOString(),
      updatedBy: "web-mock",
      reason: reason ?? null,
    });
    return overview();
  },
};

export function resetWebExtensionPlatformStateForTests() {
  gates = initialGates();
}
