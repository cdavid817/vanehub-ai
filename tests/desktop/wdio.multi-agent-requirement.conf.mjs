import { createDesktopConfig } from "./wdio-shared.mjs";

// A real-requirement rehearsal layer: two seats (codex-cli 架构师 → claude-code 实现者) drive an
// actual product change against a disposable worktree of this repository, and the harness — not
// the Agents — judges the result. Uses the plain host PATH: the seats must be the real CLIs.
//
// Opt-in only, never part of `all`: it spends real model tokens and needs both CLIs authenticated
// on the host. When running beside another desktop e2e session on the same machine, export BOTH
// VANEHUB_WEBDRIVER_PORT and TAURI_WEBDRIVER_PORT to a private port — the tauri service's
// direct-eval channel (browser.tauri.execute) reads only the latter and defaults to 4445.
export const config = await createDesktopConfig({
  specDirectory: "specs-multi-agent-requirement",
});

// Real model turns can exceed the shared 300s mocha cap, and per-test `this.timeout()` does not
// take effect under this wdio/mocha combination; raise the framework-level cap to the layer's
// worst-case stage budget.
config.mochaOpts = { ...config.mochaOpts, timeout: 25 * 60 * 1000 };
