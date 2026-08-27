import {
  imConnectorFields,
  type ImConnectorConfig,
  type ImConnectorKind,
  type ImConnectorView,
  type ImPairingStart,
  type ImRouting,
  type ImSessionBinding,
  type ImSessionAccess,
  type SaveImConnectorInput,
} from "../contracts/im";
import type { ImService } from "./im-service";
import { compactFieldPatch, connectorFieldMaps, mockAuthorization } from "./web-im-client-helpers";
import { getSessionAccess, mutateBinding } from "./web-im-session-state";

const kinds: ImConnectorKind[] = ["feishu", "telegram", "dingtalk", "wecom", "weixin"];
const limits: Record<ImConnectorKind, number> = {
  feishu: 20_000,
  telegram: 4_096,
  dingtalk: 2_000,
  wecom: 2_000,
  weixin: 2_000,
};

let routing: ImRouting | null = null;
let pairingSequence = 0;
let pairings = new Map<string, ImPairingStart>();
let pairingTimers = new Map<string, ReturnType<typeof globalThis.setTimeout>>();
let sessionBindings = new Map<string, ImSessionBinding>();
let sessionAccess = new Map<string, ImSessionAccess>();
let connectorState: Record<ImConnectorKind, ImConnectorView> = Object.fromEntries(
  kinds.map((kind) => [
    kind,
    {
      descriptor: {
        kind,
        supportsQrAuthorization: kind === "weixin",
        experimental: kind === "weixin",
        maxOutboundChars: limits[kind],
      },
      config: {
        kind,
        enabled: false,
        displayName: null,
        publicConfig: {},
        credentialRef: null,
      },
      health: {
        kind,
        lifecycle: "unconfigured",
        generation: 0,
        safeErrorCode: null,
        updatedAt: new Date().toISOString(),
      },
      hasCredentials: false,
    } satisfies ImConnectorView,
  ]),
) as Record<ImConnectorKind, ImConnectorView>;

let authorizationPoll = 0;
let authorizationActive = false;
const lifecycleSubscribers = new Set<(health: ImConnectorView["health"]) => void>();

function cloneView(view: ImConnectorView): ImConnectorView {
  return {
    ...view,
    descriptor: { ...view.descriptor },
    config: { ...view.config, publicConfig: { ...view.config.publicConfig } },
    health: { ...view.health },
  };
}

function update(kind: ImConnectorKind, mutate: (view: ImConnectorView) => ImConnectorView): void {
  connectorState = { ...connectorState, [kind]: mutate(connectorState[kind]) };
  const health = { ...connectorState[kind].health };
  lifecycleSubscribers.forEach((subscriber) => subscriber(health));
}

function mockMutationLatency(): Promise<void> {
  return new Promise((resolve) => globalThis.setTimeout(resolve, 120));
}

export const webImClient: ImService = {
  async listConnectors() {
    return kinds.map((kind) => cloneView(connectorState[kind]));
  },
  async getRouting() {
    return routing ? { ...routing } : null;
  },
  async saveRouting(nextRouting) {
    routing = { agentId: nextRouting.agentId.trim(), projectPath: nextRouting.projectPath.trim() };
    return { ...routing };
  },
  async saveConnector(input: SaveImConnectorInput) {
    await mockMutationLatency();
    const patch = compactFieldPatch(input.kind, input.credentials);
    const { publicFields, secretFields } = connectorFieldMaps(input.kind);
    const hasSecretPatch = Object.entries(patch).some(([key]) => secretFields.has(key));
    const publicConfig = { ...input.publicConfig };
    for (const [key, value] of Object.entries(patch)) {
      if (publicFields.has(key)) publicConfig[key] = value;
    }
    const hasCredentials = hasSecretPatch || connectorState[input.kind].hasCredentials;
    const complete = imConnectorFields[input.kind]
      .filter((field) => field.required)
      .every((field) => field.secret
        ? hasCredentials
        : typeof publicConfig[field.key] === "string" && String(publicConfig[field.key]).trim().length > 0);
    if (!complete) throw new Error("connector-credentials-incomplete");
    const config: ImConnectorConfig = {
      kind: input.kind,
      enabled: input.enabled,
      displayName: input.displayName ?? null,
      publicConfig,
      credentialRef: hasCredentials ? `mock://${input.kind}/credential` : null,
    };
    update(input.kind, (view) => ({
      ...view,
      config,
      hasCredentials,
      health: {
        ...view.health,
        lifecycle: input.enabled ? "connected" : (hasCredentials ? "disabled" : "unconfigured"),
        generation: view.health.generation + (input.enabled ? 1 : 0),
        safeErrorCode: null,
        updatedAt: new Date().toISOString(),
      },
    }));
    return { ...config, publicConfig: { ...config.publicConfig } };
  },
  async setConnectorEnabled(kind, enabled) {
    if (enabled && !connectorState[kind].hasCredentials) {
      throw new Error("connector-credentials-required");
    }
    update(kind, (view) => ({
      ...view,
      config: { ...view.config, enabled },
      health: {
        ...view.health,
        lifecycle: enabled ? "connected" : "disabled",
        generation: view.health.generation + (enabled ? 1 : 0),
        safeErrorCode: null,
        updatedAt: new Date().toISOString(),
      },
    }));
  },
  async restartConnector(kind) {
    if (!connectorState[kind].config.enabled) return;
    update(kind, (view) => ({
      ...view,
      health: { ...view.health, lifecycle: "connected", generation: view.health.generation + 1, updatedAt: new Date().toISOString() },
    }));
  },
  async testConnector(kind) {
    if (!connectorState[kind].hasCredentials) throw new Error("connector-credentials-required");
  },
  async clearConnector(kind) {
    update(kind, (view) => ({
      ...view,
      hasCredentials: false,
      config: { ...view.config, enabled: false, credentialRef: null },
      health: { ...view.health, lifecycle: "unconfigured", safeErrorCode: null, updatedAt: new Date().toISOString() },
    }));
  },
  async resetBindings() {},
  async getSessionBinding(sessionId) {
    return { binding: sessionBindings.get(sessionId) ?? null,
      pendingConnector: pairings.get(sessionId)?.connector ?? null,
      access: getSessionAccess(sessionAccess, sessionId, "feishu") };
  },
  async setSessionAccess(sessionId, connector, enabled) {
    const access: ImSessionAccess = { sessionId, connector, enabled, updatedAt: new Date().toISOString() };
    sessionAccess.set(`${sessionId}\u0000${connector}`, access);
    return { ...access };
  },
  async beginPairing(sessionId, connector, replaceExisting = false) {
    if (connector === "feishu" && !getSessionAccess(sessionAccess, sessionId, connector).enabled) throw new Error("im-session-disabled");
    const view = connectorState[connector];
    if (!view.config.enabled || view.health.lifecycle !== "connected") {
      throw new Error("im-connector-not-ready");
    }
    pairingSequence += 1;
    const pairing: ImPairingStart = {
      connector,
      sessionId,
      code: pairingSequence.toString(32).toUpperCase().padStart(8, "2").slice(-8),
      expiresAt: new Date(Date.now() + 600_000).toISOString(),
      replaceExisting,
    };
    pairings.set(sessionId, pairing);
    const previousTimer = pairingTimers.get(sessionId);
    if (previousTimer) globalThis.clearTimeout(previousTimer);
    pairingTimers.set(sessionId, globalThis.setTimeout(() => {
      if (pairings.get(sessionId)?.code !== pairing.code) return;
      const now = new Date().toISOString();
      sessionBindings.set(sessionId, {
        completionNotifications: false,
        connector,
        createdAt: now,
        sessionId,
        state: "active",
        updatedAt: now,
      });
      pairings.delete(sessionId);
      pairingTimers.delete(sessionId);
    }, 500));
    return { ...pairing };
  },
  async cancelPairing(sessionId, connector) {
    if (pairings.get(sessionId)?.connector !== connector) return false;
    const timer = pairingTimers.get(sessionId);
    if (timer) globalThis.clearTimeout(timer);
    pairingTimers.delete(sessionId);
    return pairings.delete(sessionId);
  },
  async setBindingPaused(sessionId, paused) {
    return mutateBinding(sessionBindings, sessionId, (binding) => (
      { ...binding, state: paused ? "paused" : "active" }
    ));
  },
  async setCompletionNotifications(sessionId, enabled) {
    return mutateBinding(sessionBindings, sessionId, (binding) => (
      { ...binding, completionNotifications: enabled }
    ));
  },
  async removeSessionBinding(sessionId) {
    return sessionBindings.delete(sessionId);
  },

  async subscribeLifecycle(handler) {
    lifecycleSubscribers.add(handler);
    return () => lifecycleSubscribers.delete(handler);
  },

  async beginWeChatAuthorization() {
    authorizationActive = true;
    authorizationPoll = 0;
    return mockAuthorization("waiting", true);
  },

  async pollWeChatAuthorization() {
    if (!authorizationActive) throw new Error("wechat-authorization-not-started");
    authorizationPoll += 1;
    if (authorizationPoll === 1) return mockAuthorization("scanned", true);
    authorizationActive = false;
    update("weixin", (view) => ({
      ...view,
      hasCredentials: true,
      config: { ...view.config, credentialRef: "mock://weixin/credential" },
      health: { ...view.health, lifecycle: "disabled", updatedAt: new Date().toISOString() },
    }));
    return mockAuthorization("confirmed", false);
  },

  async cancelWeChatAuthorization() {
    authorizationActive = false;
  },
};

export function getWebImDebugSnapshot(): string {
  return JSON.stringify({ routing, connectorState, authorizationActive, authorizationPoll });
}

export function resetWebImMock(): void {
  routing = null;
  authorizationPoll = 0;
  authorizationActive = false;
  pairingSequence = 0;
  pairingTimers.forEach((timer) => globalThis.clearTimeout(timer));
  pairingTimers = new Map();
  pairings = new Map();
  sessionBindings = new Map();
  sessionAccess = new Map();
  connectorState = Object.fromEntries(
    kinds.map((kind) => [kind, { ...connectorState[kind], config: { ...connectorState[kind].config, enabled: false, credentialRef: null }, health: { ...connectorState[kind].health, lifecycle: "unconfigured", generation: 0, updatedAt: new Date().toISOString() }, hasCredentials: false }]),
  ) as Record<ImConnectorKind, ImConnectorView>;
}
