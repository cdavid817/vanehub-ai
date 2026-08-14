import { describe, expect, it } from "vitest";
import { composerMentionQuery, replaceComposerMention } from "./composer-mention";

describe("composer mention token", () => {
  it("extracts the trailing mention run", () => {
    expect(composerMentionQuery("@not")).toBe("not");
    expect(composerMentionQuery("look at @Utils")).toBe("utils");
    expect(composerMentionQuery("@first @second")).toBe("second");
    expect(composerMentionQuery("@")).toBe("");
  });

  it("reports no mention when the caret is not in one", () => {
    expect(composerMentionQuery("")).toBeNull();
    expect(composerMentionQuery("plain text")).toBeNull();
    expect(composerMentionQuery("@done and more text")).toBeNull();
  });

  it("replaces the token and preserves the preceding whitespace character", () => {
    expect(replaceComposerMention("look at @uti", "src/utils.rs")).toBe("look at @src/utils.rs ");
    expect(replaceComposerMention("@uti", "src/utils.rs")).toBe("@src/utils.rs ");
    expect(replaceComposerMention("line\n@uti", "handle")).toBe("line\n@handle ");
  });
});
