// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { activateAppLanguage } from "../i18n";
import type { ImConnectorLifecycle, ImConnectorView, ImSessionBinding } from "../contracts/im";
import type { ImService } from "../services/im-service";
import { SessionImPane } from "./session-im-pane";

function serviceFixture(lifecycle: ImConnectorLifecycle, initialBinding: ImSessionBinding | null = null) {
  let binding = initialBinding;
  let pairingAttempt = 0;
  const connector: ImConnectorView = {
    descriptor: { kind: "telegram", experimental: false, maxOutboundChars: 4096, supportsQrAuthorization: false },
    config: { kind: "telegram", enabled: lifecycle !== "unconfigured", publicConfig: {} },
    hasCredentials: lifecycle !== "unconfigured",
    health: { kind: "telegram", generation: 1, lifecycle, updatedAt: "2026-08-13T00:00:00Z" },
  };
  const service: ImService = {
    listConnectors: vi.fn(async () => [connector]),
    getRouting: vi.fn(async () => null),
    saveRouting: vi.fn(async (routing) => routing),
    saveConnector: vi.fn(),
    setConnectorEnabled: vi.fn(),
    restartConnector: vi.fn(),
    testConnector: vi.fn(),
    clearConnector: vi.fn(),
    resetBindings: vi.fn(),
    getSessionBinding: vi.fn(async () => ({ binding, pendingConnector: null })),
    beginPairing: vi.fn(async (sessionId, kind, replaceExisting = false) => {
      pairingAttempt += 1;
      return {
        code: pairingAttempt === 1 ? "ABCDEFGH" : "IJKLMNOP",
        connector: kind,
        expiresAt: new Date(Date.now() + 600_000).toISOString(),
        replaceExisting,
        sessionId,
      };
    }),
    cancelPairing: vi.fn(async () => true),
    setBindingPaused: vi.fn(async (_sessionId, paused) => {
      binding = { ...binding!, state: paused ? "paused" : "active" };
      return binding;
    }),
    setCompletionNotifications: vi.fn(async (_sessionId, enabled) => {
      binding = { ...binding!, completionNotifications: enabled };
      return binding;
    }),
    removeSessionBinding: vi.fn(async () => { binding = null; return true; }),
    subscribeLifecycle: vi.fn(async () => () => undefined),
    beginWeChatAuthorization: vi.fn(),
    pollWeChatAuthorization: vi.fn(),
    cancelWeChatAuthorization: vi.fn(),
  };
  return service;
}

const activeBinding: ImSessionBinding = {
  connector: "telegram",
  sessionId: "session-1",
  state: "active",
  completionNotifications: false,
  createdAt: "2026-08-13T00:00:00Z",
  updatedAt: "2026-08-13T00:00:00Z",
};

describe("SessionImPane", () => {
  beforeEach(async () => activateAppLanguage("en"));

  it("shows connector-unavailable guidance for an unbound session", async () => {
    const service = serviceFixture("error");
    render(<SessionImPane service={service} sessionId="session-1" />);

    expect(await screen.findByText("No IM platform is connected and available.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Telegram" })).toBeNull();
  });

  it("requires replacement intent and displays transient pairing guidance", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected");
    render(<SessionImPane service={service} sessionId="session-1" />);
    await screen.findByRole("button", { name: "Telegram" });

    await user.click(screen.getByRole("checkbox", { name: "Replace an existing connection after confirmation" }));
    await user.click(screen.getByRole("button", { name: "Telegram" }));

    expect(await screen.findByText("/bind ABCDEFGH")).toBeTruthy();
    expect(service.beginPairing).toHaveBeenCalledWith("session-1", "telegram", true);
    await user.click(screen.getByRole("button", { name: "Generate a new code" }));
    expect(await screen.findByText("/bind IJKLMNOP")).toBeTruthy();
    expect(service.cancelPairing).toHaveBeenCalledWith("session-1", "telegram");
    await user.click(screen.getByRole("button", { name: "Cancel pairing" }));
    await waitFor(() => expect(screen.queryByText("/bind IJKLMNOP")).toBeNull());
  });

  it("updates pause and notification state and confirms removal", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", activeBinding);
    render(<SessionImPane service={service} sessionId="session-1" />);
    await screen.findByRole("button", { name: "Pause" });

    await user.click(screen.getByRole("button", { name: "Pause" }));
    expect(await screen.findByRole("button", { name: "Resume" })).toBeTruthy();
    await user.click(screen.getByRole("checkbox", { name: "Completion notifications" }));
    expect(service.setCompletionNotifications).toHaveBeenCalledWith("session-1", true);
    await user.click(screen.getByRole("button", { name: "Remove" }));
    expect(screen.getByText(/Remove this session's IM connection/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Remove connection" }));
    await waitFor(() => expect(screen.getByText("Connect this session")).toBeTruthy());
  });
});
