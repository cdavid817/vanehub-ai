// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { webAgentClient } from "../../services/web-agent-client";
import { resetWebLspMockStateForTest } from "../../services/web-lsp-client";
import { renderWithAppProviders } from "../../test/render";
import { CodeIntelligencePage } from "./code-intelligence-page";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

describe("CodeIntelligencePage", () => {
  beforeEach(async () => {
    invokeMock.mockClear();
    resetWebLspMockStateForTest();
    await activateAppLanguage("zh-CN");
  });

  it("owns every LSP setting after it is separated from Agent configuration", async () => {
    renderWithAppProviders(
      <CodeIntelligencePage
        isActive
        navigationTarget={null}
        onNavigate={vi.fn()}
        searchTerm=""
        service={webAgentClient}
      />,
    );

    expect(screen.getByRole("heading", { name: "代码智能" })).toBeTruthy();
    expect(await screen.findByRole("heading", { name: "语言服务器智能" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "受信任的工作区" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "测试语言服务器" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "运行状态" })).toBeTruthy();
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
