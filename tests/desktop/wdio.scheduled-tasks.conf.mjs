import path from "node:path";
import process from "node:process";
import { cliFixtureDir, prepareCliFixture } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareCliFixture();

// Agent discovery remains deterministic and credential-free while scheduled-task CRUD still
// crosses the rendered UI, Tauri commands, and isolated native SQLite database.
export const config = await createDesktopConfig({
  specDirectory: "specs-scheduled-tasks",
  environment: { PATH: `${cliFixtureDir}${path.delimiter}${process.env.PATH ?? ""}` },
});
