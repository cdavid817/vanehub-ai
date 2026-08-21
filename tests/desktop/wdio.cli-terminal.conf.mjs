import path from "node:path";
import process from "node:process";
import { cliFixtureDir, prepareCliFixture } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareCliFixture();

// Ahead of the inherited PATH so the native runtime resolves the builtin `opencode` Agent to the
// fixture even on a host where the real one is installed. Every other layer keeps the plain PATH.
export const config = await createDesktopConfig({
  specDirectory: "specs-cli-terminal",
  environment: { PATH: `${cliFixtureDir}${path.delimiter}${process.env.PATH ?? ""}` },
});
