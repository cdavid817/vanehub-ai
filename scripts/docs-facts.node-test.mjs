// Documentation fact tests: pin statements the guides make about the toolchain, the release
// pipeline, and Agent scope to the files those statements describe. `docs:check` proves links
// resolve; it cannot tell that a sentence went stale. Each test here reads the source of truth
// and the documents together, so a one-sided edit fails before it merges.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

function read(path) {
  return readFileSync(path, "utf8").replaceAll("\r\n", "\n");
}

const packageJson = JSON.parse(read("package.json"));
const playwrightConfig = read("playwright.config.ts");
const packageWorkflow = read(".github/workflows/package.yml");
const relaySource = read("src-tauri/src/bootstrap/managed_mcp_relay.rs");
const evaluationCenter = read("src/evaluation-center/evaluation-center.tsx");
const evaluationTypes = read("src/types/evaluation.ts");
const agentTypes = read("src/types/agent.ts");

const readmes = ["README.md", "README.zh-CN.md", "README.ja.md"].map((path) => [path, read(path)]);
const releasePages = [
  "docs/developer-guide/src/release.md",
  "docs/developer-guide/zh-CN/src/release.md",
].map((path) => [path, read(path)]);
const testingPages = [
  "docs/developer-guide/src/testing.md",
  "docs/developer-guide/zh-CN/src/testing.md",
].map((path) => [path, read(path)]);
const mcpPages = [
  "docs/user-guide/en/src/mcp.md",
  "docs/user-guide/zh-CN/src/mcp.md",
  "docs/user-guide/en/src/faq.md",
  "docs/user-guide/zh-CN/src/faq.md",
  "docs/developer-guide/src/mcp-tools.md",
  "docs/developer-guide/zh-CN/src/mcp-tools.md",
  "docs/developer-guide/src/runtime-boundaries.md",
  "docs/developer-guide/zh-CN/src/runtime-boundaries.md",
  "docs/developer-guide/src/agent-lifecycle.md",
  "docs/developer-guide/zh-CN/src/agent-lifecycle.md",
].map((path) => [path, read(path)]);
const evaluationPages = [
  "docs/user-guide/en/src/evaluation.md",
  "docs/user-guide/zh-CN/src/evaluation.md",
].map((path) => [path, read(path)]);
const memoryPages = [
  "docs/developer-guide/src/cross-session-memory.md",
  "docs/developer-guide/zh-CN/src/cross-session-memory.md",
].map((path) => [path, read(path)]);
const permissionPages = [
  "docs/developer-guide/src/permission-model.md",
  "docs/developer-guide/zh-CN/src/permission-model.md",
  "docs/user-guide/en/src/permissions.md",
  "docs/user-guide/zh-CN/src/permissions.md",
].map((path) => [path, read(path)]);
const quickStartPages = [
  "docs/user-guide/en/src/quick-start.md",
  "docs/user-guide/zh-CN/src/quick-start.md",
].map((path) => [path, read(path)]);

test("the testing chapter quotes the vitest and playwright commands verbatim", () => {
  const webServer = playwrightConfig.match(/command:\s*`([^`]+)`/)?.[1];
  assert.ok(webServer, "playwright.config.ts webServer command not found");
  const literalWebServer = webServer.replace("${developmentPort}", "5174");
  for (const [path, content] of testingPages) {
    assert.ok(content.includes(`test: "${packageJson.scripts.test}"`), `${path}: test script drifted`);
    assert.ok(
      content.includes(`test:coverage: "${packageJson.scripts["test:coverage"]}"`),
      `${path}: test:coverage script drifted`,
    );
    assert.ok(content.includes(literalWebServer), `${path}: Playwright webServer command drifted`);
    assert.doesNotMatch(content, /^## [^\n]+\n\n## /m, `${path}: an empty second-level heading`);
  }
});

test("the desktop layer count is never written down; the script owns the list", () => {
  const ci = read(".github/workflows/ci.yml");
  assert.doesNotMatch(ci, /all (?:six|ten|\d+) layers/, "ci.yml hard-codes a desktop layer count");
  const agents = read("AGENTS.md");
  assert.doesNotMatch(agents, /\d+ 个层/, "AGENTS.md hard-codes a desktop layer count");
});

test("release pages describe the signing gate the package workflow actually enforces", () => {
  assert.ok(packageWorkflow.includes('if [[ -z "${TAURI_SIGNING_PRIVATE_KEY}" ]]'));
  assert.ok(packageWorkflow.includes("env.WINDOWS_CERTIFICATE != ''"));
  assert.ok(packageWorkflow.includes("env.APPLE_CERTIFICATE != ''"));
  for (const [path, content] of releasePages) {
    assert.ok(content.includes("TAURI_SIGNING_PRIVATE_KEY"), `${path}: updater key not named`);
    assert.ok(content.includes("WINDOWS_CERTIFICATE != ''"), `${path}: Windows gate condition missing`);
    assert.ok(content.includes("APPLE_CERTIFICATE != ''"), `${path}: Apple gate condition missing`);
    assert.doesNotMatch(
      content,
      /produces a signed NSIS|notarized and stapled|NSIS \.exe<br\/>signed|\.dmg<br\/>notarize \+ staple|走 notarize \+ staple/,
      `${path}: claims present-tense OS signing`,
    );
    assert.match(content, /Not Authenticode signed|未做 Authenticode 签名/, `${path}: Windows status missing`);
    assert.match(content, /not notarized|未公证/, `${path}: macOS status missing`);
    assert.match(content, /no Windows ARM64 package|不发布 Windows ARM64 包/, `${path}: ARM64 disclaimer missing`);
  }
  const signing = read("docs/release-signing.md");
  assert.doesNotMatch(signing, /\bv?1\.0\.0\b/, "release-signing.md hard-codes a version");
  assert.match(signing, /Required now/);
  assert.match(signing, /Required only when Windows Authenticode signing is enabled/);
  assert.match(signing, /Required only when Apple Developer ID signing and notarization are enabled/);
  for (const [path, content] of readmes) {
    assert.match(
      content,
      /Authenticode/,
      `${path}: README download section must state the OS signing status`,
    );
  }
});

test("the READMEs agree on the stable/main boundary and never promise identical capabilities", () => {
  const stableIds = agentTypes.match(/managedCliAgentIds = \[([\s\S]*?)\]/)?.[1];
  assert.ok(stableIds, "managedCliAgentIds not found");
  for (const [path, content] of readmes) {
    assert.match(content, /v1\.5\.0/, `${path}: stable version not named`);
    assert.ok(content.includes("docs/reference/agents/capability-matrix.md"), `${path}: capability matrix link missing`);
    assert.ok(content.includes("docs/reference/terminology.md"), `${path}: terminology link missing`);
    assert.doesNotMatch(
      content,
      /eleven external CLI agents share|十一个外部 CLI Agent 共用|5 つの外部 CLI エージェントが/,
      `${path}: over-strong "all agents share everything" claim`,
    );
    for (const acp of ["Qwen Code", "Kimi Code CLI", "Qoder CLI", "CodeBuddy Code", "GitHub Copilot CLI", "Cursor Agent CLI"]) {
      assert.match(content, new RegExp(`\\| ${acp.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")} \\| main \\|`), `${path}: ${acp} row lacks the main status`);
    }
    assert.match(content, /\| iFlow CLI \| legacy \(main\) \|/, `${path}: iFlow row lacks the legacy status`);
  }
});

test("MCP relay scope in the guides equals the relay allowlist", () => {
  const allowlist = relaySource.match(/matches!\(agent_id,\s*([^)]+)\)/)?.[1];
  assert.ok(allowlist, "relay allowlist not found");
  const ids = [...allowlist.matchAll(/"([a-z-]+)"/g)].map((match) => match[1]).sort();
  assert.deepEqual(ids, ["claude-code", "codex-cli", "opencode"]);
  for (const [path, content] of mcpPages) {
    assert.doesNotMatch(
      content,
      /only Claude Code and Codex CLI|仅 Claude Code 与 Codex CLI|只有 Claude Code 和 Codex CLI|只对 Claude Code 与 Codex CLI/,
      `${path}: stale two-agent relay scope`,
    );
    assert.match(content, /OpenCode/, `${path}: OpenCode must be named on the relay path`);
  }
});

test("the evaluation chapter states the registry-driven candidate rule and the state counts", () => {
  assert.ok(evaluationCenter.includes('capabilityTags.includes("legacy")'));
  const max = Number(evaluationCenter.match(/MAX_ARENA_AGENTS = (\d+)/)?.[1]);
  const terminal = evaluationCenter.match(/TERMINAL = new Set\(\[([^\]]+)\]\)/)?.[1];
  const outcomes = evaluationTypes.match(/EvaluationOutcome = ([^;]+);/)?.[1];
  assert.equal(max, 8);
  assert.equal([...terminal.matchAll(/"/g)].length / 2, 7);
  assert.equal([...outcomes.matchAll(/"/g)].length / 2, 9);
  for (const [path, content] of evaluationPages) {
    assert.doesNotMatch(content, /Only onepiece and codex-cli|目前只能选 onepiece 和 codex-cli|nine terminal states|九种终态/, `${path}: stale candidate or state claim`);
    assert.match(content, /legacy/, `${path}: legacy exclusion not stated`);
    assert.match(content, /iFlow/, `${path}: iFlow exclusion example missing`);
    assert.match(content, /OpenCode/, `${path}: OpenCode candidate example missing`);
    assert.match(content, /\b8\b|八/, `${path}: arena cap missing`);
  }
});

test("the memory chapters describe governed recall, not the compatibility view", () => {
  for (const [path, content] of memoryPages) {
    assert.doesNotMatch(content, /scoped recall is not implemented|not yet implemented — do not read as done|scoped 召回是上文列出的待实现目标/, `${path}: stale recall claim`);
    for (const symbol of ["MemoryReadContext", "GovernedMemoryReadService", "search_authorized"]) {
      assert.ok(content.includes(symbol), `${path}: ${symbol} not named`);
    }
    assert.match(content, /main \/ unreleased/, `${path}: status vocabulary missing`);
  }
});

test("permission pages classify projection by transport instead of five equal agents", () => {
  const invocation = read("src-tauri/src/contexts/agent_runtime/infrastructure/providers/invocation.rs");
  assert.match(invocation, /RUNTIME_POLICY_AGENT_IDS: \[&str; 7\]/);
  assert.match(invocation, /POLICY_TEMPLATE_GOVERNED_AGENT_IDS: \[&str; 12\]/);
  for (const [path, content] of permissionPages) {
    assert.doesNotMatch(content, /all five CLI Agents|全部五个 CLI Agent|identical policy in practice|实际策略完全相同|has no native flags to project into/, `${path}: stale five-agent or identical-template claim`);
    assert.match(content, /provider-delegated/i, `${path}: provider-delegated guarantee not named`);
    assert.match(content, /terminal-readonly-unsupported|Qoder/, `${path}: Qoder read-only limitation missing`);
  }
});

test("quick start distinguishes upstream support from VaneHub management", () => {
  for (const [path, content] of quickStartPages) {
    assert.doesNotMatch(content, /Antigravity CLI does not accept a custom endpoint|has no npm package, so the UI offers no install|没有 npm 包，所以界面上不提供/, `${path}: stale Antigravity claim`);
    assert.match(content, /WinGet/, `${path}: install sources not source-aware`);
  }
});
