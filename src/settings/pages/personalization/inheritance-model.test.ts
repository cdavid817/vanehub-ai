import { describe, expect, it } from "vitest";
import type { PersonalizationPolicy } from "../../../types/personalization";
import {
  agentLayerVaries,
  hasInstructionText,
  layersBelow,
  mergeOutcome,
} from "./inheritance-model";

function policy(overrides: Partial<PersonalizationPolicy> = {}): PersonalizationPolicy {
  return {
    scopeKind: "global",
    scopeKey: "",
    revision: 1,
    instructionMergeMode: "append",
    aboutUser: "",
    styleRules: "",
    memoryReadMode: "inherit",
    explicitSaveMode: "inherit",
    automaticExtractionMode: "inherit",
    globalMemoryAccessMode: "inherit",
    ...overrides,
  };
}

const STORE: PersonalizationPolicy[] = [
  policy({ scopeKind: "global", scopeKey: "", revision: 3, aboutUser: "Backend engineer." }),
  policy({ scopeKind: "agent", scopeKey: "onepiece", revision: 2, styleRules: "Be terse." }),
  policy({ scopeKind: "workspace", scopeKey: "ws-1", revision: 5, aboutUser: "npm only." }),
  policy({ scopeKind: "workspace-agent", scopeKey: "ws-1::onepiece", revision: 1 }),
];

describe("inheritance model", () => {
  it("puts nothing below the global layer", () => {
    expect(layersBelow({ scopeKind: "global" }, STORE)).toEqual([]);
  });

  it("puts global below an Agent layer", () => {
    const below = layersBelow({ scopeKind: "agent", agentId: "onepiece" }, STORE);

    expect(below.map((layer) => layer.scopeKind)).toEqual(["global"]);
    expect(below[0].aboutUser).toBe("Backend engineer.");
    expect(below[0].revision).toBe(3);
  });

  it("puts global, the Agent layer, and the workspace layer below a workspace-Agent layer", () => {
    const below = layersBelow(
      { scopeKind: "workspace-agent", agentId: "onepiece", workspaceKey: "ws-1" },
      STORE,
    );

    // Lowest precedence first, which is the order the spec fixes and the order they apply in.
    expect(below.map((layer) => layer.scopeKind)).toEqual(["global", "agent", "workspace"]);
  });

  it("lists only the Agent layer that the scope names", () => {
    const below = layersBelow(
      { scopeKind: "workspace-agent", agentId: "someone-else", workspaceKey: "ws-1" },
      STORE,
    );

    expect(below.map((layer) => layer.scopeKind)).toEqual(["global", "workspace"]);
  });

  it("omits a layer that has never been written rather than showing an empty one", () => {
    const below = layersBelow({ scopeKind: "agent", agentId: "onepiece" }, []);

    expect(below).toEqual([]);
  });

  it("says an Agent layer applies below a workspace layer without naming one", () => {
    // Listing none reads as "nothing else applies"; listing every Agent's would claim they all do.
    expect(agentLayerVaries({ scopeKind: "workspace", workspaceKey: "ws-1" })).toBe(true);
    expect(agentLayerVaries({ scopeKind: "global" })).toBe(false);
    expect(agentLayerVaries({ scopeKind: "workspace-agent", agentId: "a", workspaceKey: "w" })).toBe(false);
  });

  it("tells apart the four things saving can do", () => {
    expect(mergeOutcome("append", true)).toBe("appended");
    expect(mergeOutcome("replace", true)).toBe("replaced");
    expect(mergeOutcome("disabled", true)).toBe("nothing");
    expect(mergeOutcome("inherit", true)).toBe("inherited");
  });

  it("calls an empty layer inherited even when its mode says append", () => {
    // Text is what a mode acts on. Promising an append of nothing would describe an effect the
    // user will not see, and they would reasonably conclude the save failed.
    expect(mergeOutcome("append", false)).toBe("inherited");
    expect(mergeOutcome("replace", false)).toBe("inherited");
  });

  it("treats whitespace as no text", () => {
    expect(hasInstructionText({ aboutUser: "   ", styleRules: "\n" })).toBe(false);
    expect(hasInstructionText({ aboutUser: "", styleRules: "x" })).toBe(true);
  });
});
