import process from "node:process";
import { createDesktopConfig } from "./wdio-shared.mjs";

// Keep the cross-platform CI gate focused on the stable smoke contract. The broader domain sweep
// remains opt-in while its host-dependent cases are promoted into the gate individually.
const runFullSuite = process.env.VANEHUB_DESKTOP_FULL_SUITE === "1" || !process.env.CI;
if (!runFullSuite) {
  process.stdout.write(
    "Desktop specs: gate run (smoke only). Set VANEHUB_DESKTOP_FULL_SUITE=1 for the full suite.\n",
  );
}

export const config = await createDesktopConfig({
  specDirectory: "specs",
  specFiles: runFullSuite ? undefined : ["smoke.e2e.mjs"],
});
