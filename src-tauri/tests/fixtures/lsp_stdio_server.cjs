"use strict";

const fs = require("node:fs");
const { fileURLToPath } = require("node:url");
const mode = process.argv[2];
const semanticMode = mode === "lsp-semantic" || mode === "lsp-native-e2e";

if (mode === "oversized") {
  process.stdout.write("Content-Length: 1024\r\n\r\n");
  setTimeout(() => {}, 30_000);
} else if (mode === "exit") {
  setTimeout(() => process.exit(0), 50);
} else if ([
  "echo",
  "lsp-success",
  "lsp-invalid-init",
  "lsp-hang",
  "lsp-semantic",
  "lsp-native-e2e",
  "lsp-crash",
  "lsp-protocol-limit",
].includes(mode)) {
  let buffered = Buffer.alloc(0);
  let initialized = false;
  let openedUri;
  let openedText;
  let changeCount = 0;
  process.stdin.on("data", (chunk) => {
    buffered = Buffer.concat([buffered, chunk]);
    drainFrames();
  });

  function drainFrames() {
    while (true) {
      const marker = buffered.indexOf("\r\n\r\n");
      if (marker < 0) return;
      const header = buffered.subarray(0, marker).toString("ascii");
      const match = /^Content-Length:\s*(\d+)$/im.exec(header);
      if (!match) process.exit(2);
      const length = Number.parseInt(match[1], 10);
      const start = marker + 4;
      if (buffered.length < start + length) return;
      const message = JSON.parse(buffered.subarray(start, start + length).toString("utf8"));
      buffered = buffered.subarray(start + length);
      handleMessage(message);
    }
  }

  function handleMessage(message) {
    if (mode === "echo") {
      send(message.id, { pong: true }, () => process.exit(0));
      process.stderr.write("x".repeat(4096));
      return;
    }
    if (mode === "lsp-hang") return;
    if (message.method === "initialize") {
      const marker = mode === "lsp-native-e2e" ? undefined : process.argv[3];
      const root = fileURLToPath(message.params.rootUri);
      if (marker && !fs.existsSync(require("node:path").join(root, marker))) {
        sendError(message.id, -32002);
        return;
      }
      const result = mode === "lsp-invalid-init"
        ? { invalid: true }
        : { capabilities: {
          definitionProvider: true,
          referencesProvider: semanticMode,
          hoverProvider: semanticMode,
          textDocumentSync: semanticMode ? 1 : 2,
        } };
      send(message.id, result);
    } else if (semanticMode && message.method === "initialized") {
      initialized = true;
    } else if (mode === "lsp-crash" && message.method === "initialized") {
      setTimeout(() => process.exit(17), 25);
    } else if (mode === "lsp-protocol-limit" && message.method === "initialized") {
      setTimeout(() => {
        process.stdout.write("Content-Length: 8388609\r\n\r\n");
      }, 25);
    } else if (semanticMode && message.method === "textDocument/didOpen") {
      if (!initialized) process.exit(3);
      openedUri = message.params.textDocument.uri;
      openedText = message.params.textDocument.text;
      publishDiagnostics(openedUri, message.params.textDocument.version, [{
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 2 },
        },
        severity: 2,
        message: "Fixture warning",
        source: "fixture",
        code: "fixture-warning",
      }]);
    } else if (semanticMode && message.method === "textDocument/didChange") {
      openedText = message.params.contentChanges[0].text;
      changeCount += 1;
      publishDiagnostics(openedUri, message.params.textDocument.version, []);
    } else if (semanticMode && message.method === "textDocument/definition") {
      const requestedUri = message.params.textDocument.uri;
      if (!initialized || !openedUri || requestedUri !== openedUri || !openedText) {
        sendError(message.id, -32002);
        return;
      }
      if (message.params.position.character === 1) return;
      send(message.id, [{
        uri: requestedUri,
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: changeCount > 0 ? 3 : 4 },
        },
      }]);
    } else if (semanticMode && message.method === "textDocument/references") {
      const requestedUri = message.params.textDocument.uri;
      const references = Array.from({ length: 55 }, () => ({
        uri: requestedUri,
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 2 },
        },
      }));
      references.push({
        uri: "https://outside.invalid/source.rs",
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 1 },
        },
      });
      send(message.id, references);
    } else if (semanticMode && message.method === "textDocument/hover") {
      send(message.id, {
        contents: [
          { language: "rust", value: "fn alpha()" },
          "Fixture documentation",
        ],
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 2 },
        },
      });
    } else if (message.method === "shutdown") {
      recordLifecycle("shutdown");
      send(message.id, null);
    } else if (message.method === "exit") {
      recordLifecycle("exit");
      process.exit(0);
    }
  }

  function send(id, result, callback) {
    sendPayload({ jsonrpc: "2.0", id, result }, callback);
  }

  function sendError(id, code) {
    sendPayload({ jsonrpc: "2.0", id, error: { code, message: "fixture" } });
  }

  function publishDiagnostics(uri, version, diagnostics) {
    sendPayload({
      jsonrpc: "2.0",
      method: "textDocument/publishDiagnostics",
      params: { uri, version, diagnostics },
    });
  }

  function recordLifecycle(event) {
    if (mode === "lsp-native-e2e" && process.argv[3]) {
      fs.appendFileSync(process.argv[3], `${event}\n`);
    }
  }

  function sendPayload(message, callback) {
    const payload = Buffer.from(JSON.stringify(message));
    process.stdout.write(`Content-Length: ${payload.length}\r\n\r\n`);
    process.stdout.write(payload, callback);
  }
} else {
  process.exit(2);
}
