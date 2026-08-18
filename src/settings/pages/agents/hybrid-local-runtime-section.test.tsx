// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import { HybridLocalRuntimeSection } from "./hybrid-local-runtime-section";

describe("HybridLocalRuntimeSection", () => {
  it.each(["futuristic", "minimal"] as const)("renders accessible controls in %s theme at narrow width", async (theme) => {
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 390 });
    const service = createAgentServiceDouble({
      listHybridRoutingRules: vi.fn().mockResolvedValue([]),
      discoverLocalModelEndpoints: vi.fn().mockResolvedValue({ operationId: "web", candidates: [] }),
    });
    renderWithAppProviders(
      <HybridLocalRuntimeSection overview={{ profiles: [], activeProfileId: null }} service={service} onSaved={vi.fn()} />,
      { theme },
    );
    expect(await screen.findByRole("heading", { name: /Hybrid local model runtime|混合本地模型运行时/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Discover localhost|发现本机服务/ })).toBeTruthy();
    expect(document.documentElement.dataset.theme).toBe(theme);
  });
});
