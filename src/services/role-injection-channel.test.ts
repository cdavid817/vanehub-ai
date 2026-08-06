import { describe, expect, it } from "vitest";
import { roleInjectionChannel } from "./role-injection-channel";

describe("roleInjectionChannel", () => {
  // These two survive context compaction, which is the whole reason to prefer them.
  it("reports the native system-prompt channel for agents that have one", () => {
    expect(roleInjectionChannel("claude-code")).toEqual({ kind: "native", compactionImmune: true });
    expect(roleInjectionChannel("codex-cli")).toEqual({ kind: "native", compactionImmune: true });
  });

  // Degrading silently would let a long session quietly drop the role and nobody would know.
  it("degrades to per-turn injection and says it is not compaction-immune", () => {
    expect(roleInjectionChannel("gemini-cli")).toEqual({ kind: "per-turn", compactionImmune: false });
    expect(roleInjectionChannel("some-unknown-agent")).toEqual({
      kind: "per-turn",
      compactionImmune: false,
    });
  });
});
