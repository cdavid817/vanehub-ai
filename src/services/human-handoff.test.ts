import { describe, expect, it } from "vitest";
import { parseHumanHandoff, applyHumanHandoff } from "./human-handoff";

describe("parseHumanHandoff", () => {
  it("reads a blocking handoff", () => {
    expect(parseHumanHandoff("做完了。\n@用户 handoff 需要你定这个方案")).toBe("handoff");
  });

  it("reads an informational handoff", () => {
    expect(parseHumanHandoff("@用户 fyi 顺带一提，我改了配置")).toBe("fyi");
  });

  it("reads a completion handoff", () => {
    expect(parseHumanHandoff("@用户 done 全部完成")).toBe("done");
  });

  // Same line-leading rule as seat mentions; prose about the user must not stop the work.
  it("ignores a mid-line mention of the user", () => {
    expect(parseHumanHandoff("这个要问 @用户 handoff 一下")).toBeNull();
  });

  // Defaulting a bare mention to blocking would make agents afraid to mention the user at all.
  it("treats a bare user mention as informational", () => {
    expect(parseHumanHandoff("@用户 我改完了")).toBe("fyi");
  });

  it("returns null when the user is not mentioned", () => {
    expect(parseHumanHandoff("@代码审查 看下")).toBeNull();
  });
});

describe("applyHumanHandoff", () => {
  // The distinction the whole three-intent design exists for: only handoff interrupts.
  it("leaves the turn with the Agents for an informational handoff", () => {
    expect(applyHumanHandoff("fyi")).toEqual({
      turnHolder: "agents",
      roundComplete: false,
      startsWaiting: false,
    });
  });

  it("transfers the turn and starts the waiting clock for a blocking handoff", () => {
    expect(applyHumanHandoff("handoff")).toEqual({
      turnHolder: "human",
      roundComplete: false,
      startsWaiting: true,
    });
  });

  it("ends the round on completion without making the human owe a reply", () => {
    expect(applyHumanHandoff("done")).toEqual({
      turnHolder: "human",
      roundComplete: true,
      startsWaiting: false,
    });
  });
});
