import { describe, expect, it } from "vitest";
import "../../i18n";
import { imErrorMessage } from "./im-page";

const translate = ((key: string) => {
  const messages: Record<string, string> = {
    "im.errors.repositoryFailed": "IM 配置读取失败",
    "im.errors.repositoryUnavailable": "IM 配置存储暂不可用",
  };
  return messages[key] ?? key;
}) as Parameters<typeof imErrorMessage>[1];

describe("imErrorMessage", () => {
  it("maps communications repository errors to user-facing messages", () => {
    expect(imErrorMessage(new Error("communications-repository-failed"), translate)).toBe("IM 配置读取失败");
    expect(imErrorMessage("communications-repository-unavailable", translate)).toBe("IM 配置存储暂不可用");
  });

  it("preserves unknown errors for diagnostics", () => {
    expect(imErrorMessage(new Error("custom-im-error"), translate)).toBe("custom-im-error");
  });
});
