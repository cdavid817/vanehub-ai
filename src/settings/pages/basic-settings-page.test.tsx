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
    expect(html).toContain("网络代理");
    expect(html).toContain("NO_PROXY");
    expect(html).toContain("30 天");
    expect(html).toContain("error / warn / info / debug");
    expect(html).toContain("disabled");
  });
});
