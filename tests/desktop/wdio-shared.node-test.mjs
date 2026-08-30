import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { createServer } from "node:net";
import test from "node:test";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";
import { setTimeout as schedule } from "node:timers";
import {
  closeDesktopSession,
  createDesktopConfig,
  createEmbeddedDriverShutdownWaiter,
} from "./wdio-shared.mjs";

test("closes WebDriver before waiting for the native application exit", async () => {
  const events = [];
  const browser = {
    sessionId: "desktop-session",
    tauri: {
      execute: async () => events.push("request-exit"),
    },
    deleteSession: async () => events.push("delete-session"),
  };

  await closeDesktopSession(browser, async () => events.push("wait-for-exit"));

  assert.deepEqual(events, ["request-exit", "delete-session", "wait-for-exit"]);
  assert.equal(browser.sessionId, undefined);
});

test("leaves WebDriver teardown to the runner when the application already exited", async () => {
  let deleted = false;
  const browser = {
    sessionId: "desktop-session",
    tauri: {
      execute: async () => {
        throw new Error("application unavailable");
      },
    },
    deleteSession: async () => {
      deleted = true;
    },
  };

  await closeDesktopSession(browser, async () => assert.fail("must not wait without an exit request"));

  assert.equal(deleted, false);
  assert.equal(browser.sessionId, "desktop-session");
});

test("waits for a draining embedded driver before the next worker starts", async () => {
  const resultDir = await mkdtemp(path.join(tmpdir(), "vanehub-wdio-shared-"));
  const server = createServer();
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert.notEqual(address, null);
  assert.equal(typeof address, "object");
  const previousArtifact = process.env.VANEHUB_DESKTOP_ARTIFACT;
  const previousResultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR;
  const previousPort = process.env.VANEHUB_WEBDRIVER_PORT;
  process.env.VANEHUB_DESKTOP_ARTIFACT = "/tmp/vanehub-desktop-test-artifact";
  process.env.VANEHUB_DESKTOP_RESULT_DIR = resultDir;
  process.env.VANEHUB_WEBDRIVER_PORT = String(address.port);

  try {
    const config = await createDesktopConfig({ specDirectory: "specs", specFiles: ["smoke.e2e.mjs"] });
    await config.onWorkerStart();
    const startedAt = Date.now();
    schedule(() => server.close(), 150);
    await config.onWorkerStart();
    assert.ok(Date.now() - startedAt >= 2_100);
    assert.ok(Date.now() - startedAt < 4_000);
  } finally {
    server.close();
    if (previousArtifact === undefined) delete process.env.VANEHUB_DESKTOP_ARTIFACT;
    else process.env.VANEHUB_DESKTOP_ARTIFACT = previousArtifact;
    if (previousResultDir === undefined) delete process.env.VANEHUB_DESKTOP_RESULT_DIR;
    else process.env.VANEHUB_DESKTOP_RESULT_DIR = previousResultDir;
    if (previousPort === undefined) delete process.env.VANEHUB_WEBDRIVER_PORT;
    else process.env.VANEHUB_WEBDRIVER_PORT = previousPort;
  }
});

test("stops the previous test-owned process before probing its driver port", async () => {
  const events = [];
  const portStates = [true, false, false];
  const waitForShutdown = createEmbeddedDriverShutdownWaiter(4567, {
    delay: async () => events.push("delay"),
    isPortOpen: async () => {
      events.push("probe-port");
      return portStates.shift() ?? false;
    },
    processReapMs: 0,
    shutdownTimeoutMs: 100,
    stopOwnedProcess: async () => events.push("stop-owned-process"),
  });

  await waitForShutdown();
  assert.deepEqual(events, []);
  await waitForShutdown();

  assert.deepEqual(events, [
    "stop-owned-process",
    "probe-port",
    "delay",
    "probe-port",
    "probe-port",
    "delay",
  ]);
});

test("attributes an unreleased port to the desktop driver lifecycle", async () => {
  const waitForShutdown = createEmbeddedDriverShutdownWaiter(4568, {
    delay: async () => {},
    isPortOpen: async () => true,
    processReapMs: 0,
    shutdownTimeoutMs: 0,
    stopOwnedProcess: async () => {},
  });

  await waitForShutdown();
  await assert.rejects(
    waitForShutdown(),
    /Desktop driver lifecycle failure: test-owned WebDriver port 4568 remained open/,
  );
});
