import { createDesktopConfig } from "./wdio-shared.mjs";

// Ordered rather than globbed: the second spec launches a fresh application against the same
// application-data directory the first one wrote to, which is what makes the relaunch real.
export const config = await createDesktopConfig({
  specDirectory: "specs-settings-persistence",
  specFiles: ["change-setting.e2e.mjs", "verify-after-relaunch.e2e.mjs"],
});
