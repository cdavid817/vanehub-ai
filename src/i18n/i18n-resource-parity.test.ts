import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { i18n } from ".";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

function findDuplicateKeys(filePath: string): string[] {
  const raw = readFileSync(filePath, "utf8");
  const keys = [...raw.matchAll(/"([a-zA-Z0-9_.-]+)":\s*"/g)].map((match) => match[1]);
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const key of keys) {
    if (seen.has(key)) duplicates.add(key);
    seen.add(key);
  }
  return [...duplicates];
}

describe("i18n resources", () => {
  it("keeps zh-CN and en key sets aligned", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(zhCN).sort());
  });

  it("has no duplicate keys in the raw locale JSON source", () => {
    expect(findDuplicateKeys("src/i18n/locales/zh-CN.json")).toEqual([]);
    expect(findDuplicateKeys("src/i18n/locales/en.json")).toEqual([]);
  });

  it("provides representative page translations in both supported languages", async () => {
    await i18n.changeLanguage("zh-CN");
    expect(i18n.t("agents.title")).toBe("Agent 管理");
    expect(i18n.t("sdk.title")).toBe("SDK 依赖");
    expect(i18n.t("mcp.title")).toBe("MCP 服务器");
    expect(i18n.t("createSession.title")).toBe("创建会话");
    expect(i18n.t("chat.config.configure")).toBe("配置");
    expect(i18n.t("im.platform.weixin.name")).toBe("个人微信");
    expect(i18n.t("im.routing.title")).toBe("默认路由");
    expect(i18n.t("layout.activityBar.scheduledTasks")).toBe("定时任务");
    expect(i18n.t("loops.title")).toBe("循环工程");
    expect(i18n.t("loops.inspection.back")).toBe("返回循环工程");
    expect(i18n.t("loops.web.evidence.decisionReady")).toContain("独立验证者建议验收");

    await i18n.changeLanguage("en");
    expect(i18n.t("agents.title")).toBe("Agent Management");
    expect(i18n.t("sdk.title")).toBe("SDK Dependencies");
    expect(i18n.t("mcp.title")).toBe("MCP Servers");
    expect(i18n.t("createSession.title")).toBe("Create Session");
    expect(i18n.t("chat.config.configure")).toBe("Configure");
    expect(i18n.t("im.platform.weixin.name")).toBe("Personal WeChat");
    expect(i18n.t("im.routing.title")).toBe("Default Routing");
    expect(i18n.t("layout.activityBar.scheduledTasks")).toBe("Scheduled tasks");
    expect(i18n.t("loops.title")).toBe("Loops");
    expect(i18n.t("loops.inspection.back")).toBe("Back to Loop Center");
    expect(i18n.t("loops.web.evidence.decisionReady")).toContain("independent Verifier recommends acceptance");
  });
});
