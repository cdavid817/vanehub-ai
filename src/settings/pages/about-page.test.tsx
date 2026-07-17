import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { AboutPage } from "./about-page";

describe("AboutPage", () => {
  it("renders localized software details, GitHub link, changelog, and update action", () => {
    const html = renderToString(<AboutPage />);

    expect(html).toContain("关于 VaneHub AI");
    expect(html).toContain("https://github.com/cdavid817/vanehub-ai");
    expect(html).toContain("最近变更");
    expect(html).toContain("检查更新");
    expect(html).toContain("Claude Code");
    expect(html).toContain("Tauri 2 Desktop");
  });
});
