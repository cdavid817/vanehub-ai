import { describe, expect, it } from "vitest";
import { i18n } from ".";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

describe("i18n resources", () => {
  it("keeps zh-CN and en key sets aligned", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(zhCN).sort());
  });

  it("provides representative page translations in both supported languages", async () => {
    await i18n.changeLanguage("zh-CN");
    expect(i18n.t("agents.title")).toBe("Agent 管理");
    expect(i18n.t("sdk.title")).toBe("SDK 依赖");
    expect(i18n.t("mcp.title")).toBe("MCP 服务器");
    expect(i18n.t("createSession.title")).toBe("创建会话");
    expect(i18n.t("chat.config.configure")).toBe("配置");

    await i18n.changeLanguage("en");
    expect(i18n.t("agents.title")).toBe("Agent Management");
    expect(i18n.t("sdk.title")).toBe("SDK Dependencies");
    expect(i18n.t("mcp.title")).toBe("MCP Servers");
    expect(i18n.t("createSession.title")).toBe("Create Session");
    expect(i18n.t("chat.config.configure")).toBe("Configure");
  });
});
