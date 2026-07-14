const http = require("node:http");

function serverInfo() {
  return {
    protocolVersion: "2025-06-18",
    capabilities: {
      tools: {},
    },
    serverInfo: {
      name: "vanehub-fixture-http",
      version: "1.0.0",
    },
  };
}

function toolList() {
  return {
    tools: [
      {
        name: "fixture_http_echo",
        description: "Echo input from the VaneHub HTTP fixture",
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

function json(res, id, result, sessionId = "fixture-session") {
  res.writeHead(200, {
    "content-type": "application/json",
    "mcp-session-id": sessionId,
  });
  res.end(JSON.stringify({ jsonrpc: "2.0", id, result }));
}

const server = http.createServer((req, res) => {
  if (req.method === "DELETE") {
    res.writeHead(202);
    res.end();
    return;
  }

  if (req.method !== "POST" || req.url !== "/mcp") {
    res.writeHead(404);
    res.end("not found");
    return;
  }

  let body = "";
  req.on("data", (chunk) => {
    body += chunk;
  });
  req.on("end", () => {
    const message = body ? JSON.parse(body) : {};
    if (message.method === "initialize") {
      json(res, message.id, serverInfo());
      return;
    }
    if (message.method === "tools/list") {
      json(res, message.id, toolList(), req.headers["mcp-session-id"] || "fixture-session");
      return;
    }
    if (!Object.prototype.hasOwnProperty.call(message, "id")) {
      res.writeHead(202);
      res.end();
      return;
    }
    json(res, message.id, {});
  });
});

server.listen(Number(process.argv[2] || 0), "127.0.0.1", () => {
  const address = server.address();
  process.stdout.write(`READY http://127.0.0.1:${address.port}/mcp\n`);
});
