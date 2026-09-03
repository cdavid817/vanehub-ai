import path from "node:path";
import process from "node:process";
import { agentMcpFixtureDir, prepareAgentMcpFixtures } from "./wdio-agent-mcp-fixture.mjs";
import { pathWithoutCompetingAgents } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareAgentMcpFixtures();
// The fixture must be the only installation, not merely the first: a real one left reachable makes
// two sources with the fixture in front, which the launch resolver refuses as PATH shadowing.
const fixturePath = await pathWithoutCompetingAgents(agentMcpFixtureDir);

export const config = await createDesktopConfig({
  specDirectory: "specs-agent-mcp",
  environment: {
    PATH: fixturePath,
    VANEHUB_CLI_FIXTURE_ROOT: agentMcpFixtureDir,
    ALL_PROXY: "",
    all_proxy: "",
    NO_PROXY: "127.0.0.1,localhost",
    no_proxy: "127.0.0.1,localhost",
    VANEHUB_MCP_AGENT_EVIDENCE_DIR: path.join(
      process.env.VANEHUB_DESKTOP_RESULT_DIR,
      "agent-mcp-evidence",
    ),
  },
});
