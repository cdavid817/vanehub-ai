import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";
import { spawnSync } from "node:child_process";
import { validateSource } from "./agent-capability-matrix-validate.mjs";

const root = resolve(import.meta.dirname, "..");
const source = JSON.parse(
  readFileSync(resolve(root, "docs/reference/agents/capability-matrix.source.json"), "utf8"),
);
// JSON round-trip rather than structuredClone: the lint config predates the global.
const clone = () => JSON.parse(JSON.stringify(source));

test("the curated source agrees with every code registry it mirrors", () => {
  assert.deepEqual(validateSource(source, root).problems, []);
});

test("dropping an agent is reported by id", () => {
  const broken = clone();
  broken.agents = broken.agents.filter((agent) => agent.id !== "qoder-cli");
  const { problems } = validateSource(broken, root);
  assert.ok(problems.some((line) => line.includes("managedCliAgentIds")));
  assert.ok(problems.some((line) => line.startsWith("qoder-cli: in catalog.v2.json")));
});

test("flipping the MCP relay flag on OpenCode is caught against the Rust allowlist", () => {
  const broken = clone();
  broken.agents.find((agent) => agent.id === "opencode").mcpRelay = "own-config";
  const { problems } = validateSource(broken, root);
  assert.ok(problems.some((line) => line.includes('mcpRelay "relayed"') && line.includes("opencode")));
});

test("a wrong usage capability or transport names the agent and the declared value", () => {
  const broken = clone();
  const antigravity = broken.agents.find((agent) => agent.id === "antigravity-cli");
  antigravity.usageReporting = "unavailable";
  antigravity.transport = "acp-stdio";
  const { problems } = validateSource(broken, root);
  assert.ok(problems.some((line) => line.includes("antigravity-cli") && line.includes("HeadlessReported")));
  assert.ok(problems.some((line) => line.includes("antigravity-cli") && line.includes("HEADLESS")));
});

test("curated upstream versions are rejected so provenance stays generated", () => {
  const broken = clone();
  broken.agents[0].liveQualification.upstreamVersion = "9.9.9";
  const { problems } = validateSource(broken, root);
  assert.ok(problems.some((line) => line.includes("upstreamVersion/verifiedOn are generated")));
});

test("the committed matrix is current", () => {
  const result = spawnSync(process.execPath, ["scripts/generate-agent-capability-matrix.mjs", "--check"], {
    cwd: root,
    encoding: "utf8",
  });
  assert.equal(result.status, 0, `${result.stdout}${result.stderr}`);
});
