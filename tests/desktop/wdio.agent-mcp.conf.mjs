import path from "node:path";
import process from "node:process";
import { agentMcpFixtureDir, prepareAgentMcpFixtures } from "./wdio-agent-mcp-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareAgentMcpFixtures();

export const config = await createDesktopConfig({
  specDirectory: "specs-agent-mcp",
  environment: {
    PATH: `${agentMcpFixtureDir}${path.delimiter}${process.env.PATH ?? ""}`,
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
