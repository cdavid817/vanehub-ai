// @vitest-environment jsdom

import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
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
    expect(html).not.toContain("Preview");
  });

  it("reports update-available then restart-required status for its nav entry as a real check-and-install flow progresses (task 12.16)", async () => {
    const onStatusChange = vi.fn();
    const user = userEvent.setup();
    const queryClient = new QueryClient();
    render(
      <QueryClientProvider client={queryClient}>
        <AboutPage onStatusChange={onStatusChange} />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith(null));

    await user.click(await screen.findByRole("button", { name: "检查更新" }));
    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
      kind: "update-available",
      labelKey: "about.update.available",
      labelParams: { version: "0.2.0" },
    }));

    await user.click(screen.getByRole("button", { name: "下载并安装" }));
    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
      kind: "restart-required",
      labelKey: "about.update.readyRestart",
    }));
  });
});
