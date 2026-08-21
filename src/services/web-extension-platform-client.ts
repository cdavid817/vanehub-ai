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
 * A fixture table, not a second copy of the native evaluation rules. Each gate declares the two
 * statuses it can present and the mock swaps between them; build availability, forced-disable
 * precedence, and prerequisite ordering all stay in Rust, where they are tested. If those rules
 * change, this file keeps returning honest fixtures rather than silently disagreeing.
 *
 * The two runtime-bearing gates report `not_compiled` in both positions because that is what a
 * browser build genuinely is — there is no Wasmtime engine and no sidecar host here — rather than
 * implying they are merely switched off.
 */
interface GateFixture {
  readonly feature: ExtensionPlatformFeature;
  readonly buildAvailable: boolean;
  readonly disabledStatus: FeatureGateStatus;
  readonly enabledStatus: FeatureGateStatus;
}

const NOT_COMPILED: FeatureGateStatus = { kind: "not_compiled" };

const GATE_FIXTURES: readonly GateFixture[] = [
  {
    feature: "catalog",
    buildAvailable: true,
    disabledStatus: { kind: "runtime_disabled" },
    enabledStatus: { kind: "enabled" },
  },
  {
    feature: "external_packages",
    buildAvailable: true,
    disabledStatus: { kind: "runtime_disabled" },
    enabledStatus: { kind: "enabled" },
  },
  {
    feature: "lifecycle_hooks",
    buildAvailable: true,
    disabledStatus: { kind: "runtime_disabled" },
    enabledStatus: { kind: "enabled" },
  },
  {
    feature: "authorization_rules",
    buildAvailable: true,
    disabledStatus: { kind: "runtime_disabled" },
    enabledStatus: { kind: "enabled" },
  },
  {
    feature: "connectors",
    buildAvailable: true,
    disabledStatus: { kind: "runtime_disabled" },
    enabledStatus: { kind: "enabled" },
  },
  {
    feature: "wasm_module_runtime",
    buildAvailable: false,
    disabledStatus: NOT_COMPILED,
    enabledStatus: NOT_COMPILED,
  },
  {
    feature: "sidecar_runtime",
    buildAvailable: false,
    disabledStatus: NOT_COMPILED,
    enabledStatus: NOT_COMPILED,
  },
];

function fixtureFor(feature: ExtensionPlatformFeature): GateFixture {
  const fixture = GATE_FIXTURES.find((candidate) => candidate.feature === feature);
  if (!fixture) throw new Error(`Unknown capability gate: ${feature}`);
  return fixture;
}

function initialGates(): Map<ExtensionPlatformFeature, FeatureGate> {
  return new Map(
    GATE_FIXTURES.map((fixture) => [
      fixture.feature,
      {
        feature: fixture.feature,
        status: fixture.disabledStatus,
        buildAvailable: fixture.buildAvailable,
        desiredEnabled: false,
        revision: 0,
        updatedAt: null,
        updatedBy: null,
        reason: null,
      },
    ]),
  );
}

let gates = initialGates();

function readGate(feature: ExtensionPlatformFeature): FeatureGate {
  const gate = gates.get(feature);
  if (!gate) throw new Error(`Unknown capability gate: ${feature}`);
  return gate;
}

function overview(): FeatureGateOverview {
  return {
    gates: GATE_FIXTURES.map((fixture) => ({ ...readGate(fixture.feature) })),
    freshness: { kind: "current" },
  };
}

export const webExtensionPlatformClient: ExtensionPlatformService = {
  async getFeatureGates() {
    return overview();
  },
  async setFeatureGate({ feature, desiredEnabled, expectedRevision, reason }) {
    const fixture = fixtureFor(feature);
    const current = readGate(feature);
    if (desiredEnabled && !fixture.buildAvailable) {
      throw new Error(`feature_unavailable_in_build: ${feature}`);
    }
    if (current.revision !== expectedRevision) {
      throw new Error(`stale_revision: ${feature}`);
    }
    gates.set(feature, {
      ...current,
      status: desiredEnabled ? fixture.enabledStatus : fixture.disabledStatus,
      desiredEnabled,
      revision: current.revision + 1,
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
