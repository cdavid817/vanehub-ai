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
const fixture = await createCliManagementFixture({
  root: path.join(process.env.VANEHUB_DESKTOP_RESULT_DIR ?? process.cwd(), "cli-fixture"),
});

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
    VANEHUB_CLI_FIXTURE_ROOT: fixture.root,
  },
});

export const config = {
  ...baseConfig,
  specFileRetries: 1,
  specFileRetriesDelay: 5,
};
