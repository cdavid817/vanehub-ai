import type { SshConnectionService } from "./ssh-connection-service";
import type {
  SaveSshConnectionInput,
  SshConnection,
  SshConnectionTestResult,
} from "../types/ssh-connection";

let nextConnectionId = 1;
let connections: SshConnection[] = [];
const passwordPresence = new Set<string>();

export function findWebSshConnection(connectionId: string): SshConnection | null {
  const connection = connections.find((candidate) => candidate.id === connectionId);
  return connection ? { ...connection } : null;
}

function nowIso() {
  return new Date().toISOString();
}

function validate(input: SaveSshConnectionInput, previous?: SshConnection) {
  if (!input.name.trim())
    throw new Error("SSH connection name cannot be empty.");
  if (!input.host.trim()) throw new Error("SSH host cannot be empty.");
  if (!Number.isInteger(input.port) || input.port < 1 || input.port > 65535) {
    throw new Error("SSH port is invalid.");
  }
  if (!input.user.trim()) throw new Error("SSH user cannot be empty.");
  if (!input.defaultPath.trim())
    throw new Error("SSH default path cannot be empty.");
  if (input.authMode === "key" && !input.keyPath?.trim()) {
    throw new Error("SSH key path cannot be empty.");
  }
  if (
    input.authMode === "password" &&
    !input.password?.trim() &&
    !previous?.hasPassword
  ) {
    throw new Error("SSH password is required for password authentication.");
  }
}

function toConnection(
  id: string,
  input: SaveSshConnectionInput,
  createdAt: string,
  previous?: SshConnection,
): SshConnection {
  validate(input, previous);
  const timestamp = nowIso();
  const endpointChanged =
    previous !== undefined &&
    (previous.host !== input.host.trim() || previous.port !== input.port);
  const compatibilityChanged =
    previous !== undefined &&
    (endpointChanged ||
      previous.user !== input.user.trim() ||
      previous.authMode !== input.authMode ||
      (previous.keyPath ?? "") !== (input.keyPath?.trim() ?? "") ||
      Boolean(input.password?.trim()));
  if (input.authMode === "password" && input.password?.trim())
    passwordPresence.add(id);
  if (input.authMode === "key") passwordPresence.delete(id);
  return {
    id,
    name: input.name.trim(),
    host: input.host.trim(),
    port: input.port,
    user: input.user.trim(),
    defaultPath: input.defaultPath.trim(),
    authMode: input.authMode,
    keyPath: input.authMode === "key" ? input.keyPath?.trim() || null : null,
    hasPassword:
      input.authMode === "password" &&
      (passwordPresence.has(id) || Boolean(previous?.hasPassword)),
    revision:
      previous === undefined
        ? 1
        : previous.revision + (compatibilityChanged ? 1 : 0),
    hostTrust: endpointChanged ? null : (previous?.hostTrust ?? null),
    testStatus: previous?.testStatus ?? "not-tested",
    lastConnectedAt: previous?.lastConnectedAt ?? null,
    lastError: previous?.lastError ?? null,
    createdAt,
    updatedAt: timestamp,
  };
}

export const webSshConnectionClient: SshConnectionService = {
  async listConnections() {
    return connections.map((connection) => ({ ...connection }));
  },

  async createConnection(input) {
    const timestamp = nowIso();
    const connection = toConnection(
      `web-ssh-${nextConnectionId++}`,
      input,
      timestamp,
    );
    connections = [connection, ...connections];
    return { ...connection };
  },

  async updateConnection(connectionId, input) {
    const current = connections.find(
      (connection) => connection.id === connectionId,
    );
    if (!current) throw new Error(`SSH connection not found: ${connectionId}`);
    const updated = toConnection(
      connectionId,
      input,
      current.createdAt,
      current,
    );
    connections = connections.map((connection) =>
      connection.id === connectionId ? updated : connection,
    );
    return { ...updated };
  },

  async deleteConnection(connectionId) {
    connections = connections.filter(
      (connection) => connection.id !== connectionId,
    );
    passwordPresence.delete(connectionId);
  },

  async testConnection(connectionId): Promise<SshConnectionTestResult> {
    const current = connections.find(
      (connection) => connection.id === connectionId,
    );
    if (!current) throw new Error(`SSH connection not found: ${connectionId}`);
    if (current.authMode === "password" && !current.hasPassword) {
      throw new Error("SSH password is required for password authentication.");
    }
    const testedAt = nowIso();
    const updated: SshConnection = {
      ...current,
      testStatus: "succeeded",
      lastConnectedAt: testedAt,
      lastError: null,
      updatedAt: testedAt,
    };
    connections = connections.map((connection) =>
      connection.id === connectionId ? updated : connection,
    );
    return {
      status: "succeeded",
      message: "Web mock SSH probe succeeded.",
      testedAt,
    };
  },
};
