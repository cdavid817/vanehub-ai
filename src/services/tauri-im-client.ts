import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  imConnectorConfigSchema,
  imConnectorHealthSchema,
  imPairingStartSchema,
  imRoutingSchema,
  imSessionBindingSchema,
  imSessionBindingViewSchema,
  imSessionAccessSchema,
  parseImConnectorViews,
  parseImRouting,
  parseWeChatAuthorization,
  type ImConnectorKind,
  type ImRouting,
  type SaveImConnectorInput,
} from "../contracts/im";
import type { ImService } from "./im-service";

export const tauriImClient: ImService = {
  async listConnectors() {
    return parseImConnectorViews(await invoke<unknown>("list_im_connectors"));
  },
  async getRouting() {
    return parseImRouting(await invoke<unknown>("get_im_routing"));
  },
  async saveRouting(routing: ImRouting) {
    return imRoutingSchema.parse(await invoke<unknown>("save_im_routing", { routing }));
  },
  async saveConnector(input: SaveImConnectorInput) {
    return imConnectorConfigSchema.parse(await invoke<unknown>("save_im_connector", { input }));
  },
  setConnectorEnabled(kind: ImConnectorKind, enabled: boolean) {
    return invoke<void>("set_im_connector_enabled", { kind, enabled });
  },
  restartConnector(kind: ImConnectorKind) {
    return invoke<void>("restart_im_connector", { kind });
  },
  testConnector(kind: ImConnectorKind) {
    return invoke<void>("test_im_connector", { kind });
  },
  clearConnector(kind: ImConnectorKind) {
    return invoke<void>("clear_im_connector", { kind });
  },
  resetBindings(kind?: ImConnectorKind) {
    return invoke<void>("reset_im_bindings", { kind: kind ?? null });
  },
  async getSessionBinding(sessionId, connector) {
    const raw = await invoke<unknown>("get_im_session_binding", { sessionId, connector });
    const view = imSessionBindingViewSchema.parse(raw);
    const expectedConnector = view.binding?.connector ?? connector;
    if (view.access.connector !== expectedConnector) throw new Error("im-session-access-connector-mismatch");
    return view;
  },
  async setSessionAccess(sessionId, connector, enabled) {
    return imSessionAccessSchema.parse(
      await invoke<unknown>("set_im_session_access", { sessionId, connector, enabled }),
    );
  },
  async beginPairing(sessionId, connector, replaceExisting = false) {
    return imPairingStartSchema.parse(
      await invoke<unknown>("begin_im_pairing", { sessionId, connector, replaceExisting }),
    );
  },
  cancelPairing(sessionId, connector) {
    return invoke<boolean>("cancel_im_pairing", { sessionId, connector });
  },
  async setBindingPaused(sessionId, paused) {
    return imSessionBindingSchema.parse(
      await invoke<unknown>("set_im_binding_paused", { sessionId, paused }),
    );
  },
  async setCompletionNotifications(sessionId, enabled) {
    return imSessionBindingSchema.parse(
      await invoke<unknown>("set_im_completion_notifications", { sessionId, enabled }),
    );
  },
  removeSessionBinding(sessionId) {
    return invoke<boolean>("remove_im_session_binding", { sessionId });
  },
  async subscribeLifecycle(handler) {
    return listen<unknown>("im-connector:lifecycle", (event) => {
      const health = imConnectorHealthSchema.safeParse(event.payload);
      if (health.success) handler(health.data);
    });
  },
  async beginWeChatAuthorization() {
    return parseWeChatAuthorization(await invoke<unknown>("begin_wechat_authorization"));
  },
  async pollWeChatAuthorization() {
    return parseWeChatAuthorization(await invoke<unknown>("poll_wechat_authorization"));
  },
  cancelWeChatAuthorization() {
    return invoke<void>("cancel_wechat_authorization");
  },
};
