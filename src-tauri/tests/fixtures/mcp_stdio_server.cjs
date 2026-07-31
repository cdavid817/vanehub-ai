const readline = require("node:readline");

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

function send(id, result) {
  process.stdout.write(`${JSON.stringify({ jsonrpc: "2.0", id, result })}\n`);
}

function serverInfo() {
  return {
    protocolVersion: "2025-06-18",
    capabilities: {
      tools: {},
    },
    serverInfo: {
      name: "vanehub-fixture-stdio",
      version: "1.0.0",
    },
  };
}

function toolList() {
  return {
    tools: [
      {
        name: "fixture_echo",
        description: "Echo input from the VaneHub stdio fixture",
        inputSchema: {
          type: "object",
          properties: {
            text: { type: "string" },
          },
        },
      },
    ],
  };
}

function toolCallResult(params) {
  const { name, arguments: args } = params || {};
  if (name === "fixture_echo") {
    const text = (args && args.text) || "";
    return {
      content: [{ type: "text", text: `echo: ${text}` }],
    };
  }
  return {
    content: [{ type: "text", text: `Unknown tool "${name}".` }],
    isError: true,
  };
}

rl.on("line", (line) => {
  if (!line.trim()) return;
  const message = JSON.parse(line);
  if (message.method === "initialize") {
    send(message.id, serverInfo());
    return;
  }
  if (message.method === "tools/list") {
    send(message.id, toolList());
    return;
  }
  if (message.method === "tools/call") {
    send(message.id, toolCallResult(message.params));
  }
});
