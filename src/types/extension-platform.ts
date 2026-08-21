export type ExtensionPlatformFeature =
  | "catalog"
  | "external_packages"
  | "lifecycle_hooks"
  | "authorization_rules"
  | "connectors"
  | "wasm_module_runtime"
  | "sidecar_runtime";

/**
 * Effective gate status.
 *
 * `not_compiled` and `runtime_disabled` are separate members on purpose. Collapsing them would
 * let the UI offer a toggle for a gate no amount of toggling can reach — the code is not in the
 * build, and only a different build changes that.
 */
export type FeatureGateStatus =
  | { kind: "not_compiled" }
  | { kind: "runtime_disabled" }
  | { kind: "enabled" }
  | { kind: "blocked_by_prerequisite"; reason: string }
  | { kind: "forced_disabled"; reason: string };

export interface FeatureGate {
  feature: ExtensionPlatformFeature;
  status: FeatureGateStatus;
  buildAvailable: boolean;
  desiredEnabled: boolean;
  revision: number;
  updatedAt: string | null;
  updatedBy: string | null;
  reason: string | null;
}

/**
 * Whether the reported gates came from a successful read.
 *
 * Independent of each gate's own status: a degraded overview still answers every gate, it just
 * says the answer may be stale (`reload_failed`) or a fail-closed default (`never_loaded`).
 */
export type FeatureGateFreshness =
  | { kind: "current" }
  | { kind: "degraded"; degradation: "never_loaded" | "reload_failed" };

export interface FeatureGateOverview {
  gates: FeatureGate[];
  freshness: FeatureGateFreshness;
}

export interface SetFeatureGateRequest {
  feature: ExtensionPlatformFeature;
  desiredEnabled: boolean;
  /** The revision last observed. A mismatch is rejected rather than overwriting a concurrent edit. */
  expectedRevision: number;
  reason?: string | null;
}
