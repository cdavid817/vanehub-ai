import { execFile } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import { cliFixtureIsCurrent } from "./wdio-cli-fixture.mjs";

export const agentMcpFixtureDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "fixtures",
  "agent-mcp",
);

/**
 * Compiles the fake Agents this layer drives, and only when their source is newer.
 *
 * They are built beside `agent-mcp-fixture.mjs` because each one launches it by sibling path; move
 * the binaries anywhere else and every invocation dies with `MODULE_NOT_FOUND`.
 *
 * The build is skipped when it would produce nothing new. wdio evaluates this config once per
 * worker, so an unconditional `rustc` relinks binaries a previous worker's application may still
 * hold. Windows releases that handle a moment after the process dies rather than with it, so the
 * link loses the race and fails the whole layer at config load -- seconds in, before a spec runs,
 * with no assertion to explain it. The same defect took down three other layers in this branch.
 */
export async function prepareAgentMcpFixtures() {
  if (process.platform !== "win32") return;
  const source = path.join(agentMcpFixtureDir, "agent-mcp-fixture.rs");
  for (const name of ["claude", "codex", "opencode"]) {
    const binary = path.join(agentMcpFixtureDir, `${name}.exe`);
    if (await cliFixtureIsCurrent(source, binary)) continue;
    await promisify(execFile)("rustc", [source, "-o", binary]);
  }
}
