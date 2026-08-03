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
const state = { phase: "ready", deleteCount: 0, requests: [] };

function respondJson(res, id, result, sessionId = "fixture-session") {
  const body = rpcResponse(id, result);
  res.writeHead(200, {
    "content-type": "application/json",
    "content-length": Buffer.byteLength(body),
    "mcp-session-id": sessionId,
  });
  res.end(body);
}

function respondSse(res, id, result, sessionId = "fixture-session") {
  const body = `event: message\ndata: ${rpcResponse(id, result)}\n\n`;
  res.writeHead(200, {
    "content-type": "text/event-stream",
    "content-length": Buffer.byteLength(body),
    "mcp-session-id": sessionId,
  });
  res.end(body);
}

function respondRpc(res, id, result, sessionId) {
  if (mode === "sse-response") respondSse(res, id, result, sessionId);
  else respondJson(res, id, result, sessionId);
}

function recordRequest(req, message) {
  state.requests.push({
    method: req.method,
    rpcMethod: message?.method || null,
    hasId: Object.prototype.hasOwnProperty.call(message || {}, "id"),
    sessionId: req.headers["mcp-session-id"] || null,
  });
}

const server = http.createServer((req, res) => {
  if (req.method === "GET" && req.url === "/phase") {
    res.writeHead(200, { "content-type": "text/plain", connection: "close" });
    res.end(state.phase);
    return;
  }
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

  if (req.method === "DELETE" && req.url === "/mcp") {
    state.phase = "cleanup";
    state.deleteCount += 1;
    recordRequest(req, null);
    const finishDelete = () => {
      res.writeHead(mode === "fail-delete" ? 500 : 202, { connection: "close" });
      res.end();
    };
    if (mode === "hang-delete") return;
    if (mode === "delay-delete") setTimeout(finishDelete, 250);
    else finishDelete();
    return;
  }

  if (req.method !== "POST" || req.url !== "/mcp") {
    res.writeHead(404, { connection: "close" });
    res.end("not found");
    return;
  }
  if (mode === "redirect") {
    res.writeHead(307, { location: "/redirect-target", connection: "close" });
    res.end();
    return;
  }
  if (mode === "disconnect") {
    req.socket.destroy();
    return;
  }

  const chunks = [];
  req.on("data", (chunk) => chunks.push(chunk));
  req.on("end", () => {
    let message;
    try {
      message = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
    } catch {
      res.writeHead(400, { connection: "close" });
      res.end();
      return;
    }
    recordRequest(req, message);

    if (!Object.prototype.hasOwnProperty.call(message, "id")) {
      state.phase = "notification";
      res.writeHead(202, { "content-length": 0, connection: "close" });
      res.end();
      return;
    }

    if (message.method === "initialize") state.phase = "initialize";
    else if (message.method === "tools/list") state.phase = "discovery";
    else if (message.method === "tools/call") state.phase = "invocation";

    if (mode === `hang-${state.phase}`) return;
    if (mode === "invalid-frame") {
      res.writeHead(200, { "content-type": "application/json", connection: "close" });
      res.end("not-json");
      return;
    }
    if (mode === "body-limit-plus-one") {
      const body = rpcResponseAtBytes(message.id, LIMITS.protocolMessageBytes + 1);
      res.writeHead(200, {
        "content-type": "application/json",
        "content-length": Buffer.byteLength(body),
        connection: "close",
      });
      res.end(body);
      return;
    }
    if (mode === "sse-event-limit-plus-one") {
      const data = rpcResponseAtBytes(message.id, LIMITS.protocolMessageBytes + 1);
      const body = `data: ${data}\n\n`;
      res.writeHead(200, {
        "content-type": "text/event-stream",
        "content-length": Buffer.byteLength(body),
        connection: "close",
      });
      res.end(body);
      return;
    }

    const sessionId = req.headers["mcp-session-id"] || "fixture-session";
    if (message.method === "initialize") {
      respondRpc(res, message.id, serverInfo("vanehub-fixture-http"), sessionId);
      return;
    }
    if (message.method === "tools/list") {
      respondRpc(
        res,
        message.id,
        toolList(mode, "fixture_http_echo", "Echo input from the VaneHub HTTP fixture"),
        sessionId,
      );
      return;
    }
    if (message.method === "tools/call") {
      respondRpc(
        res,
        message.id,
        toolCallResult(mode, "http echo", message.params?.arguments?.text || ""),
        sessionId,
      );
      return;
    }
    respondRpc(res, message.id, {}, sessionId);
  });
});

server.listen(Number(process.argv[2] || 0), "127.0.0.1", () => {
  const address = server.address();
  process.stdout.write(`READY http://127.0.0.1:${address.port}/mcp\n`);
});
