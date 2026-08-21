import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { createDesktopConfig } from "./wdio-shared.mjs";

const fixtureDir = path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures", "cli");

// Same fixture PATH as the CLI terminal layer: a session needs a deterministic Agent, and an
// installed real one would make this layer's result depend on the host.
export const config = await createDesktopConfig({
  specDirectory: "specs-session-workspace",
  environment: { PATH: `${fixtureDir}${path.delimiter}${process.env.PATH ?? ""}` },
});
