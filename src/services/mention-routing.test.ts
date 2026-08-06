import { describe, expect, it } from "vitest";
import { parseHandoffMentions } from "./mention-routing";

const mentions = ["架构师", "代码审查", "实现者", "opus", "opus-45"];

function parse(text: string, self = "架构师") {
  return parseHandoffMentions({ text, mentions, selfMention: self, maxMentions: 2 });
}

describe("parseHandoffMentions", () => {
  it("routes a mention at the start of a line", () => {
    expect(parse("做完了。\n@代码审查 帮我看下").targets).toEqual(["代码审查"]);
  });

  // The rule that makes ordinary prose safe: writing about a teammate must not dispatch them.
  it("ignores a mention in the middle of a line", () => {
    expect(parse("做完了，让 @代码审查 看一下").targets).toEqual([]);
  });

  it("allows leading whitespace and markdown list or quote prefixes", () => {
    expect(parse("  @代码审查 看下").targets).toEqual(["代码审查"]);
    expect(parse("- @代码审查 看下").targets).toEqual(["代码审查"]);
    expect(parse("> @代码审查 看下").targets).toEqual(["代码审查"]);
    expect(parse("1. @代码审查 看下").targets).toEqual(["代码审查"]);
  });

  // An agent explaining how routing works must not accidentally trigger it.
  it("ignores mentions inside fenced code blocks", () => {
    const text = ["示例：", "```", "@代码审查 这样写", "```", "结束"].join("\n");
    expect(parse(text).targets).toEqual([]);
  });

  it("filters a self-mention instead of looping back", () => {
    expect(parse("@架构师 继续").targets).toEqual([]);
  });

  // Longest match first, or "@opus-45" silently routes to "@opus".
  it("prefers the longest matching handle", () => {
    expect(parse("@opus-45 上", "架构师").targets).toEqual(["opus-45"]);
  });

  it("requires a token boundary after the handle", () => {
    expect(parse("@代码审查者 看下").targets).toEqual([]);
  });

  it("routes several mentions on separate lines, preserving order", () => {
    expect(parse("@实现者 先做\n@代码审查 再看").targets).toEqual(["实现者", "代码审查"]);
  });

  it("caps the number of targets and reports the truncation", () => {
    const result = parse("@实现者 a\n@代码审查 b\n@opus c");
    expect(result.targets).toEqual(["实现者", "代码审查"]);
    expect(result.truncatedReason).toBe("too-many-mentions");
  });

  it("does not repeat a seat mentioned twice", () => {
    expect(parse("@代码审查 a\n@代码审查 b").targets).toEqual(["代码审查"]);
  });

  it("reports no reason when nothing was truncated", () => {
    expect(parse("@代码审查 看下").truncatedReason).toBeNull();
  });

  it("ignores an unknown handle", () => {
    expect(parse("@产品经理 看下").targets).toEqual([]);
  });
});
