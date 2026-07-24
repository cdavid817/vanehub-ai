import assert from "node:assert/strict";
import { createServer } from "node:net";
import test from "node:test";
import {
  allocateScreenshotPort,
  parseRequestedPort,
} from "./docs-screenshot-port.mjs";

test("allocates an available loopback port when none is configured", async () => {
  const port = await allocateScreenshotPort();
  assert.ok(Number.isInteger(port));
  assert.ok(port > 0 && port <= 65_535);
});

test("rejects invalid configured ports", () => {
  assert.throws(() => parseRequestedPort("not-a-port"), /must be an integer/);
  assert.throws(() => parseRequestedPort("65536"), /between 1 and 65535/);
});

test("reports an unavailable configured port without reusing its server", async () => {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  assert.ok(address && typeof address !== "string");

  try {
    await assert.rejects(
      allocateScreenshotPort(String(address.port)),
      new RegExp(`loopback port ${address.port}`),
    );
  } finally {
    await new Promise((resolve, reject) => {
      server.close((error) => {
        if (error) reject(error);
        else resolve();
      });
    });
  }
});
