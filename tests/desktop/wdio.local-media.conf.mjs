import process from "node:process";
import { createDesktopConfig } from "./wdio-shared.mjs";
import { localMediaFixtureEnvironment, prepareLocalMediaFixture } from "./wdio-local-media-fixture.mjs";

const resultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR;
if (!resultDir) throw new Error("Desktop result directory is required.");

// Prepared before the application starts: the runtime validates every one of these paths during
// bootstrap and aborts with a configuration error rather than falling back, so a tree built late
// would take the application down instead of failing this configuration.
const manifest = prepareLocalMediaFixture(resultDir);

/**
 * The deterministic local-media layer.
 *
 * Its spec directory is deliberately outside `specs/`: the ordinary layers run the same artifact
 * without any of these variables set, and a glob that reached these specs would run them against
 * the production assembly, where a microphone and three inference engines do not exist.
 */
export const config = await createDesktopConfig({
  specDirectory: "specs-local-media",
  environment: localMediaFixtureEnvironment(manifest),
});
