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

// One or more spec file names, comma-separated. Diagnosis only: without it, reproducing a single
// spec's failure means a full sweep, and a twenty-minute round trip is what makes a flaky desktop
// spec cheaper to explain away than to investigate.
const requested = (process.env.VANEHUB_DESKTOP_SPEC ?? "")
  .split(",")
  .map((name) => name.trim())
  .filter(Boolean);
if (requested.length > 0) {
  process.stdout.write(`Desktop specs: restricted to ${requested.join(", ")}.\n`);
}

export const config = await createDesktopConfig({
  specDirectory: "specs",
  specFiles: requested.length > 0 ? requested : runFullSuite ? undefined : ["smoke.e2e.mjs"],
});
