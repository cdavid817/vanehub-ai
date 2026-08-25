import { createDesktopConfig } from "./wdio-shared.mjs";

// Long-running large-granularity rehearsal: a three-seat group chat (codex-cli 架构师 →
// claude-code 实现者 → opencode 代码审查 → 实现者 rework) develops a real multi-file feature
// against a worktree of this repository. Real CLIs, real model calls, mechanical judging.
export const config = await createDesktopConfig({
  specDirectory: "specs-multi-agent-longrun",
});

// A real seat turn routinely runs past the shared 300s mocha cap (a rework that rewrites tests
// takes minutes on its own, and the toolchain judge runs after it) — and per-test
// `this.timeout()` does not take effect under this wdio/mocha combination. Give every stage the
// spec's worst-case budget at the framework level instead.
config.mochaOpts = { ...config.mochaOpts, timeout: 45 * 60 * 1000 };
