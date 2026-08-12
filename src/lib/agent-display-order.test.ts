import { describe, expect, it } from "vitest";
import {
  createSessionCliPriority,
  orderByAgentPriority,
  settingsAgentPriority,
} from "./agent-display-order";

describe("Agent display ordering", () => {
  it("defines the requested settings and create-session priorities", () => {
    expect(settingsAgentPriority).toEqual([
      "claude-code",
      "codex-cli",
      "opencode",
      "antigravity-cli",
      "gemini-cli",
      "onepiece",
    ]);
    expect(createSessionCliPriority).toEqual(settingsAgentPriority.slice(0, 5));
  });

  it("orders any supported subset without synthesizing missing entries", () => {
    expect(orderByAgentPriority(["gemini-cli", "claude-code", "opencode"], (id) => id))
      .toEqual(["claude-code", "opencode", "gemini-cli"]);
  });

  it("keeps unknown Agents in their original relative order after known entries", () => {
    const items = ["custom-z", "onepiece", "custom-a", "codex-cli"];
    expect(orderByAgentPriority(items, (id) => id))
      .toEqual(["codex-cli", "onepiece", "custom-z", "custom-a"]);
  });
});
