import { renderToString } from "react-dom/server";
import "../../i18n";
import { SettingsProvider } from "../settings-provider";
import { BasicSettingsPage } from "./basic-settings-page";
import { describe, expect, it } from "vitest";

describe("BasicSettingsPage", () => {
  it("renders log management policies and disables local open action in Web mock state", () => {
    const html = renderToString(
      <SettingsProvider>
        <BasicSettingsPage />
      </SettingsProvider>,
    );

    expect(html).toContain("日志管理");
    expect(html).toContain("启动与系统行为");
    expect(html).toContain("数据管理");
    expect(html).toContain("开机自启");
    expect(html).toContain("网络代理");
    expect(html).toContain("文件夹打开方式");
    expect(html).toContain("检测已安装程序");
    expect(html).toContain("NO_PROXY");
    expect(html).toContain("30 天");
    expect(html).toContain("error / warn / info / debug");
    expect(html).toContain("disabled");
  });
});
