import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";

const SERVER_NAME = "desktop-agent-mcp";
const agent = process.env.VANEHUB_MCP_FIXTURE_AGENT
  || path.basename(process.argv[1]).replace(/\.exe$/i, "");

function claudeProjection(args) {
  const index = args.indexOf("--mcp-config");
  if (index < 0 || !args[index + 1]) throw new Error("Claude MCP configuration was not injected");
  return JSON.parse(fs.readFileSync(args[index + 1], "utf8")).mcpServers?.[SERVER_NAME];
}

function codexProjection(args) {
  const entries = {};
  for (let index = 0; index < args.length - 1; index += 1) {
    if (args[index] !== "-c") continue;
    const value = args[index + 1];
    const match = /^mcp_servers\.("(?:[^"\\]|\\.)*")\.(command|args)=(.*)$/u.exec(value);
    if (!match || JSON.parse(match[1]) !== SERVER_NAME) continue;
    entries[match[2]] = JSON.parse(match[3]);
  }
  return entries.command ? entries : null;
}

function opencodeProjection() {
  const inline = process.env.OPENCODE_CONFIG_CONTENT;
  if (!inline) throw new Error("OpenCode inline MCP configuration was not injected");
  const entry = JSON.parse(inline).mcp?.[SERVER_NAME];
  if (!entry || entry.type !== "local" || !Array.isArray(entry.command)) return null;
  return { command: entry.command[0], args: entry.command.slice(1) };
}

function projection() {
  const args = process.argv.slice(2);
  if (agent === "claude") return claudeProjection(args);
  if (agent === "codex") return codexProjection(args);
  if (agent === "opencode") return opencodeProjection();
  throw new Error(`Unknown fixture Agent ${agent}`);
}

function writeEvidence(value) {
  const directory = process.env.VANEHUB_MCP_AGENT_EVIDENCE_DIR;
  if (!directory) return;
  fs.mkdirSync(directory, { recursive: true });
  fs.appendFileSync(
    path.join(directory, `${agent}.jsonl`),
    `${JSON.stringify({ agent, pid: process.pid, ...value })}\n`,
    "utf8",
  );
}

function rpcClient(entry) {
  if (!entry?.command) throw new Error(`${agent} did not receive ${SERVER_NAME}`);
  const child = spawn(entry.command, entry.args ?? [], {
    stdio: ["pipe", "pipe", "pipe"],
    windowsHide: true,
  });
  const pending = new Map();
  let stderr = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr = `${stderr}${chunk}`.slice(-2_000); });
  readline.createInterface({ input: child.stdout, crlfDelay: Infinity }).on("line", (line) => {
    const message = JSON.parse(line);
    const waiter = pending.get(message.id);
    if (waiter) {
      pending.delete(message.id);
      waiter.resolve(message);
    }
  });
  child.once("exit", (code) => {
    for (const waiter of pending.values()) {
      waiter.reject(new Error(`MCP relay exited ${code}: ${stderr}`));
    }
    pending.clear();
  });
  const request = (id, method, params) => new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, ...(params ? { params } : {}) })}\n`);
  });
  const notify = (method) => child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method })}\n`);
  return { child, request, notify };
}

function emitCompletion(text) {
  const id = `${agent}-${process.pid}`;
  if (agent === "claude") {
    process.stdout.write(`${JSON.stringify({ type: "system", session_id: id })}\n`);
    process.stdout.write(`${JSON.stringify({ type: "content_block_delta", delta: { text } })}\n`);
    process.stdout.write(`${JSON.stringify({ type: "result" })}\n`);
    return;
  }
  if (agent === "codex") {
    process.stdout.write(`${JSON.stringify({ type: "thread.started", thread_id: id })}\n`);
    process.stdout.write(`${JSON.stringify({ type: "item.completed", item: { type: "message", role: "assistant", content: [{ type: "output_text", text }] } })}\n`);
    process.stdout.write(`${JSON.stringify({ type: "turn.completed" })}\n`);
    return;
  }
  process.stdout.write(`${JSON.stringify({ type: "step_start", sessionID: id, part: { type: "step-start" } })}\n`);
  process.stdout.write(`${JSON.stringify({ type: "text", sessionID: id, part: { type: "text", text } })}\n`);
  process.stdout.write(`${JSON.stringify({ type: "step_finish", sessionID: id, part: { type: "step-finish", reason: "stop" } })}\n`);
}

async function main() {
  if (process.argv.slice(2).includes("--version")) {
    process.stdout.write(`vanehub-${agent}-fixture 1.0.0\n`);
    return;
  }
  const entry = projection();
  const client = rpcClient(entry);
  const initialized = await client.request(1, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: `vanehub-${agent}-fixture`, version: "1.0.0" },
  });
  client.notify("notifications/initialized");
  const catalog = await client.request(2, "tools/list");
  const called = await client.request(3, "tools/call", {
    name: "fixture_echo",
    arguments: { text: `${agent}-mcp-effective` },
  });
  const output = called.result?.content?.[0]?.text;
  if (output !== `echo: ${agent}-mcp-effective`) throw new Error(`Unexpected MCP result ${output}`);
  writeEvidence({
    phase: "completed",
    protocolVersion: initialized.result?.protocolVersion,
    tools: catalog.result?.tools?.map((tool) => tool.name),
    output,
  });
  emitCompletion(`MCP_EFFECTIVE ${agent} ${output}`);
  client.child.stdin.end();
}

main().catch((error) => {
  writeEvidence({ phase: "failed", error: error.message });
  process.stderr.write(`VANEHUB MCP fixture failed: ${error.message}\n`);
  process.exitCode = 1;
});
