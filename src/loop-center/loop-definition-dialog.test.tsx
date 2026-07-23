import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../i18n";
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
});
