// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { activateAppLanguage } from "../../../i18n";
import type { CliConnectionCheck, CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import { webCliEnvironmentSnapshots } from "../../../services/web-cli-environment-fixtures";

const checkCliConnection = vi.fn<(agentId: string, providerId: string | null) => Promise<CliConnectionCheck>>();
const openExternalUrl = vi.fn<(url: string) => Promise<void>>();

vi.mock("../../../services/runtime-agent-client", () => ({
  agentService: {
    checkCliConnection: (agentId: string, providerId: string | null) => checkCliConnection(agentId, providerId),
    openExternalUrl: (url: string) => openExternalUrl(url),
  },
}));

function fixture(agentId: string): CliEnvironmentSnapshot {
  const snapshot = webCliEnvironmentSnapshots().find((item) => item.agentId === agentId);
  if (!snapshot) throw new Error(`no fixture for ${agentId}`);
  return snapshot;
}

describe("CliConnectionActions", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await activateAppLanguage("zh-CN");
  });
  afterEach(cleanup);

  it("runs an explicit handshake for an installed ACP tool and renders the negotiation, not a sign-in state", async () => {
    const { CliConnectionActions } = await import("./cli-connection-actions");
    checkCliConnection.mockResolvedValue({
      agentId: "qwen-code",
      transport: "acp-stdio",
      protocolVersion: 1,
      loadSession: true,
      agentName: "qwen-code",
      agentVersion: "0.9.0",
      authMethods: [],
      elapsedMs: 42,
    });
    render(<CliConnectionActions snapshot={fixture("qwen-code")} />);

    fireEvent.click(screen.getByRole("button", { name: "检查连接" }));
    await waitFor(() => expect(checkCliConnection).toHaveBeenCalledWith("qwen-code", null));
    const report = await screen.findByTestId("cli-connection-report");
    expect(report.textContent).toContain("ACP v1 · 42 ms");
    expect(report.textContent).toContain("该程序支持");
    expect(report.textContent).toContain("未声明");
    expect(report.textContent).toContain("不代表已登录");
  });

  it("shows the failure reason and never treats a rejected check as a result", async () => {
    const { CliConnectionActions } = await import("./cli-connection-actions");
    checkCliConnection.mockRejectedValue(new Error("acp-protocol-version-unsupported"));
    render(<CliConnectionActions snapshot={fixture("kimi-cli")} />);

    fireEvent.click(screen.getByRole("button", { name: "检查连接" }));
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("acp-protocol-version-unsupported");
    expect(screen.queryByTestId("cli-connection-report")).toBeNull();
  });

  it("offers no handshake for a tool that is not installed or has no ACP transport, but keeps the sign-in guide", async () => {
    const { CliConnectionActions } = await import("./cli-connection-actions");
    // Qoder: ACP transport, nothing on PATH. iFlow: legacy, terminal only.
    for (const agentId of ["qoder-cli", "iflow-cli"]) {
      const { unmount } = render(<CliConnectionActions snapshot={fixture(agentId)} />);
      expect(screen.queryByRole("button", { name: "检查连接" })).toBeNull();
      expect(screen.getByRole("button", { name: "登录说明" })).toBeTruthy();
      unmount();
    }
    // Claude Code has neither: nothing is rendered at all.
    const { container } = render(<CliConnectionActions snapshot={fixture("claude-code")} />);
    expect(container.textContent).toBe("");
  });

  it("opens the vendor's documentation only through the external-link service, on a click", async () => {
    const { CliConnectionActions } = await import("./cli-connection-actions");
    openExternalUrl.mockResolvedValue(undefined);
    render(<CliConnectionActions snapshot={fixture("cursor-agent-cli")} />);

    expect(openExternalUrl).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "登录说明" }));
    await waitFor(() => expect(openExternalUrl).toHaveBeenCalledWith("https://cursor.com/docs/cli/installation"));

    openExternalUrl.mockRejectedValueOnce(new Error("Only HTTPS external URLs are allowed."));
    fireEvent.click(screen.getByRole("button", { name: "登录说明" }));
    expect((await screen.findByRole("alert")).textContent).toContain("HTTPS");
  });
});
