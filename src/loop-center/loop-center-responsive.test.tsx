import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../i18n";
import { LoopCenter } from "./loop-center";

describe("LoopCenter responsive navigation", () => {
  it("renders labelled narrow-width drawer triggers and bounded panels", () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const html = renderToStaticMarkup(
      <QueryClientProvider client={queryClient}>
        <LoopCenter />
      </QueryClientProvider>,
    );

    expect(html).toContain('aria-controls="loop-navigation-drawer"');
    expect(html).toContain('aria-controls="loop-inspector-drawer"');
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain('title="打开循环列表"');
    expect(html).toContain('title="打开循环检查器"');
    expect(html).toContain('id="loop-navigation-drawer"');
    expect(html).toContain('id="loop-inspector-drawer"');
    expect(html).toContain("min-[1024px]:grid-cols-");
    expect(html).toContain("translate-x-full invisible min-[1024px]:visible");
  });
});
