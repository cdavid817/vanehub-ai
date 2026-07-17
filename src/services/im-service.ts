import type {
  ImConnectorConfig,
  ImConnectorKind,
  ImConnectorView,
  ImRouting,
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
  beginWeChatAuthorization(): Promise<WeChatAuthorization>;
  pollWeChatAuthorization(): Promise<WeChatAuthorization>;
  cancelWeChatAuthorization(): Promise<void>;
}
