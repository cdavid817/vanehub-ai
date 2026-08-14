// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { BuiltinToolReadiness } from "../../../types/builtin-tools";
import { OnePieceToolReadiness } from "./onepiece-tool-readiness";

const snapshot: BuiltinToolReadiness = {
  agentId: "onepiece",
  observedAt: "2026-08-14T00:00:00Z",
  capabilities: [
    {
      capability: "browser",
      modes: [
        { mode: "read", state: "ready", reasonCode: null, simulated: false },
        { mode: "execute", state: "ready", reasonCode: null, simulated: false },
      ],
    },
    {
      capability: "delegation",
      modes: [
        {
          mode: "apply",
          state: "unavailable",
          reasonCode: "backend_unavailable",
          simulated: false,
        },
      ],
    },
  ],
};

describe("OnePieceToolReadiness", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("renders per-mode readiness and refreshes through AgentService", async () => {
    const getBuiltinToolReadiness = vi.fn(async () => snapshot);
    const service = createAgentServiceDouble({ getBuiltinToolReadiness });
    const { user } = renderWithAppProviders(<OnePieceToolReadiness service={service} />);

    expect(await screen.findByText("浏览器")).toBeTruthy();
    expect(screen.getByText("外部 AI 委托")).toBeTruthy();
    expect(screen.getByText("所需后端不可用")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "刷新诊断" }));
    await waitFor(() => expect(getBuiltinToolReadiness).toHaveBeenCalledTimes(2));
    expect(getBuiltinToolReadiness).toHaveBeenLastCalledWith("onepiece");
  });

  it("does not expose native error details", async () => {
    const service = createAgentServiceDouble({
      getBuiltinToolReadiness: async () => Promise.reject(new Error("C:/secret/tool.log")),
    });
    renderWithAppProviders(<OnePieceToolReadiness service={service} />);

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("无法加载工具就绪状态，未启动任何工具。");
    expect(alert.textContent).not.toContain("secret");
  });
});
