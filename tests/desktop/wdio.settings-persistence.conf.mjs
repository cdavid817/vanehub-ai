import { createDesktopConfig } from "./wdio-shared.mjs";

// Ordered rather than globbed: the second spec launches a fresh application against the same
// application-data directory the first one wrote to, which is what makes the relaunch real.
const baseConfig = await createDesktopConfig({
  specDirectory: "specs-settings-persistence",
  specFiles: ["change-setting.e2e.mjs", "verify-after-relaunch.e2e.mjs"],
});

// The embedded macOS driver exits with the application and can need a moment before accepting the
// relaunch session. Retry only this restart-sensitive layer, preserving the same native data dir.
export const config = {
  ...baseConfig,
  specFileRetries: 2,
  specFileRetriesDelay: 5,
};
