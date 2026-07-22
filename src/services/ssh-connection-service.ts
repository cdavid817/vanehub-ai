import type {
  SaveSshConnectionInput,
  SshConnection,
  SshConnectionTestResult,
} from "../types/ssh-connection";

export interface SshConnectionService {
  listConnections(): Promise<SshConnection[]>;
  createConnection(input: SaveSshConnectionInput): Promise<SshConnection>;
  updateConnection(
    connectionId: string,
    input: SaveSshConnectionInput,
  ): Promise<SshConnection>;
  deleteConnection(connectionId: string): Promise<void>;
  testConnection(connectionId: string): Promise<SshConnectionTestResult>;
}
