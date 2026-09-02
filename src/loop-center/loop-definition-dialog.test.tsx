import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../i18n";
import { loopQueryKeys } from "../hooks/loop-query";
import { loopFixtureCases } from "../test/loop-fixtures";
import { LoopDefinitionDialog } from "./loop-definition-dialog";

describe("LoopDefinitionDialog", () => {
  it("renders an accessible four-step creation flow", () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const html = renderToStaticMarkup(<QueryClientProvider client={client}><LoopDefinitionDialog definition={null} onClose={() => undefined} onSaved={() => undefined} /></QueryClientProvider>);

    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-modal="true"');
    expect(html).toContain("1. 目标与范围");
    expect(html).toContain("2. 角色智能体");
    expect(html).toContain("3. 验证与限制");
    expect(html).toContain("4. 检查确认");
    expect(html).toContain("下一步");
  });

  it("retains unavailable saved project and branch selections visibly", () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(loopQueryKeys.projects, [{ path: "D:/available", displayName: "available", available: true, simulated: true }]);
    client.setQueryData(loopQueryKeys.branches("D:/missing"), [{ name: "main", kind: "local", available: true, simulated: true }]);
    const html = renderToStaticMarkup(<QueryClientProvider client={client}><LoopDefinitionDialog definition={loopFixtureCases.unavailableSelection()} onClose={() => undefined} onSaved={() => undefined} /></QueryClientProvider>);

    expect(html).toContain("D:/missing — 不可用");
    expect(html).toContain("deleted-branch — 不可用");
    expect(html).toContain("启用定义");
  });

  it("marks discovered but simulated project/branch choices distinctly from unavailable ones", () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(loopQueryKeys.projects, [
      { path: "D:/live", displayName: "live", available: true, simulated: false },
      { path: "D:/mocked", displayName: "mocked", available: true, simulated: true },
      { path: "D:/mocked-gone", displayName: "mocked-gone", available: false, simulated: true },
    ]);
    client.setQueryData(loopQueryKeys.branches(""), []);
    const html = renderToStaticMarkup(<QueryClientProvider client={client}><LoopDefinitionDialog definition={null} onClose={() => undefined} onSaved={() => undefined} /></QueryClientProvider>);

    // A real, reachable choice carries no suffix at all -- only a genuine gap earns one.
    expect(html).toContain(">live<");
    expect(html).not.toContain("live — ");
    expect(html).toContain("mocked — 模拟运行");
    expect(html).toContain("mocked-gone — 不可用, 模拟运行");
  });
});
