const fs = require("node:fs");
const { spawn } = require("node:child_process");
const readline = require("node:readline");
const {
  LIMITS,
  rpcResponse,
  rpcResponseAtBytes,
  serverInfo,
  toolCallResult,
  toolList,
} = require("./mcp_fixture_data.cjs");

const mode = process.argv[2] || "normal";
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
let descendant;

if (mode === "stderr-secret" || mode === "failure-secret-descendant") {
  process.stderr.write("Authorization: Bearer fixture-stderr-secret\n");
}
if (mode === "stderr-limit-plus-one") {
  process.stderr.write(`fixture-stderr-secret:${"x".repeat(LIMITS.stderrBytes + 1)}\n`);
}
if (mode === "spawn-descendant" || mode === "failure-secret-descendant") {
  descendant = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], {
    stdio: "ignore",
    windowsHide: true,
  });
  const pidFile = process.env.VANEHUB_MCP_FIXTURE_DESCENDANT_PID_FILE;
  if (pidFile) fs.writeFileSync(pidFile, String(descendant.pid), "utf8");
}

function send(id, result) {
  process.stdout.write(`${rpcResponse(id, result)}\n`);
}

function phase(method) {
  if (method === "initialize") return "initialize";
  if (method === "tools/list") return "discovery";
  if (method === "tools/call") return "invocation";
  return "other";
}

function cleanupDescendant() {
  if (descendant && descendant.exitCode === null) descendant.kill();
}

rl.on("line", (line) => {
  if (!line.trim()) return;
  const message = JSON.parse(line);
  const currentPhase = phase(message.method);
  if (mode === `hang-${currentPhase}`) return;
  if (mode === `disconnect-${currentPhase}`) {
    cleanupDescendant();
    process.exit(0);
  }
  if (
    (mode === "invalid-frame" || mode === "failure-secret-descendant") &&
    message.method === "initialize"
  ) {
    process.stdout.write("not-json\n");
    return;
  }
  if (mode === "frame-limit-plus-one" && message.method === "initialize") {
    process.stdout.write(`${rpcResponseAtBytes(message.id, LIMITS.protocolMessageBytes + 1)}\n`);
    return;
  }
  if (message.method === "initialize") {
    send(message.id, serverInfo("vanehub-fixture-stdio"));
    return;
  }
  if (message.method === "tools/list") {
    send(
      message.id,
      toolList(mode, "fixture_echo", "Echo input from the VaneHub stdio fixture"),
    );
    return;
  }
  if (message.method === "tools/call") {
    const name = message.params?.name;
    if (name !== "fixture_echo") {
      send(message.id, {
        content: [{ type: "text", text: `Unknown tool "${name}".` }],
        isError: true,
      });
      return;
    }
    send(
      message.id,
      toolCallResult(mode, "echo", message.params?.arguments?.text || ""),
    );
    return;
  }
  if (message.method === "fixture/shutdown") {
    send(message.id, {});
    cleanupDescendant();
    rl.close();
  }
});

rl.on("close", () => {
  cleanupDescendant();
  process.exit(0);
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    cleanupDescendant();
    process.exit(0);
  });
}
