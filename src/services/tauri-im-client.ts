import { invoke } from "@tauri-apps/api/core";
import {
  imConnectorConfigSchema,
  imRoutingSchema,
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
