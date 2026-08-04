const http = require("node:http");
const {
  LIMITS,
  rpcResponse,
  rpcResponseAtBytes,
  serverInfo,
  toolCallResult,
  toolList,
} = require("./mcp_fixture_data.cjs");

const mode = process.argv[3] || "normal";
const state = { phase: "ready", streamCount: 0, requests: [] };
let stream;

function sendEvent(data, event = "message") {
  if (!stream) return;
  stream.write(`event: ${event}\ndata: ${data}\n\n`);
}

function response(id, result) {
  sendEvent(rpcResponse(id, result));
}

const server = http.createServer((req, res) => {
  if (req.method === "GET" && req.url === "/state") {
    const body = JSON.stringify(state);
    res.writeHead(200, {
      "content-type": "application/json",
      "content-length": Buffer.byteLength(body),
      connection: "close",
    });
    res.end(body);
    return;
  }

  if (req.method === "GET" && req.url === "/sse") {
    state.phase = "stream";
    state.streamCount += 1;
    if (mode === "redirect") {
      res.writeHead(307, { location: "/redirect-target", connection: "close" });
      res.end();
      return;
    }
    if (mode === "disconnect-stream") {
      req.socket.destroy();
      return;
    }
    res.writeHead(200, {
      "content-type": "text/event-stream",
      "cache-control": "no-cache",
      connection: "keep-alive",
    });
    stream = res;
    sendEvent("/messages", "endpoint");
    req.on("close", () => {
      if (stream === res) stream = undefined;
    });
    return;
  }

  if (req.method !== "POST" || req.url !== "/messages") {
    res.writeHead(404, { connection: "close" });
    res.end();
    return;
  }
  if (mode === "disconnect-message") {
    req.socket.destroy();
    return;
  }

  const chunks = [];
  req.on("data", (chunk) => chunks.push(chunk));
  req.on("end", () => {
    const message = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
    state.requests.push({
      rpcMethod: message.method || null,
      hasId: Object.prototype.hasOwnProperty.call(message, "id"),
    });
    res.writeHead(202, { "content-length": 0, connection: "close" });
    res.end();
    if (!Object.prototype.hasOwnProperty.call(message, "id")) return;

    if (message.method === "initialize") state.phase = "initialize";
    else if (message.method === "tools/list") state.phase = "discovery";
    else if (message.method === "tools/call") state.phase = "invocation";
    if (mode === `hang-${state.phase}`) return;
    if (mode === "invalid-frame") {
      sendEvent("not-json");
      return;
    }
    if (mode === "event-limit-plus-one") {
      sendEvent(rpcResponseAtBytes(message.id, LIMITS.protocolMessageBytes + 1));
      return;
    }
    if (message.method === "initialize") {
      response(message.id, serverInfo("vanehub-fixture-sse", "2024-11-05"));
      return;
    }
    if (message.method === "tools/list") {
      response(
        message.id,
        toolList(mode, "fixture_sse_echo", "Echo input from the legacy SSE fixture"),
      );
      return;
    }
    if (message.method === "tools/call") {
      response(
        message.id,
        toolCallResult(mode, "sse echo", message.params?.arguments?.text || ""),
      );
      return;
    }
    response(message.id, {});
  });
});

server.listen(Number(process.argv[2] || 0), "127.0.0.1", () => {
  const address = server.address();
  process.stdout.write(`READY http://127.0.0.1:${address.port}/sse\n`);
});
