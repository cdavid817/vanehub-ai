import path from "node:path";
import process from "node:process";
import {
  cliFixtureDir,
  pathWithoutCompetingAgents,
  prepareCliFixture,
} from "./wdio-cli-fixture.mjs";
import { agentEvaluationSpecFiles } from "./spec-manifest.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

const mode = process.env.VANEHUB_AGENT_EVALUATION_MODE ?? "fixture-opencode";
const fixture = mode === "fixture-opencode";
let agentPath = process.env.PATH ?? "";
if (fixture) {
  await prepareCliFixture();
  agentPath = await pathWithoutCompetingAgents(cliFixtureDir);
  if (process.platform !== "win32") {
    // The POSIX fixture uses `#!/usr/bin/env node`. PATH isolation can remove Node's directory
    // when that directory also contains an unrelated managed Agent, so restore only the runtime
    // needed to execute the committed OpenCode fixture.
    agentPath = [cliFixtureDir, path.dirname(process.execPath), ...agentPath.split(path.delimiter)]
      .filter((entry, index, entries) => entries.indexOf(entry) === index)
      .join(path.delimiter);
  }
}

export const config = await createDesktopConfig({
  specDirectory: "specs-agent-evaluation",
  specFiles: agentEvaluationSpecFiles(mode),
  environment: {
    PATH: agentPath,
    VANEHUB_AGENT_EVALUATION_MODE: mode,
    VANEHUB_DESKTOP_LIVE_AGENTS: fixture ? "0" : "1",
    // The spec owns the credential boundary. The desktop child receives it only in the typed
    // profile command, never as an ambient environment value that a launched process could inherit.
    VANEHUB_ONEPIECE_API_KEY: "",
  },
  captureFailureScreenshots: fixture,
  captureServiceLogs: fixture,
  logLevel: fixture ? "info" : "silent",
  commandTimeout: 90_000,
  mochaTimeout: 12 * 60_000,
});
