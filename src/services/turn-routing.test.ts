import { describe, expect, it } from "vitest";
import { nextTurnTargets, routeUserMessage } from "./turn-routing";

const mentions = ["架构师", "代码审查", "实现者"];

describe("routeUserMessage", () => {
  it("routes to the seat named at the start of a line", () => {
    expect(routeUserMessage({ text: "@代码审查 看下", mentions, lastHolder: "架构师", firstSeat: "架构师" }))
      .toBe("代码审查");
  });

  // Matches conversation instinct: replying without naming anyone continues with whoever spoke last.
  it("routes an unaddressed message to whoever last held the turn", () => {
    expect(routeUserMessage({ text: "继续", mentions, lastHolder: "实现者", firstSeat: "架构师" }))
      .toBe("实现者");
  });

  it("falls back to the first seat when nobody has held the turn yet", () => {
    expect(routeUserMessage({ text: "开始吧", mentions, lastHolder: null, firstSeat: "架构师" }))
      .toBe("架构师");
  });
});

describe("nextTurnTargets", () => {
  it("extends the chain to the mentioned seat", () => {
    const result = nextTurnTargets({
      reply: "做完了。\n@代码审查 帮我看下",
      mentions,
      speaker: "实现者",
      depth: 1,
      maxDepth: 15,
      maxMentions: 2,
    });
    expect(result.targets).toEqual(["代码审查"]);
    expect(result.endedReason).toBeNull();
  });

  it("stops at the configured chain depth and says why", () => {
    const result = nextTurnTargets({
      reply: "@代码审查 继续",
      mentions,
      speaker: "实现者",
      depth: 15,
      maxDepth: 15,
      maxMentions: 2,
    });
    expect(result.targets).toEqual([]);
    expect(result.endedReason).toBe("max-depth");
  });

  it("reports truncation when a reply names more seats than allowed", () => {
    const result = nextTurnTargets({
      reply: "@架构师 a\n@代码审查 b",
      mentions,
      speaker: "实现者",
      depth: 1,
      maxDepth: 15,
      maxMentions: 1,
    });
    expect(result.targets).toHaveLength(1);
    expect(result.endedReason).toBe("too-many-mentions");
  });

  // A round that ends without a mention is the normal case, not an error.
  it("ends the chain quietly when the reply names nobody", () => {
    const result = nextTurnTargets({
      reply: "做完了。",
      mentions,
      speaker: "实现者",
      depth: 1,
      maxDepth: 15,
      maxMentions: 2,
    });
    expect(result.targets).toEqual([]);
    expect(result.endedReason).toBeNull();
  });
});
