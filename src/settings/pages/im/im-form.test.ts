import { describe, expect, it } from "vitest";
import { compactCredentials, credentialFields, hasCompleteCredentials, validateRouting } from "./im-form";

describe("IM settings form", () => {
  it("requires both routing values", () => {
    expect(validateRouting("", "")).toEqual({ agentId: "required", projectPath: "required" });
    expect(validateRouting("codex", "D:\\code\\project")).toEqual({});
  });

  it("does not submit empty credential placeholders", () => {
    expect(compactCredentials({ botToken: "", appSecret: "   " })).toBeUndefined();
    expect(compactCredentials({ botToken: "  replacement-token  " })).toEqual({ botToken: "replacement-token" });
  });

  it("defines write-only fields for all credential-based platforms", () => {
    expect(Object.keys(credentialFields).sort()).toEqual(["dingtalk", "feishu", "telegram", "wecom"]);
    expect(hasCompleteCredentials("feishu", { appId: "id", appSecret: "secret" })).toBe(true);
    expect(hasCompleteCredentials("feishu", { appId: "id" })).toBe(false);
  });
});
