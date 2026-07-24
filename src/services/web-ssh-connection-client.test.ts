import { describe, expect, it } from "vitest";
import { webSshConnectionClient } from "./web-ssh-connection-client";

describe("webSshConnectionClient", () => {
  it("simulates credential presence without returning plaintext passwords", async () => {
    const connection = await webSshConnectionClient.createConnection({
      name: "Dev",
      host: "dev.example.test",
      port: 2222,
      user: "dev",
      defaultPath: "/work/app",
      authMode: "password",
      password: "secret-password",
    });

    expect(connection).toMatchObject({
      host: "dev.example.test",
      port: 2222,
      hasPassword: true,
      revision: 1,
      hostTrust: null,
    });
    expect(JSON.stringify(connection)).not.toContain("secret-password");

    const result = await webSshConnectionClient.testConnection(connection.id);

    expect(result.status).toBe("succeeded");
  });

  it("increments revisions only for connection-compatible changes", async () => {
    const created = await webSshConnectionClient.createConnection({
      name: "Original",
      host: "revision.example.test",
      port: 22,
      user: "dev",
      defaultPath: "/work",
      authMode: "key",
      keyPath: "/keys/dev",
    });
    const renamed = await webSshConnectionClient.updateConnection(created.id, {
      name: "Renamed",
      host: created.host,
      port: created.port,
      user: created.user,
      defaultPath: created.defaultPath,
      authMode: created.authMode,
      keyPath: created.keyPath,
    });
    const endpointChanged = await webSshConnectionClient.updateConnection(
      created.id,
      {
        name: renamed.name,
        host: "replacement.example.test",
        port: renamed.port,
        user: renamed.user,
        defaultPath: renamed.defaultPath,
        authMode: renamed.authMode,
        keyPath: renamed.keyPath,
      },
    );

    expect(renamed.revision).toBe(1);
    expect(endpointChanged.revision).toBe(2);
    expect(endpointChanged.hostTrust).toBeNull();
  });
});
