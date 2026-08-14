import { describe, expect, it } from "vitest";
import {
  composerMentionQuery,
  describeLineRange,
  fileReferenceId,
  formatMentionRange,
  parseComposerMention,
  replaceComposerMention,
} from "./composer-mention";

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

describe("composer mention line ranges", () => {
  it("splits a range suffix off the path", () => {
    expect(parseComposerMention("src/utils.rs:10-50")).toEqual({
      path: "src/utils.rs",
      startLine: 10,
      endLine: 50,
    });
  });

  it("treats a single line as a one-line range", () => {
    expect(parseComposerMention("src/utils.rs:42")).toEqual({
      path: "src/utils.rs",
      startLine: 42,
      endLine: 42,
    });
  });

  it("leaves a token without a suffix alone", () => {
    expect(parseComposerMention("src/utils.rs")).toEqual({ path: "src/utils.rs" });
  });

  it("keeps a colon that is not a range as part of the path", () => {
    for (const token of ["src/od:d.rs", "src/utils.rs:", "src/utils.rs:abc", "src/utils.rs:10-", ":10-50"]) {
      expect(parseComposerMention(token).startLine).toBeUndefined();
    }
  });

  it("carries malformed bounds through so one rule decides validity", () => {
    // The domain rejects these; parsing them here rather than swallowing them is what makes
    // the user see the rejection instead of a silently unfindable path.
    expect(parseComposerMention("a.rs:0-5")).toEqual({ path: "a.rs", startLine: 0, endLine: 5 });
    expect(parseComposerMention("a.rs:50-10")).toEqual({ path: "a.rs", startLine: 50, endLine: 10 });
  });

  it("searches on the path portion while a range is being typed", () => {
    expect(composerMentionQuery("@src/Utils.rs:10-50")).toBe("src/utils.rs");
    expect(composerMentionQuery("@src/utils.rs:1")).toBe("src/utils.rs");
  });

  it("round-trips a range through its rendered forms", () => {
    expect(formatMentionRange({ startLine: 10, endLine: 50 })).toBe(":10-50");
    expect(formatMentionRange({ startLine: 42, endLine: 42 })).toBe(":42");
    expect(formatMentionRange({})).toBe("");
    expect(describeLineRange({ startLine: 10, endLine: 50 })).toBe("10-50");
    expect(describeLineRange({ startLine: 42, endLine: 42 })).toBe("42");
    expect(describeLineRange({})).toBeNull();
  });

  it("gives two regions of one file distinct identities", () => {
    const first = fileReferenceId("src/utils.rs", { startLine: 10, endLine: 20 });
    const second = fileReferenceId("src/utils.rs", { startLine: 50, endLine: 60 });
    const whole = fileReferenceId("src/utils.rs", {});
    expect(new Set([first, second, whole]).size).toBe(3);
    expect(whole).toBe("src/utils.rs");
  });
});
