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

rl.on("line", (line) => {
  if (!line.trim()) return;
  const message = JSON.parse(line);
  if (message.method === "initialize") {
    send(message.id, serverInfo());
    return;
  }
  if (message.method === "tools/list") {
    send(message.id, toolList());
  }
});
