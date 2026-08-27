import { createDesktopConfig } from "./wdio-shared.mjs";
import { externalSpecFiles } from "./spec-manifest.mjs";

// The specs that verify what no fixture can: a real package manager changing a real global install,
// a real host Python environment, a real SSH server. They keep the plain inherited PATH on purpose
// -- a fixture Agent here would defeat the only thing these specs exist to check.
//
// Never part of the required gate. `test-desktop.mjs` reports BLOCKED, not PASSED, when the
// prerequisites are absent, so an unconfigured runner cannot be mistaken for a passing one.
export const config = await createDesktopConfig({
  specDirectory: "specs",
  specFiles: externalSpecFiles(),
});
