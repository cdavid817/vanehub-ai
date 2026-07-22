import { invoke } from "@tauri-apps/api/core";
import type { SshConnectionService } from "./ssh-connection-service";
import type {
  SaveSshConnectionInput,
  SshConnection,
  SshConnectionTestResult,
} from "../types/ssh-connection";

export const tauriSshConnectionClient: SshConnectionService = {
  listConnections() {
    return invoke<SshConnection[]>("list_ssh_connections");
  },

  createConnection(input: SaveSshConnectionInput) {
    return invoke<SshConnection>("create_ssh_connection", { input });
  },

  updateConnection(connectionId: string, input: SaveSshConnectionInput) {
    return invoke<SshConnection>("update_ssh_connection", {
      connectionId,
      input,
    });
  },

  async deleteConnection(connectionId: string) {
    await invoke<void>("delete_ssh_connection", { connectionId });
  },

  testConnection(connectionId: string) {
    return invoke<SshConnectionTestResult>("test_ssh_connection", {
      connectionId,
    });
  },
};
