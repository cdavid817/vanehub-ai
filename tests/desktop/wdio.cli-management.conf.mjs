import { writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { createCliManagementFixture } from "./cli-management-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

/**
 * The native CLI Management layer.
 *
 * The fixture is built before the application starts and its directories go on the front of PATH,
 * so the runtime's own discovery, probing, planning, and mutation run on their production paths
 * against binaries this file wrote. Nothing here reaches a real npm, a real WinGet, a vendor URL,
 * a credential store, or the user's database -- and `cli-side-effect-guard.mjs` checks that
 * afterwards rather than trusting it.
 */
// Inside the run root, next to the isolated application data, so disposal takes it with everything
// else. The relocated home below collects WebView2's user-data folder, which has no business in an
// evidence directory that CI uploads.
const runRoot = process.env.VANEHUB_APP_DATA_DIR
  ? path.dirname(process.env.VANEHUB_APP_DATA_DIR)
  : process.cwd();
const fixture = await createCliManagementFixture({ root: path.join(runRoot, "cli-fixture") });

// Handed to the specs through a file rather than an environment variable, because the spec process
// and this config process are the same process only by accident of the runner.
await writeFile(
  path.join(process.env.VANEHUB_DESKTOP_RESULT_DIR ?? process.cwd(), "cli-fixture.json"),
  `${JSON.stringify(fixture, null, 2)}\n`,
  "utf8",
);

const baseConfig = await createDesktopConfig({
  specDirectory: "specs-cli-management",
  // Ordered: the second spec launches a fresh application against the same data directory, which
  // is what makes "the snapshot survived a restart" a claim about SQLite rather than about React.
  specFiles: ["cli-lifecycle.e2e.mjs", "cli-persistence.e2e.mjs"],
  environment: {
    PATH: fixture.pathValue,
    // Windows resolves `npm.cmd` through PATHEXT; without it the fake package manager is invisible
    // to the process gateway even though its directory is first on PATH.
    ...(fixture.pathext ? { PATHEXT: fixture.pathext } : {}),
    // PATH alone does not isolate discovery: it also enumerates known locations under the user's
    // home. Without these the layer finds whatever the developer has installed and reports it.
    ...fixture.homeEnvironment,
    VANEHUB_CLI_FIXTURE_ROOT: fixture.root,
  },
});

// No retries. The fixture is mutated by design -- an upgrade rewrites the version a fake CLI
// reports -- so a second attempt runs against a host the first attempt already changed, and neither
// a pass nor a failure from it says anything about the code.
export const config = baseConfig;
