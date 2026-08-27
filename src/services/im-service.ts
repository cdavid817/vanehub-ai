import type {
  ImConnectorConfig,
  ImConnectorKind,
  ImConnectorView,
  ImPairingStart,
  ImRouting,
  ImSessionBinding,
  ImSessionBindingView,
  ImSessionAccess,
  SaveImConnectorInput,
  WeChatAuthorization,
} from "../contracts/im";
export interface ImService {
  listConnectors(): Promise<ImConnectorView[]>;
  getRouting(): Promise<ImRouting | null>;
  saveRouting(routing: ImRouting): Promise<ImRouting>;
  saveConnector(input: SaveImConnectorInput): Promise<ImConnectorConfig>;
  setConnectorEnabled(kind: ImConnectorKind, enabled: boolean): Promise<void>;
  restartConnector(kind: ImConnectorKind): Promise<void>;
  testConnector(kind: ImConnectorKind): Promise<void>;
  clearConnector(kind: ImConnectorKind): Promise<void>;
  resetBindings(kind?: ImConnectorKind): Promise<void>;
  getSessionBinding(sessionId: string): Promise<ImSessionBindingView>;
  setSessionAccess(sessionId: string, connector: ImConnectorKind, enabled: boolean): Promise<ImSessionAccess>;
  beginPairing(sessionId: string, connector: ImConnectorKind, replaceExisting?: boolean): Promise<ImPairingStart>;
  cancelPairing(sessionId: string, connector: ImConnectorKind): Promise<boolean>;
  setBindingPaused(sessionId: string, paused: boolean): Promise<ImSessionBinding>;
  setCompletionNotifications(sessionId: string, enabled: boolean): Promise<ImSessionBinding>;
  removeSessionBinding(sessionId: string): Promise<boolean>;
  subscribeLifecycle(handler: (health: import("../contracts/im").ImConnectorHealth) => void): Promise<() => void>;
  beginWeChatAuthorization(): Promise<WeChatAuthorization>;
  pollWeChatAuthorization(): Promise<WeChatAuthorization>;
  cancelWeChatAuthorization(): Promise<void>;
}
