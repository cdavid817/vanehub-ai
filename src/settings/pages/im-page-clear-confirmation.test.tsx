// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import "../../i18n";
import { ImPage } from "./im-page";
import type { ImConnectorView } from "../../contracts/im";

const telegramConnector: ImConnectorView = {
  descriptor: { kind: "telegram", supportsQrAuthorization: false, experimental: false, maxOutboundChars: 4096 },
  config: { kind: "telegram", enabled: true, publicConfig: {} },
  health: { kind: "telegram", lifecycle: "connected", generation: 1, updatedAt: "2026-01-01" },
  hasCredentials: true,
};

vi.mock("../../services/runtime-im-client", () => ({
  imService: {
    listConnectors: vi.fn(() => Promise.resolve([telegramConnector])),
    subscribeLifecycle: vi.fn(() => Promise.resolve(() => undefined)),
    clearConnector: vi.fn(() => Promise.resolve()),
  },
}));

describe("IM connector Clear requires confirmation (task 12.14)", () => {
  it("does not clear credentials until the confirmation is accepted, and clears them once it is", async () => {
    const { imService } = await import("../../services/runtime-im-client");
    render(<ImPage searchTerm="" />);

    await screen.findByText("Telegram");
    fireEvent.click(screen.getByText("Telegram"));
    const clear = await screen.findByRole("button", { name: "清除" });
    fireEvent.click(clear);

    const dialog = await screen.findByRole("dialog");
    expect(dialog.textContent).toContain("清除已保存的凭据");
    expect(imService.clearConnector).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(imService.clearConnector).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "清除" }));
    const secondDialog = await screen.findByRole("dialog");
    fireEvent.click(within(secondDialog).getByRole("button", { name: "确认" }));
    await waitFor(() => expect(imService.clearConnector).toHaveBeenCalledWith("telegram"));
  });
});
