import { createDesktopConfig } from "./wdio-shared.mjs";

// Long-running large-granularity rehearsal: a three-seat group chat (codex-cli 架构师 →
// claude-code 实现者 → opencode 代码审查 → 实现者 rework) develops a real multi-file feature
// against a worktree of this repository. Real CLIs, real model calls, mechanical judging.
export const config = await createDesktopConfig({
  specDirectory: "specs-multi-agent-longrun",
});
