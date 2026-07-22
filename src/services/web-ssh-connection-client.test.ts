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
    });
    expect(JSON.stringify(connection)).not.toContain("secret-password");

    const result = await webSshConnectionClient.testConnection(connection.id);

    expect(result.status).toBe("succeeded");
  });
});
