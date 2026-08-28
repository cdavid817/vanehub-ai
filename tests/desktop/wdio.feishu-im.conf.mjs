import path from "node:path";
import process from "node:process";
import { cliFixtureDir, prepareCliFixture } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareCliFixture();

const baseConfig = await createDesktopConfig({
  specDirectory: "specs-feishu-im",
  captureFailureScreenshots: false,
  // The second spec starts a fresh native process against the same data directory. A glob cannot
  // guarantee that ordering, so it cannot prove restart persistence.
  specFiles: [
    "enable-and-exit.e2e.mjs",
    "session-access.e2e.mjs",
    "multi-agent.e2e.mjs",
    "resilience.e2e.mjs",
  ],
  environment: {
    PATH: `${cliFixtureDir}${path.delimiter}${process.env.PATH ?? ""}`,
    VANEHUB_FEISHU_IM_FIXTURE: "1",
  },
});

export const config = baseConfig;
