// Cross-checks the hand-curated capability matrix source against the code it describes.
//
// The matrix is curated rather than fully generated because four of its dimensions (transport,
// install sources, relay allowlist, usage capability) have no JSON projection today: they are Rust
// literals. Curation is allowed only because every curated value that the code can contradict is
// re-read from that code here, so a registry change that is not mirrored fails `--check` with the
// agent and field named.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const FILES = {
  agentTypes: "src/types/agent.ts",
  relay: "src-tauri/src/bootstrap/managed_mcp_relay.rs",
  invocation: "src-tauri/src/contexts/agent_runtime/infrastructure/providers/invocation.rs",
  cliConfig: "src-tauri/src/contexts/tooling/cli_config/domain/mod.rs",
  schema: "src-tauri/src/contexts/agent_runtime/infrastructure/schema.rs",
  definitions: "src-tauri/src/contexts/agent_runtime/infrastructure/providers/definitions.rs",
  catalog: "src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json",
  sourceAudit: "src/contracts/fixtures/cli-parameter-source-audit.json",
};

const USAGE_BY_CAPABILITY = {
  HeadlessAndTerminalReported: "headless-and-terminal",
  HeadlessReported: "headless-reported",
  Unavailable: "unavailable",
};
const TRANSPORT_BY_CONST = { HEADLESS: "headless", ACP: "acp-stdio", TERMINAL_ONLY: "terminal-only" };

function read(root, key) {
  return readFileSync(resolve(root, FILES[key]), "utf8").replaceAll("\r\n", "\n");
}

function quotedList(source, name) {
  const match = source.match(new RegExp(`${name}\\b[^=]*=\\s*\\[([\\s\\S]*?)\\];`));
  if (!match) throw new Error(`Could not find the constant ${name}.`);
  return [...match[1].matchAll(/"([a-z0-9-]+)"/g)].map((entry) => entry[1]);
}

function sameSet(actual, expected) {
  const a = [...actual].sort();
  const b = [...expected].sort();
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

function perAgentBlocks(definitions) {
  // Each provider definition starts with `id: "<agent>"`; the next `id:` line closes it.
  const blocks = new Map();
  const ids = [...definitions.matchAll(/^\s{8}id: "([a-z0-9-]+)",\n/gm)];
  for (let index = 0; index < ids.length; index += 1) {
    const start = ids[index].index;
    const end = index + 1 < ids.length ? ids[index + 1].index : definitions.length;
    blocks.set(ids[index][1], definitions.slice(start, end));
  }
  return blocks;
}

export function validateSource(source, root) {
  const problems = [];
  const cli = source.agents.filter((agent) => agent.id !== "onepiece");
  const cliIds = cli.map((agent) => agent.id);
  const byId = new Map(source.agents.map((agent) => [agent.id, agent]));
  if (!byId.has("onepiece")) problems.push("Missing the built-in agent row `onepiece`.");

  // (a) id set and order.
  const managed = quotedList(read(root, "agentTypes"), "managedCliAgentIds");
  if (managed.join(",") !== cliIds.join(",")) {
    problems.push(
      `CLI agent ids/order differ from managedCliAgentIds in ${FILES.agentTypes}: ` +
        `expected [${managed.join(", ")}], source has [${cliIds.join(", ")}].`,
    );
  }

  // (b) MCP relay allowlist.
  const relayMatch = read(root, "relay").match(/matches!\(agent_id,\s*([^)]*)\)/);
  if (!relayMatch) problems.push(`Could not find the relay allowlist in ${FILES.relay}.`);
  else {
    const allowlist = [...relayMatch[1].matchAll(/"([a-z0-9-]+)"/g)].map((entry) => entry[1]);
    const relayed = cli.filter((agent) => agent.mcpRelay === "relayed").map((agent) => agent.id);
    if (!sameSet(allowlist, relayed)) {
      problems.push(
        `mcpRelay "relayed" set [${relayed.join(", ")}] differs from the allowlist in ` +
          `${FILES.relay} [${allowlist.join(", ")}].`,
      );
    }
  }

  // (c) permission projection sets.
  const invocation = read(root, "invocation");
  const governed = quotedList(invocation, "POLICY_TEMPLATE_GOVERNED_AGENT_IDS");
  if (!sameSet(governed, cliIds)) {
    problems.push(`POLICY_TEMPLATE_GOVERNED_AGENT_IDS [${governed.join(", ")}] is not the CLI id set.`);
  }
  const runtime = quotedList(invocation, "RUNTIME_POLICY_AGENT_IDS");
  const runtimeSource = cli
    .filter((agent) =>
      ["runtime-flags-pty-only-under-acp", "runtime-flags-terminal-only"].includes(
        agent.permissionProjection,
      ),
    )
    .map((agent) => agent.id);
  if (!sameSet(runtime, runtimeSource)) {
    problems.push(
      `permissionProjection runtime-flags set [${runtimeSource.join(", ")}] differs from ` +
        `RUNTIME_POLICY_AGENT_IDS [${runtime.join(", ")}].`,
    );
  }

  // (d) provider configuration support set.
  const supported = quotedList(read(root, "cliConfig"), "SUPPORTED_AGENT_IDS");
  const configured = cli
    .filter((agent) => agent.providerConfig !== "not-applicable")
    .map((agent) => agent.id);
  if (!sameSet(supported, configured)) {
    problems.push(
      `providerConfig managed set [${configured.join(", ")}] differs from SUPPORTED_AGENT_IDS ` +
        `[${supported.join(", ")}] in ${FILES.cliConfig}.`,
    );
  }

  // (e) legacy tag.
  const schema = read(root, "schema");
  // Each seed tuple is `(\n "id", ... &[tags], "origin",\n )`; scan tuple by tuple so a tag list
  // is attributed to its own agent and never to the nearest earlier id.
  const legacyInCode = [...schema.matchAll(/\(\n\s*"([a-z0-9-]+)",([\s\S]*?)\n\s*\),/g)]
    .filter((entry) => /&\[[^\]]*"legacy"[^\]]*\]/.test(entry[2]))
    .map((entry) => entry[1]);
  const legacyInSource = source.agents
    .filter((agent) => agent.releaseStatus === "legacy")
    .map((agent) => agent.id);
  if (!sameSet(legacyInCode, legacyInSource)) {
    problems.push(
      `releaseStatus "legacy" set [${legacyInSource.join(", ")}] differs from the seeds tagged ` +
        `legacy in ${FILES.schema} [${legacyInCode.join(", ")}].`,
    );
  }
  for (const agent of source.agents) {
    const expected = agent.releaseStatus !== "legacy";
    if (agent.evaluationCandidate !== expected) {
      problems.push(`${agent.id}: evaluationCandidate must be ${expected} (derived from the legacy tag).`);
    }
  }

  // (f) usage capability and (g) transport per definitions.rs block.
  const blocks = perAgentBlocks(read(root, "definitions"));
  for (const agent of cli) {
    const block = blocks.get(agent.id);
    if (!block) {
      problems.push(`${agent.id}: no provider definition block in ${FILES.definitions}.`);
      continue;
    }
    const usage = block.match(/usage: ProviderUsageCapability::(\w+)/)?.[1];
    if (USAGE_BY_CAPABILITY[usage] !== agent.usageReporting) {
      problems.push(
        `${agent.id}: usageReporting "${agent.usageReporting}" but definitions.rs declares ` +
          `ProviderUsageCapability::${usage}.`,
      );
    }
    const transport = block.match(/transports: (HEADLESS|ACP|TERMINAL_ONLY)/)?.[1];
    if (TRANSPORT_BY_CONST[transport] !== agent.transport) {
      problems.push(
        `${agent.id}: transport "${agent.transport}" but definitions.rs declares ${transport}.`,
      );
    }
    if (agent.managedChat !== (agent.transport !== "terminal-only")) {
      problems.push(`${agent.id}: managedChat contradicts a ${agent.transport} transport.`);
    }
  }

  // (h) every catalog agent has a row.
  const catalog = JSON.parse(read(root, "catalog"));
  for (const entry of catalog.agents) {
    if (!byId.has(entry.agentId)) problems.push(`${entry.agentId}: in catalog.v2.json but not in the matrix source.`);
  }

  // (i) upstream provenance is read from the source audit, never curated.
  const audit = JSON.parse(read(root, "sourceAudit"));
  for (const agent of cli) {
    if (!audit.sources[agent.id] || !audit.binaries[agent.id]) {
      problems.push(`${agent.id}: no official source / verified binary in ${FILES.sourceAudit}.`);
    }
    if ("upstreamVersion" in agent.liveQualification || "verifiedOn" in agent.liveQualification) {
      problems.push(`${agent.id}: upstreamVersion/verifiedOn are generated; remove them from the source.`);
    }
  }
  return { problems, audit };
}
