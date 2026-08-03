const LIMITS = Object.freeze({
  importDocumentBytes: 1024 * 1024,
  importServerEntries: 128,
  configurationCollectionEntries: 128,
  configurationSerializedBytes: 256 * 1024,
  protocolMessageBytes: 2 * 1024 * 1024,
  toolsPerServer: 128,
  providerTools: 256,
  toolNameBytes: 256,
  toolDescriptionBytes: 8 * 1024,
  schemaBytes: 128 * 1024,
  jsonDepth: 32,
  toolArgumentsBytes: 256 * 1024,
  toolResultBytes: 1024 * 1024,
  stderrBytes: 64 * 1024,
});

const FIXTURE_MODES = Object.freeze({
  stdio: [
    "normal",
    "hang-initialize",
    "hang-discovery",
    "hang-invocation",
    "disconnect-initialize",
    "disconnect-discovery",
    "disconnect-invocation",
    "invalid-frame",
    "frame-limit-plus-one",
    "tools-limit-plus-one",
    "tool-name-limit-plus-one",
    "description-limit-plus-one",
    "schema-limit-plus-one",
    "schema-depth-limit-plus-one",
    "catalog-limit-plus-one",
    "result-limit-plus-one",
    "stderr-secret",
    "stderr-limit-plus-one",
    "spawn-descendant",
    "failure-secret-descendant",
  ],
  streamableHttp: [
    "normal",
    "sse-response",
    "notification-202",
    "redirect",
    "disconnect",
    "invalid-frame",
    "body-limit-plus-one",
    "sse-event-limit-plus-one",
    "hang-initialize",
    "hang-discovery",
    "hang-invocation",
    "fail-delete",
    "delay-delete",
    "hang-delete",
    "tools-limit-plus-one",
    "tool-name-limit-plus-one",
    "description-limit-plus-one",
    "schema-limit-plus-one",
    "schema-depth-limit-plus-one",
    "catalog-limit-plus-one",
    "result-limit-plus-one",
  ],
  legacySse: [
    "normal",
    "redirect",
    "disconnect-stream",
    "disconnect-message",
    "invalid-frame",
    "event-limit-plus-one",
    "hang-initialize",
    "hang-discovery",
    "hang-invocation",
    "tools-limit-plus-one",
    "tool-name-limit-plus-one",
    "description-limit-plus-one",
    "schema-limit-plus-one",
    "schema-depth-limit-plus-one",
    "catalog-limit-plus-one",
    "result-limit-plus-one",
  ],
});

function serverInfo(name, protocolVersion = "2025-06-18") {
  return {
    protocolVersion,
    capabilities: { tools: {} },
    serverInfo: { name, version: "1.0.0" },
  };
}

function baseTool(name, description) {
  return {
    name,
    description,
    inputSchema: {
      type: "object",
      properties: { text: { type: "string" } },
    },
  };
}

function jsonObjectAtBytes(byteLength) {
  const value = { type: "object", padding: "" };
  const overhead = Buffer.byteLength(JSON.stringify(value));
  value.padding = "x".repeat(byteLength - overhead);
  return value;
}

function nestedObject(depth) {
  let value = null;
  for (let current = 1; current < depth; current += 1) value = { nested: value };
  return value;
}

function toolList(mode, toolName, description) {
  const tool = baseTool(toolName, description);
  if (mode === "tools-limit-plus-one") {
    return {
      tools: Array.from({ length: LIMITS.toolsPerServer + 1 }, (_, index) => ({
        ...baseTool(`fixture_tool_${index}`, "Fixture tool"),
      })),
    };
  }
  if (mode === "tool-name-limit-plus-one") tool.name = "n".repeat(LIMITS.toolNameBytes + 1);
  if (mode === "description-limit-plus-one") {
    tool.description = "d".repeat(LIMITS.toolDescriptionBytes + 1);
  }
  if (mode === "schema-limit-plus-one") {
    tool.inputSchema = jsonObjectAtBytes(LIMITS.schemaBytes + 1);
  }
  if (mode === "schema-depth-limit-plus-one") {
    tool.inputSchema = nestedObject(LIMITS.jsonDepth + 1);
  }
  if (mode === "catalog-limit-plus-one") {
    const tools = Array.from({ length: 18 }, (_, index) => ({
      ...baseTool(`fixture_catalog_${index}`, "Catalog boundary fixture"),
      inputSchema: jsonObjectAtBytes(120 * 1024),
    }));
    return { tools };
  }
  return { tools: [tool] };
}

function toolCallResult(mode, prefix, text) {
  const rendered = mode === "result-limit-plus-one"
    ? "r".repeat(LIMITS.toolResultBytes + 1)
    : `${prefix}: ${text}`;
  return { content: [{ type: "text", text: rendered }], isError: false };
}

function rpcResponse(id, result) {
  return JSON.stringify({ jsonrpc: "2.0", id, result });
}

function rpcResponseAtBytes(id, byteLength) {
  const response = { jsonrpc: "2.0", id, result: { padding: "" } };
  const overhead = Buffer.byteLength(JSON.stringify(response));
  response.result.padding = "x".repeat(byteLength - overhead);
  return JSON.stringify(response);
}

function limitPlusOneFixtures() {
  const entries = (count, value) => Object.fromEntries(
    Array.from({ length: count }, (_, index) => [`entry-${index}`, value]),
  );
  return {
    importDocument: "x".repeat(LIMITS.importDocumentBytes + 1),
    importServers: entries(LIMITS.importServerEntries + 1, { command: "node" }),
    args: Array.from({ length: LIMITS.configurationCollectionEntries + 1 }, () => "x"),
    env: entries(LIMITS.configurationCollectionEntries + 1, "x"),
    headers: entries(LIMITS.configurationCollectionEntries + 1, "x"),
    serializedConfiguration: jsonObjectAtBytes(LIMITS.configurationSerializedBytes + 1),
    protocolMessage: rpcResponseAtBytes(1, LIMITS.protocolMessageBytes + 1),
    tools: toolList("tools-limit-plus-one", "tool", "description").tools,
    providerTools: Array.from({ length: LIMITS.providerTools + 1 }, (_, index) => `tool-${index}`),
    toolName: "n".repeat(LIMITS.toolNameBytes + 1),
    toolDescription: "d".repeat(LIMITS.toolDescriptionBytes + 1),
    schema: jsonObjectAtBytes(LIMITS.schemaBytes + 1),
    schemaDepth: nestedObject(LIMITS.jsonDepth + 1),
    toolArguments: jsonObjectAtBytes(LIMITS.toolArgumentsBytes + 1),
    toolArgumentDepth: nestedObject(LIMITS.jsonDepth + 1),
    toolResult: "r".repeat(LIMITS.toolResultBytes + 1),
    stderr: Buffer.alloc(LIMITS.stderrBytes + 1, "x"),
  };
}

module.exports = {
  FIXTURE_MODES,
  LIMITS,
  limitPlusOneFixtures,
  rpcResponse,
  rpcResponseAtBytes,
  serverInfo,
  toolCallResult,
  toolList,
};
