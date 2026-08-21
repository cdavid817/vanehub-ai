import path from "node:path";
import process from "node:process";
import { cliFixtureDir, prepareCliFixture } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareCliFixture();

// Same fixture PATH as the CLI terminal layer: a session needs a deterministic Agent, and an
// installed real one would make this layer's result depend on the host.
export const config = await createDesktopConfig({
  specDirectory: "specs-dialogs",
  environment: { PATH: `${cliFixtureDir}${path.delimiter}${process.env.PATH ?? ""}` },
});
