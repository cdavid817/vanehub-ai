import { createDesktopConfig } from "./wdio-shared.mjs";

// Loop Engineering layer: the existing IPC domain spec first (definitions, validation, run-control
// guards), then the UI walkthrough that drives the real Loop centre — empty state, the four-step
// creation wizard, the overview and the preflight dialog.
export const config = await createDesktopConfig({
  specDirectory: "specs-loop",
  specFiles: ["../specs/domain-loop.e2e.mjs", "loop-engineering-ui.e2e.mjs"],
});
