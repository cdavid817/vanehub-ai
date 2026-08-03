const assert = require("node:assert/strict");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const readline = require("node:readline");
const { once } = require("node:events");
const { spawn } = require("node:child_process");
const {
  FIXTURE_MODES,
  LIMITS,
  limitPlusOneFixtures,
  rpcResponseAtBytes,
  toolCallResult,
  toolList,
} = require("./mcp_fixture_data.cjs");

const fixtures = __dirname;
const probe = process.argv[2];

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function nestedDepth(value) {
  let depth = 1;
  let current = value;
  while (current && typeof current === "object" && Object.prototype.hasOwnProperty.call(current, "nested")) {
    depth += 1;
    current = current.nested;
  }
  return depth;
}

function withTimeout(promise, milliseconds, label) {
  return Promise.race([
    promise,
    delay(milliseconds).then(() => { throw new Error(`${label} timed out`); }),
  ]);
}

function createLineReader(stream) {
  const lines = [];
  const waiting = [];
  const rl = readline.createInterface({ input: stream, crlfDelay: Infinity });
  rl.on("line", (line) => {
    const resolve = waiting.shift();
    if (resolve) resolve(line);
    else lines.push(line);
  });
  return {
    next(label = "fixture line") {
      if (lines.length) return Promise.resolve(lines.shift());
      return withTimeout(new Promise((resolve) => waiting.push(resolve)), 2000, label);
    },
    close() { rl.close(); },
  };
}

async function stopChild(child) {
  if (child.exitCode !== null) return;
  child.kill();
  await Promise.race([once(child, "exit"), delay(1000)]);
  if (child.exitCode === null) child.kill();
}

function startStdio(mode = "normal", env = {}) {
  const child = spawn(process.execPath, [path.join(fixtures, "mcp_stdio_server.cjs"), mode], {
    env: { ...process.env, ...env },
    stdio: ["pipe", "pipe", "pipe"],
    windowsHide: true,
  });
  const stderrChunks = [];
  child.stderr.on("data", (chunk) => stderrChunks.push(chunk));
  return { child, lines: createLineReader(child.stdout), stderrChunks };
}

function sendStdio(child, id, method, params) {
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
}

async function startHttpFixture(script, mode = "normal") {
  const child = spawn(process.execPath, [path.join(fixtures, script), "0", mode], {
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  const lines = createLineReader(child.stdout);
  const ready = await lines.next("HTTP fixture ready line");
  assert.match(ready, /^READY http:\/\/127\.0\.0\.1:\d+\//);
  return { child, lines, url: ready.slice("READY ".length) };
}

async function postJson(url, message, options = {}) {
  return fetch(url, {
    method: "POST",
    body: JSON.stringify(message),
    headers: { "content-type": "application/json", ...(options.headers || {}) },
    redirect: options.redirect || "manual",
    signal: options.signal,
  });
}

async function probeStdio() {
  const normal = startStdio();
  try {
    sendStdio(normal.child, 1, "initialize", {});
    assert.equal(JSON.parse(await normal.lines.next()).result.serverInfo.name, "vanehub-fixture-stdio");
    sendStdio(normal.child, 2, "tools/list", {});
    assert.equal(JSON.parse(await normal.lines.next()).result.tools[0].name, "fixture_echo");
    sendStdio(normal.child, 3, "tools/call", { name: "fixture_echo", arguments: { text: "hi" } });
    assert.equal(JSON.parse(await normal.lines.next()).result.content[0].text, "echo: hi");
  } finally {
    await stopChild(normal.child);
  }

  const invalid = startStdio("invalid-frame");
  try {
    sendStdio(invalid.child, 1, "initialize", {});
    assert.equal(await invalid.lines.next(), "not-json");
  } finally {
    await stopChild(invalid.child);
  }

  const oversized = startStdio("frame-limit-plus-one");
  try {
    sendStdio(oversized.child, 1, "initialize", {});
    assert.equal(Buffer.byteLength(await oversized.lines.next()), LIMITS.protocolMessageBytes + 1);
  } finally {
    await stopChild(oversized.child);
  }

  const hanging = startStdio("hang-initialize");
  try {
    sendStdio(hanging.child, 1, "initialize", {});
    await assert.rejects(withTimeout(hanging.lines.next(), 100, "hang probe"), /timed out/);
  } finally {
    await stopChild(hanging.child);
  }

  const disconnected = startStdio("disconnect-initialize");
  try {
    sendStdio(disconnected.child, 1, "initialize", {});
    await withTimeout(once(disconnected.child, "exit"), 2000, "stdio disconnect");
    assert.equal(disconnected.child.exitCode, 0);
  } finally {
    await stopChild(disconnected.child);
  }

  const secret = startStdio("stderr-limit-plus-one");
  try {
    await withTimeout((async () => {
      while (secret.stderrChunks.length === 0) await delay(10);
    })(), 2000, "stderr fixture output");
    const stderr = Buffer.concat(secret.stderrChunks).toString("utf8");
    assert.match(stderr, /fixture-stderr-secret/);
    assert.ok(Buffer.byteLength(stderr) > LIMITS.stderrBytes);
  } finally {
    await stopChild(secret.child);
  }

  const pidFile = path.join(os.tmpdir(), `vanehub-mcp-descendant-${process.pid}-${Date.now()}.txt`);
  const descendant = startStdio("spawn-descendant", {
    VANEHUB_MCP_FIXTURE_DESCENDANT_PID_FILE: pidFile,
  });
  try {
    await withTimeout((async () => {
      while (!fs.existsSync(pidFile)) await delay(10);
    })(), 2000, "descendant pid");
    assert.ok(Number(fs.readFileSync(pidFile, "utf8")) > 0);
    sendStdio(descendant.child, 9, "fixture/shutdown", {});
    await descendant.lines.next("shutdown response");
    await withTimeout(once(descendant.child, "exit"), 2000, "descendant shutdown");
  } finally {
    await stopChild(descendant.child);
    fs.rmSync(pidFile, { force: true });
  }

  assert.equal(toolList("tools-limit-plus-one", "tool", "desc").tools.length, LIMITS.toolsPerServer + 1);
  assert.equal(toolList("tool-name-limit-plus-one", "tool", "desc").tools[0].name.length, LIMITS.toolNameBytes + 1);
  assert.equal(toolList("description-limit-plus-one", "tool", "desc").tools[0].description.length, LIMITS.toolDescriptionBytes + 1);
  assert.equal(Buffer.byteLength(JSON.stringify(toolList("schema-limit-plus-one", "tool", "desc").tools[0].inputSchema)), LIMITS.schemaBytes + 1);
  assert.ok(Buffer.byteLength(JSON.stringify(toolList("catalog-limit-plus-one", "tool", "desc"))) > LIMITS.protocolMessageBytes);
  assert.equal(toolCallResult("result-limit-plus-one", "echo", "").content[0].text.length, LIMITS.toolResultBytes + 1);
  assert.equal(Buffer.byteLength(rpcResponseAtBytes(1, LIMITS.protocolMessageBytes + 1)), LIMITS.protocolMessageBytes + 1);
  const boundaries = limitPlusOneFixtures();
  assert.equal(Buffer.byteLength(boundaries.importDocument), LIMITS.importDocumentBytes + 1);
  assert.equal(Object.keys(boundaries.importServers).length, LIMITS.importServerEntries + 1);
  assert.equal(boundaries.args.length, LIMITS.configurationCollectionEntries + 1);
  assert.equal(Object.keys(boundaries.env).length, LIMITS.configurationCollectionEntries + 1);
  assert.equal(Object.keys(boundaries.headers).length, LIMITS.configurationCollectionEntries + 1);
  assert.equal(Buffer.byteLength(JSON.stringify(boundaries.serializedConfiguration)), LIMITS.configurationSerializedBytes + 1);
  assert.equal(Buffer.byteLength(boundaries.protocolMessage), LIMITS.protocolMessageBytes + 1);
  assert.equal(boundaries.tools.length, LIMITS.toolsPerServer + 1);
  assert.equal(boundaries.providerTools.length, LIMITS.providerTools + 1);
  assert.equal(Buffer.byteLength(boundaries.toolName), LIMITS.toolNameBytes + 1);
  assert.equal(Buffer.byteLength(boundaries.toolDescription), LIMITS.toolDescriptionBytes + 1);
  assert.equal(Buffer.byteLength(JSON.stringify(boundaries.schema)), LIMITS.schemaBytes + 1);
  assert.equal(nestedDepth(boundaries.schemaDepth), LIMITS.jsonDepth + 1);
  assert.equal(Buffer.byteLength(JSON.stringify(boundaries.toolArguments)), LIMITS.toolArgumentsBytes + 1);
  assert.equal(nestedDepth(boundaries.toolArgumentDepth), LIMITS.jsonDepth + 1);
  assert.equal(Buffer.byteLength(boundaries.toolResult), LIMITS.toolResultBytes + 1);
  assert.equal(boundaries.stderr.length, LIMITS.stderrBytes + 1);
  for (const required of ["invalid-frame", "spawn-descendant", "stderr-secret", "hang-invocation"]) {
    assert.ok(FIXTURE_MODES.stdio.includes(required));
  }
}

async function probeStreamableHttp() {
  const normal = await startHttpFixture("mcp_http_server.cjs");
  try {
    const initialized = await postJson(normal.url, { jsonrpc: "2.0", id: 1, method: "initialize" });
    assert.equal(initialized.status, 200);
    assert.equal(initialized.headers.get("mcp-session-id"), "fixture-session");
    assert.equal((await initialized.json()).result.serverInfo.name, "vanehub-fixture-http");
    const headers = { "mcp-session-id": "fixture-session" };
    const notification = await postJson(normal.url, { jsonrpc: "2.0", method: "notifications/initialized" }, { headers });
    assert.equal(notification.status, 202);
    const listed = await postJson(normal.url, { jsonrpc: "2.0", id: 2, method: "tools/list" }, { headers });
    assert.equal((await listed.json()).result.tools[0].name, "fixture_http_echo");
    const called = await postJson(normal.url, { jsonrpc: "2.0", id: 3, method: "tools/call", params: { arguments: { text: "hi" } } }, { headers });
    assert.equal((await called.json()).result.content[0].text, "http echo: hi");
    const deleted = await fetch(normal.url, { method: "DELETE", headers });
    assert.equal(deleted.status, 202);
    const state = await fetch(new URL("/state", normal.url)).then((response) => response.json());
    assert.equal(state.deleteCount, 1);
    assert.ok(state.requests.some((request) => request.sessionId === "fixture-session"));
  } finally {
    await stopChild(normal.child);
  }

  const sse = await startHttpFixture("mcp_http_server.cjs", "sse-response");
  try {
    const response = await postJson(sse.url, { jsonrpc: "2.0", id: 1, method: "initialize" });
    assert.match(response.headers.get("content-type"), /text\/event-stream/);
    assert.match(await response.text(), /data: .*vanehub-fixture-http/);
  } finally {
    await stopChild(sse.child);
  }

  const redirect = await startHttpFixture("mcp_http_server.cjs", "redirect");
  try {
    const response = await postJson(redirect.url, { jsonrpc: "2.0", id: 1, method: "initialize" });
    assert.equal(response.status, 307);
  } finally {
    await stopChild(redirect.child);
  }

  const disconnected = await startHttpFixture("mcp_http_server.cjs", "disconnect");
  try {
    await assert.rejects(postJson(disconnected.url, { jsonrpc: "2.0", id: 1, method: "initialize" }));
  } finally {
    await stopChild(disconnected.child);
  }

  const hanging = await startHttpFixture("mcp_http_server.cjs", "hang-initialize");
  try {
    await assert.rejects(postJson(
      hanging.url,
      { jsonrpc: "2.0", id: 1, method: "initialize" },
      { signal: AbortSignal.timeout(100) },
    ));
  } finally {
    await stopChild(hanging.child);
  }

  for (const [mode, expected] of [
    ["invalid-frame", "not-json"],
    ["body-limit-plus-one", LIMITS.protocolMessageBytes + 1],
    ["sse-event-limit-plus-one", "sse-limit"],
  ]) {
    const fixture = await startHttpFixture("mcp_http_server.cjs", mode);
    try {
      const response = await postJson(fixture.url, { jsonrpc: "2.0", id: 1, method: "initialize" });
      const body = await response.text();
      if (expected === "not-json") assert.equal(body, expected);
      else if (expected === "sse-limit") assert.ok(Buffer.byteLength(body) > LIMITS.protocolMessageBytes);
      else assert.equal(Buffer.byteLength(body), expected);
    } finally {
      await stopChild(fixture.child);
    }
  }
  for (const required of ["notification-202", "hang-delete", "catalog-limit-plus-one"]) {
    assert.ok(FIXTURE_MODES.streamableHttp.includes(required));
  }
}

async function readSseUntil(reader, pattern, label, timeout = 3000) {
  let text = "";
  await withTimeout((async () => {
    while (!pattern.test(text)) {
      const { done, value } = await reader.read();
      if (done) throw new Error(`${label} disconnected`);
      text += Buffer.from(value).toString("utf8");
    }
  })(), timeout, label);
  return text;
}

async function openLegacyStream(url) {
  const response = await fetch(url, { redirect: "manual" });
  assert.equal(response.status, 200);
  const reader = response.body.getReader();
  await readSseUntil(reader, /event: endpoint[\s\S]*data: \/messages/, "legacy endpoint");
  return reader;
}

async function probeLegacySse() {
  const normal = await startHttpFixture("mcp_legacy_sse_server.cjs");
  try {
    const reader = await openLegacyStream(normal.url);
    const endpoint = new URL("/messages", normal.url);
    assert.equal((await postJson(endpoint, { jsonrpc: "2.0", id: 1, method: "initialize" })).status, 202);
    assert.match(await readSseUntil(reader, /vanehub-fixture-sse/, "legacy initialize"), /vanehub-fixture-sse/);
    await postJson(endpoint, { jsonrpc: "2.0", id: 2, method: "tools/list" });
    assert.match(await readSseUntil(reader, /fixture_sse_echo/, "legacy list"), /fixture_sse_echo/);
    await postJson(endpoint, { jsonrpc: "2.0", id: 3, method: "tools/call", params: { arguments: { text: "hi" } } });
    assert.match(await readSseUntil(reader, /sse echo: hi/, "legacy call"), /sse echo: hi/);
    await reader.cancel();
  } finally {
    await stopChild(normal.child);
  }

  const redirect = await startHttpFixture("mcp_legacy_sse_server.cjs", "redirect");
  try {
    const response = await fetch(redirect.url, { redirect: "manual" });
    assert.equal(response.status, 307);
  } finally {
    await stopChild(redirect.child);
  }

  const streamDisconnect = await startHttpFixture("mcp_legacy_sse_server.cjs", "disconnect-stream");
  try {
    await assert.rejects(fetch(streamDisconnect.url));
  } finally {
    await stopChild(streamDisconnect.child);
  }

  const messageDisconnect = await startHttpFixture("mcp_legacy_sse_server.cjs", "disconnect-message");
  try {
    const reader = await openLegacyStream(messageDisconnect.url);
    await assert.rejects(postJson(
      new URL("/messages", messageDisconnect.url),
      { jsonrpc: "2.0", id: 1, method: "initialize" },
    ));
    await reader.cancel();
  } finally {
    await stopChild(messageDisconnect.child);
  }

  const hanging = await startHttpFixture("mcp_legacy_sse_server.cjs", "hang-initialize");
  try {
    const reader = await openLegacyStream(hanging.url);
    await postJson(new URL("/messages", hanging.url), { jsonrpc: "2.0", id: 1, method: "initialize" });
    await assert.rejects(readSseUntil(reader, /never-arrives/, "legacy hang", 100), /timed out/);
    await reader.cancel();
  } finally {
    await stopChild(hanging.child);
  }

  const invalid = await startHttpFixture("mcp_legacy_sse_server.cjs", "invalid-frame");
  try {
    const reader = await openLegacyStream(invalid.url);
    await postJson(new URL("/messages", invalid.url), { jsonrpc: "2.0", id: 1, method: "initialize" });
    assert.match(await readSseUntil(reader, /data: not-json/, "legacy invalid frame"), /not-json/);
    await reader.cancel();
  } finally {
    await stopChild(invalid.child);
  }

  const oversized = await startHttpFixture("mcp_legacy_sse_server.cjs", "event-limit-plus-one");
  try {
    const reader = await openLegacyStream(oversized.url);
    await postJson(new URL("/messages", oversized.url), { jsonrpc: "2.0", id: 1, method: "initialize" });
    const event = await readSseUntil(reader, /\n\n$/, "legacy oversized event");
    assert.ok(Buffer.byteLength(event) > LIMITS.protocolMessageBytes);
    await reader.cancel();
  } finally {
    await stopChild(oversized.child);
  }
  for (const required of ["disconnect-stream", "disconnect-message", "hang-invocation", "catalog-limit-plus-one"]) {
    assert.ok(FIXTURE_MODES.legacySse.includes(required));
  }
}

const probes = {
  stdio: probeStdio,
  "streamable-http": probeStreamableHttp,
  "legacy-sse": probeLegacySse,
};

if (!probes[probe]) throw new Error(`Unknown fixture probe: ${probe}`);
probes[probe]().then(() => process.stdout.write(`OK ${probe}\n`)).catch((error) => {
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});
