import path from "node:path";
import process from "node:process";
import { cliFixtureDir, prepareCliFixture } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareCliFixture();

const runId = process.env.VANEHUB_TEST_RUN_ID;
if (!runId) throw new Error("The live Feishu run context is unavailable.");

const credentialProfile = `desktop-live-${runId}`;

const baseConfig = await createDesktopConfig({
  specDirectory: "specs-feishu-live",
  specFiles: [
    "credential-safety.e2e.mjs",
    "operator-inbound.e2e.mjs",
    "restart-and-invalid.e2e.mjs",
    "credential-cleanup.e2e.mjs",
  ],
  captureFailureScreenshots: false,
  captureServiceLogs: false,
  commandTimeout: 90_000,
  mochaTimeout: 45 * 60_000,
  logLevel: "silent",
  environment: {
    PATH: `${cliFixtureDir}${path.delimiter}${process.env.PATH ?? ""}`,
    VANEHUB_FEISHU_LIVE_QUALIFICATION: "1",
    VANEHUB_IM_CREDENTIAL_PROFILE: credentialProfile,
    VANEHUB_FEISHU_TEST_TENANT: "",
    VANEHUB_FEISHU_APP_ID: "",
    VANEHUB_FEISHU_APP_SECRET: "",
    VANEHUB_FEISHU_TEST_CHAT_ID: "",
  },
});

export const config = baseConfig;
