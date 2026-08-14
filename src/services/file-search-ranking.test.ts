import { describe, expect, it } from "vitest";
import { normalizeFileSearchQuery, rankFileCandidates, scoreFileCandidate } from "./file-search-ranking";

const candidates = [
  { name: "utils.rs", path: "utils.rs" },
  { name: "my_utils.rs", path: "my_utils.rs" },
  { name: "helper.rs", path: "utils/helper.rs" },
  { name: "util.rs", path: "util.rs" },
];

describe("file search ranking", () => {
  it("scores exact above prefix above substring above path-only", () => {
    expect(scoreFileCandidate("utils.rs", "utils.rs", "utils.rs")).toBe(100);
    expect(scoreFileCandidate("utils", "utils.rs", "utils.rs")).toBe(80);
    expect(scoreFileCandidate("tils", "utils.rs", "utils.rs")).toBe(60);
    expect(scoreFileCandidate("src", "utils.rs", "src/utils.rs")).toBe(40);
    expect(scoreFileCandidate("nomatch", "utils.rs", "utils.rs")).toBeNull();
  });

  it("orders results by tier and excludes non-matches", () => {
    expect(rankFileCandidates("utils", candidates, 10).map((entry) => entry.path)).toEqual([
      "utils.rs",
      "my_utils.rs",
      "utils/helper.rs",
    ]);
  });

  it("breaks ties on depth then path order", () => {
    const tied = [
      { name: "target.rs", path: "deep/a/target.rs" },
      { name: "target.rs", path: "b/target.rs" },
      { name: "target.rs", path: "a/target.rs" },
    ];
    expect(rankFileCandidates("target.rs", tied, 10).map((entry) => entry.path)).toEqual([
      "a/target.rs",
      "b/target.rs",
      "deep/a/target.rs",
    ]);
  });

  it("matches against the relative path when the query carries a separator", () => {
    const nested = [
      { name: "ChatInputBox.tsx", path: "src/components/chat/ChatInputBox.tsx" },
      { name: "ChatInputBox.tsx", path: "src/other/ChatInputBox.tsx" },
    ];
    expect(rankFileCandidates("chat/chatinputbox", nested, 10).map((entry) => entry.path)).toEqual([
      "src/components/chat/ChatInputBox.tsx",
    ]);
  });

  it("normalizes case and backslashes before matching", () => {
    expect(normalizeFileSearchQuery("  SRC\\Components  ")).toBe("src/components");
    const mixed = [{ name: "Widget.tsx", path: "src/components/Widget.tsx" }];
    expect(rankFileCandidates("SRC\\components", mixed, 10)).toHaveLength(1);
  });

  it("browses shallowest first for an empty query", () => {
    const browse = [
      { name: "leaf.rs", path: "deep/nested/leaf.rs" },
      { name: "top.rs", path: "top.rs" },
    ];
    expect(rankFileCandidates("", browse, 10).map((entry) => entry.path)).toEqual([
      "top.rs",
      "deep/nested/leaf.rs",
    ]);
  });

  it("caps the returned result count", () => {
    expect(rankFileCandidates("utils", candidates, 2)).toHaveLength(2);
  });
});
