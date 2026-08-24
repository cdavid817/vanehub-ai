import path from "node:path";
import process from "node:process";
import { cliFixtureDir, prepareCliFixture } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

await prepareCliFixture();

// The same fixture PATH the other session layers use: creating a session needs a deterministic
// Agent on PATH, and an installed real one would make this layer's result depend on the host. The
// Shells this layer opens are the host's own shell, not the fixture — that is the subject.
export const config = await createDesktopConfig({
  specDirectory: "specs-session-shell",
  environment: { PATH: `${cliFixtureDir}${path.delimiter}${process.env.PATH ?? ""}` },
});
