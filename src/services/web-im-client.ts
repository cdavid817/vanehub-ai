import type {
  ImConnectorConfig,
  ImConnectorKind,
  ImConnectorView,
  ImRouting,
  SaveImConnectorInput,
  WeChatAuthorization,
} from "../contracts/im";
import type { ImService } from "./im-service";

const kinds: ImConnectorKind[] = ["feishu", "telegram", "dingtalk", "wecom", "weixin"];
const limits: Record<ImConnectorKind, number> = {
  feishu: 20_000,
  telegram: 4_096,
  dingtalk: 2_000,
  wecom: 2_000,
  weixin: 2_000,
};

let routing: ImRouting | null = null;
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
    routing = { ...nextRouting };
    return { ...routing };
  },

  async saveConnector(input: SaveImConnectorInput) {
    await mockMutationLatency();
    if (input.enabled && !routing) throw new Error("im-routing-required");
    const hasCredentials = Object.values(input.credentials ?? {}).some((value) => value.trim().length > 0);
    const config: ImConnectorConfig = {
      kind: input.kind,
      enabled: input.enabled,
      displayName: input.displayName ?? null,
      publicConfig: { ...input.publicConfig },
      credentialRef: hasCredentials || connectorState[input.kind].hasCredentials ? `mock://${input.kind}/credential` : null,
    };
    update(input.kind, (view) => ({
      ...view,
      config,
      hasCredentials: hasCredentials || view.hasCredentials,
      health: {
        ...view.health,
        lifecycle: input.enabled ? "connected" : (hasCredentials || view.hasCredentials ? "disabled" : "unconfigured"),
        generation: view.health.generation + (input.enabled ? 1 : 0),
        safeErrorCode: null,
        updatedAt: new Date().toISOString(),
      },
    }));
    return { ...config, publicConfig: { ...config.publicConfig } };
  },

  async setConnectorEnabled(kind, enabled) {
    if (enabled && !routing) throw new Error("im-routing-required");
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

function mockAuthorization(status: WeChatAuthorization["status"], includeImage: boolean): WeChatAuthorization {
  const mockQr = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 21 21"><rect width="21" height="21" fill="white"/><g fill="black"><path d="M1 1h7v7H1zm2 2v3h3V3zm10-2h7v7h-7zm2 2v3h3V3zM1 13h7v7H1zm2 2v3h3v-3z" fill-rule="evenodd"/><path d="M10 2h2v2h-2zm0 4h3v2h-3zm-1 3h2v3H9zm4 0h2v2h-2zm3 1h4v2h-4zm-5 3h3v2h-3zm5 0h2v3h-2zm-7 3h2v4H9zm4 1h2v3h-2zm4 1h3v2h-3z"/></g></svg>`;
  return {
    status,
    imageDataUrl: includeImage
      ? `data:image/svg+xml,${encodeURIComponent(mockQr)}`
      : null,
    expiresAt: includeImage ? new Date(Date.now() + 300_000).toISOString() : null,
    safeErrorCode: null,
  };
}

export function getWebImDebugSnapshot(): string {
  return JSON.stringify({ routing, connectorState, authorizationActive, authorizationPoll });
}

export function resetWebImMock(): void {
  routing = null;
  authorizationPoll = 0;
  authorizationActive = false;
  connectorState = Object.fromEntries(
    kinds.map((kind) => [kind, { ...connectorState[kind], config: { ...connectorState[kind].config, enabled: false, credentialRef: null }, health: { ...connectorState[kind].health, lifecycle: "unconfigured", generation: 0, updatedAt: new Date().toISOString() }, hasCredentials: false }]),
  ) as Record<ImConnectorKind, ImConnectorView>;
}
