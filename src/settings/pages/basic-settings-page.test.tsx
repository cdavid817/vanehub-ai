// @vitest-environment jsdom

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "../../i18n";
import { settingsService } from "../../services/runtime-settings-client";
import { SettingsProvider } from "../settings-provider";
import { BasicSettingsPage } from "./basic-settings-page";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("BasicSettingsPage", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders log management policies and disables local open action in Web mock state", async () => {
    const { container } = render(
      <SettingsProvider>
        <BasicSettingsPage />
      </SettingsProvider>,
    );
    await screen.findByText("日志管理");
    const html = container.innerHTML;

    expect(html).toContain("日志管理");
    expect(html).toContain("启动与窗口");
    expect(html).toContain("数据与存储");
    expect(html).toContain("设置持久化与运行时说明");
    expect(html).toContain("开机自启");
    expect(html).toContain("网络代理");
    expect(html).toContain("默认打开方式");
    expect(html).toContain("检测已安装程序");
    expect(html).toContain("NO_PROXY");
    expect(html).toContain("30 天");
    expect(html).toContain("error / warn / info / debug");
    expect(html).toContain("disabled");
    expect(screen.getByRole("combobox", { name: "应用语言" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "简体中文" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "English" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "繁體中文" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "日本語" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "한국어" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "主题" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "字体大小" })).toBeTruthy();
    expect(html).toContain("打开项目文件和文件夹时优先使用的程序");

    const commonIndex = html.indexOf("常用设置");
    const startupIndex = html.indexOf("启动与窗口");
    const workspaceIndex = html.indexOf("工作区");
    const advancedIndex = html.indexOf("高级配置");
    expect(commonIndex).toBeLessThan(startupIndex);
    expect(startupIndex).toBeLessThan(workspaceIndex);
    expect(workspaceIndex).toBeLessThan(advancedIndex);

    const advanced = screen.getByText("高级配置").closest("details");
    expect(advanced?.open).toBe(false);
  });

  it("persists the default project directory through the settings provider", async () => {
    const user = userEvent.setup();
    render(
      <SettingsProvider>
        <BasicSettingsPage />
      </SettingsProvider>,
    );

    const input = await screen.findByRole("textbox", { name: "默认项目目录" });
    await user.clear(input);
    await user.type(input, "D:\\Projects");
    await user.tab();

    await waitFor(() => {
      const stored = JSON.parse(window.localStorage.getItem("vanehub.appSettings") ?? "{}") as { defaultFolderPath?: string };
      expect(stored.defaultFolderPath).toBe("D:\\Projects");
    });
  });

  it("switches language immediately and restores the persisted Web setting", async () => {
    const user = userEvent.setup();
    const firstRender = render(
      <SettingsProvider>
        <BasicSettingsPage />
      </SettingsProvider>,
    );

    const language = await screen.findByRole("combobox", { name: "应用语言" });
    await user.selectOptions(language, "ja");

    await waitFor(() => {
      expect(document.documentElement.lang).toBe("ja");
      const stored = JSON.parse(window.localStorage.getItem("vanehub.appSettings") ?? "{}") as { applicationLanguage?: string };
      expect(stored.applicationLanguage).toBe("ja");
    }, { timeout: 10_000 });
    await screen.findByRole("heading", { name: "基本構成" });

    firstRender.unmount();
    render(
      <SettingsProvider>
        <BasicSettingsPage />
      </SettingsProvider>,
    );

    await screen.findByRole("heading", { name: "基本構成" });
    expect((screen.getByRole("combobox", { name: "アプリケーション言語" }) as HTMLSelectElement).value).toBe("ja");
  });

  it("keeps settings unchanged when reset confirmation is cancelled", async () => {
    const user = userEvent.setup();
    const stored = JSON.stringify({ defaultFolderPath: "D:\\Keep" });
    window.localStorage.setItem("vanehub.appSettings", stored);
    render(
      <SettingsProvider>
        <BasicSettingsPage />
      </SettingsProvider>,
    );

    await user.click(await screen.findByRole("button", { name: "重置默认" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("确定要将所有基础配置恢复为默认值吗？此操作会覆盖当前设置。")).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "取消" }));

    expect(window.localStorage.getItem("vanehub.appSettings")).toBe(stored);
  });

  it("shows the failure on the theme row alone, not a page-wide state (task 12.10)", async () => {
    // Rollback-on-failure itself is pre-existing behavior in `saveSetting`, not something this
    // task built -- this test is scoped to what task 12.10 actually added: the row-level pending
    // and error display, not re-verifying the rollback's own correctness under concurrent saves
    // from other sections (a separate, real question this test does not need to settle).
    const user = userEvent.setup();
    // Conditional on the call's own key, not call order -- other sections may save their own
    // settings on mount, and a plain `mockRejectedValueOnce` would silently reject whichever of
    // those happens to call first instead of the theme change this test actually exercises.
    // Non-theme keys pass through to the real implementation so the rest of the page still works.
    const realSaveSetting = settingsService.saveSetting.bind(settingsService);
    const saveSpy = vi.spyOn(settingsService, "saveSetting").mockImplementation((input) => {
      if (input.key === "theme") return Promise.reject(new Error("boom"));
      return realSaveSetting(input);
    });
    const { unmount } = render(
      <SettingsProvider>
        <BasicSettingsPage />
      </SettingsProvider>,
    );

    const theme = await screen.findByRole("combobox", { name: "主题" }) as HTMLSelectElement;
    const language = screen.getByRole("combobox", { name: "应用语言" }) as HTMLSelectElement;
    await user.selectOptions(theme, "简约风");

    // Pending, then error, both scoped to this one row -- an unrelated row is never disabled by
    // another row's own in-flight or failed save (design.md Decision 11: "mutation 只禁用目标动作").
    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("boom"));
    expect(language.disabled).toBe(false);

    saveSpy.mockRestore();
    unmount();
  });

  it("reports a null status once Node is available, then an error status when a save fails (task 12.16)", async () => {
    const user = userEvent.setup();
    const nodeInfoSpy = vi.spyOn(settingsService, "getNodeInfo").mockResolvedValue({
      available: true,
      path: "/usr/local/bin/node",
      reason: null,
      version: "20.11.0",
    });
    const onStatusChange = vi.fn();
    const { unmount } = render(
      <SettingsProvider>
        <BasicSettingsPage onStatusChange={onStatusChange} />
      </SettingsProvider>,
    );

    const theme = await screen.findByRole("combobox", { name: "主题" }) as HTMLSelectElement;
    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith(null));

    const realSaveSetting = settingsService.saveSetting.bind(settingsService);
    const saveSpy = vi.spyOn(settingsService, "saveSetting").mockImplementation((input) => {
      if (input.key === "theme") return Promise.reject(new Error("boom"));
      return realSaveSetting(input);
    });
    await user.selectOptions(theme, "简约风");

    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
      kind: "error",
      labelKey: "basic.status.error",
    }));

    saveSpy.mockRestore();
    nodeInfoSpy.mockRestore();
    unmount();
  });
});
