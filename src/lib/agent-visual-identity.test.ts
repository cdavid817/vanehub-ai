import { describe, expect, it } from "vitest";
import { getAgentVisualIdentity } from "./agent-visual-identity";

describe("agent visual identity", () => {
  it("keeps OnePiece identity stable across registry-derived surfaces", () => {
    const first = getAgentVisualIdentity("onepiece");
    const second = getAgentVisualIdentity("onepiece");

    expect(first.label).toBe("OnePiece");
    expect(first.tone).toContain("violet");
    expect(second).toBe(first);
    expect(getAgentVisualIdentity("custom-api").label).toBe("Agent");
  });
});
