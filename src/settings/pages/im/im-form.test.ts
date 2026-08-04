import { describe, expect, it } from "vitest";
import {
  compactCredentials,
  credentialDraftAfterSave,
  credentialFields,
  hasCompleteCredentials,
  routingMatchesSaved,
  validateRouting,
} from "./im-form";

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

  it("uses persisted public fields and stored secrets when validating partial edits", () => {
    expect(hasCompleteCredentials("feishu", {}, { appId: "persisted-id" }, true)).toBe(true);
    expect(hasCompleteCredentials("feishu", { appId: "replacement-id" }, {}, true)).toBe(true);
    expect(hasCompleteCredentials("feishu", { appSecret: "replacement-secret" }, { appId: "persisted-id" })).toBe(true);
  });

  it("clears plaintext secrets after success or failure while retaining safe failed edits", () => {
    const draft = { appId: "replacement-id", appSecret: "plaintext-secret" };
    expect(credentialDraftAfterSave("feishu", draft, true)).toEqual({});
    expect(credentialDraftAfterSave("feishu", draft, false)).toEqual({ appId: "replacement-id" });
  });

  it("marks routing ready only after editable state matches the normalized save result", () => {
    const normalized = { agentId: "codex-cli", projectPath: "D:\\normalized" };
    expect(routingMatchesSaved(" codex-cli ", "D:\\normalized", normalized)).toBe(false);
    expect(routingMatchesSaved(normalized.agentId, normalized.projectPath, normalized)).toBe(true);
  });
});
