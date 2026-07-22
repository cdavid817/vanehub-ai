import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import "../../i18n";
import { AboutPage } from "./about-page";

describe("AboutPage", () => {
  it("renders localized software details, GitHub link, changelog, and update action", () => {
    const queryClient = new QueryClient();
    const html = renderToString(<QueryClientProvider client={queryClient}><AboutPage /></QueryClientProvider>);

    expect(html).toContain("关于 VaneHub AI");
    expect(html).toContain("https://github.com/cdavid817/vanehub-ai");
    expect(html).toContain("最近变更");
    expect(html).toContain("检查更新");
    expect(html).toContain("产品定位");
    expect(html).toContain("软件详情");
    expect(html).not.toContain("本地 CLI 环境");
  });
});
