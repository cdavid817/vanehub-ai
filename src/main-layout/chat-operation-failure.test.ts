import { describe, expect, it } from "vitest";
import { createChatOperationFailureEvent } from "./chat-operation-failure";

describe("createChatOperationFailureEvent", () => {
  it("normalizes terminal chat failures for durable client logging", () => {
    expect(createChatOperationFailureEvent("MainLayout.sendMessage", new Error("request failed"))).toEqual({
      level: "error",
      kind: "critical-operation-failure",
      message: "request failed",
      source: "MainLayout.sendMessage",
    });
  });
});
