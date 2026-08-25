import { execFile } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

export const agentMcpFixtureDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "fixtures",
  "agent-mcp",
);

export async function prepareAgentMcpFixtures() {
  if (process.platform !== "win32") return;
  const source = path.join(agentMcpFixtureDir, "agent-mcp-fixture.rs");
  for (const name of ["claude", "codex", "opencode"]) {
    await promisify(execFile)("rustc", [source, "-o", path.join(agentMcpFixtureDir, `${name}.exe`)]);
  }
}
