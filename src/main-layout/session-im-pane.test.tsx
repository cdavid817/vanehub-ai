// @vitest-environment jsdom

import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { activateAppLanguage } from "../i18n";
import type {
  ImConnectorLifecycle,
  ImConnectorView,
  ImSessionAccess,
  ImSessionBinding,
} from "../contracts/im";
import type { ImService } from "../services/im-service";
import { SessionImPane } from "./session-im-pane";

function serviceFixture(
  lifecycle: ImConnectorLifecycle,
  initialBinding: ImSessionBinding | null = null,
  initialAccess = true,
) {
  let binding = initialBinding;
  const accessBySession = new Map<string, boolean>([["session-1", initialAccess]]);
  let pairingAttempt = 0;
  const connector: ImConnectorView = {
    descriptor: { kind: "feishu", experimental: false, maxOutboundChars: 20_000, supportsQrAuthorization: false },
    config: { kind: "feishu", enabled: lifecycle !== "unconfigured", publicConfig: {} },
    hasCredentials: lifecycle !== "unconfigured",
    health: { kind: "feishu", generation: 1, lifecycle, updatedAt: "2026-08-13T00:00:00Z" },
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
    getSessionBinding: vi.fn(async (sessionId, selectedConnector) => ({
      access: {
        connector: selectedConnector,
        enabled: accessBySession.get(sessionId) ?? false,
        sessionId,
        updatedAt: "2026-08-13T00:00:00Z",
      },
      binding: sessionId === "session-1" ? binding : null,
      pendingConnector: null,
    })),
    setSessionAccess: vi.fn(async (sessionId, connector, enabled) => {
      accessBySession.set(sessionId, enabled);
      return { connector, enabled, sessionId, updatedAt: "2026-08-13T00:01:00Z" };
    }),
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

const activeBinding: ImSessionBinding = {
  connector: "feishu",
  sessionId: "session-1",
  state: "active",
  completionNotifications: false,
  createdAt: "2026-08-13T00:00:00Z",
  updatedAt: "2026-08-13T00:00:00Z",
};

describe("SessionImPane", () => {
  beforeEach(async () => activateAppLanguage("en"));

  it("defaults to off and enables Feishu controls from the keyboard", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", null, false);
    render(<SessionImPane service={service} sessionId="session-1" />);

    const accessSwitch = await screen.findByRole("switch", { name: "Enable Feishu for this session" });
    expect((accessSwitch as HTMLInputElement).checked).toBe(false);
    expect(screen.queryByRole("button", { name: "Feishu" })).toBeNull();
    accessSwitch.focus();
    await user.keyboard(" ");

    await waitFor(() => expect((accessSwitch as HTMLInputElement).checked).toBe(true));
    expect(service.setSessionAccess).toHaveBeenCalledWith("session-1", "feishu", true);
    expect(screen.getByRole("button", { name: "Feishu" })).toBeTruthy();
  });

  it("selects a ready connector and scopes access and pairing to it", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", null, false);
    const telegram: ImConnectorView = {
      descriptor: { kind: "telegram", experimental: false, maxOutboundChars: 4_096, supportsQrAuthorization: false },
      config: { kind: "telegram", enabled: true, publicConfig: {} },
      hasCredentials: true,
      health: { kind: "telegram", generation: 1, lifecycle: "connected", updatedAt: "2026-08-13T00:00:00Z" },
    };
    const feishu = (await service.listConnectors())[0];
    vi.mocked(service.listConnectors).mockResolvedValue([feishu, telegram]);
    render(<SessionImPane service={service} sessionId="session-1" />);

    const selector = await screen.findByRole("combobox", { name: "IM connector" });
    await user.selectOptions(selector, "telegram");
    const accessSwitch = await screen.findByRole("switch", { name: "Enable Telegram for this session" });
    await user.click(accessSwitch);

    await waitFor(() => expect(service.setSessionAccess).toHaveBeenCalledWith(
      "session-1",
      "telegram",
      true,
    ));
    await user.click(screen.getByRole("button", { name: "Telegram" }));
    expect(service.beginPairing).toHaveBeenCalledWith("session-1", "telegram", false);
  });

  it("keeps access off and reports a failed enable mutation", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", null, false);
    vi.mocked(service.setSessionAccess).mockRejectedValueOnce(new Error("native-enable-failed"));
    render(<SessionImPane service={service} sessionId="session-1" />);
    const accessSwitch = await screen.findByRole("switch", { name: "Enable Feishu for this session" });

    await user.click(accessSwitch);

    expect((await screen.findByRole("alert")).textContent).toContain("native-enable-failed");
    expect((accessSwitch as HTMLInputElement).checked).toBe(false);
    expect(screen.queryByRole("button", { name: "Feishu" })).toBeNull();
  });

  it("disables repeated access mutations while one is pending", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", null, false);
    const pending = deferred<ImSessionAccess>();
    vi.mocked(service.setSessionAccess).mockReturnValueOnce(pending.promise);
    render(<SessionImPane service={service} sessionId="session-1" />);
    const accessSwitch = await screen.findByRole("switch", { name: "Enable Feishu for this session" });

    await user.click(accessSwitch);
    await waitFor(() => expect((accessSwitch as HTMLInputElement).disabled).toBe(true));
    expect(service.setSessionAccess).toHaveBeenCalledOnce();
    await act(async () => {
      pending.resolve({
        connector: "feishu",
        enabled: true,
        sessionId: "session-1",
        updatedAt: "2026-08-13T00:02:00Z",
      });
      await pending.promise;
    });
    await waitFor(() => expect((accessSwitch as HTMLInputElement).disabled).toBe(false));
    expect((accessSwitch as HTMLInputElement).checked).toBe(true);
  });

  it("keeps access isolated when the selected session changes", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", null, true);
    const { rerender } = render(<SessionImPane service={service} sessionId="session-1" />);
    const accessSwitch = await screen.findByRole("switch", { name: "Enable Feishu for this session" });
    expect((accessSwitch as HTMLInputElement).checked).toBe(true);

    rerender(<SessionImPane service={service} sessionId="session-2" />);
    await waitFor(() => expect((accessSwitch as HTMLInputElement).checked).toBe(false));
    await user.click(accessSwitch);
    await waitFor(() => expect((accessSwitch as HTMLInputElement).checked).toBe(true));
    expect(service.setSessionAccess).toHaveBeenLastCalledWith("session-2", "feishu", true);

    rerender(<SessionImPane service={service} sessionId="session-1" />);
    await waitFor(() => expect((accessSwitch as HTMLInputElement).checked).toBe(true));
  });

  it("shows connector-unavailable guidance for an unbound session", async () => {
    const service = serviceFixture("error");
    render(<SessionImPane service={service} sessionId="session-1" />);

    expect(await screen.findByText("No IM platform is connected and available.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Feishu" })).toBeNull();
  });

  it("requires replacement intent and displays transient pairing guidance", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected");
    render(<SessionImPane service={service} sessionId="session-1" />);
    await screen.findByRole("button", { name: "Feishu" });

    await user.click(screen.getByRole("checkbox", { name: "Replace an existing connection after confirmation" }));
    await user.click(screen.getByRole("button", { name: "Feishu" }));

    expect(await screen.findByText("/bind ABCDEFGH")).toBeTruthy();
    expect(service.beginPairing).toHaveBeenCalledWith("session-1", "feishu", true);
    await user.click(screen.getByRole("button", { name: "Generate a new code" }));
    expect(await screen.findByText("/bind IJKLMNOP")).toBeTruthy();
    expect(service.cancelPairing).toHaveBeenCalledWith("session-1", "feishu");
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

  it("requires confirmation before disabling a bound session", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", activeBinding);
    render(<SessionImPane service={service} sessionId="session-1" />);
    const accessSwitch = await screen.findByRole("switch", { name: "Enable Feishu for this session" });

    await user.click(accessSwitch);
    expect(screen.getByText(/Disable Feishu access/)).toBeTruthy();
    expect(service.setSessionAccess).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Disable access" }));

    await waitFor(() => expect((accessSwitch as HTMLInputElement).checked).toBe(false));
    expect(service.setSessionAccess).toHaveBeenCalledWith("session-1", "feishu", false);
    expect(screen.queryByRole("button", { name: "Pause" })).toBeNull();
  });

  it("preserves a manual pause after access is disabled and re-enabled", async () => {
    const user = userEvent.setup();
    const service = serviceFixture("connected", activeBinding);
    render(<SessionImPane service={service} sessionId="session-1" />);
    await user.click(await screen.findByRole("button", { name: "Pause" }));
    expect(await screen.findByRole("button", { name: "Resume" })).toBeTruthy();
    const accessSwitch = screen.getByRole("switch", { name: "Enable Feishu for this session" });

    await user.click(accessSwitch);
    await user.click(screen.getByRole("button", { name: "Disable access" }));
    await waitFor(() => expect((accessSwitch as HTMLInputElement).checked).toBe(false));
    await user.click(accessSwitch);

    expect(await screen.findByRole("button", { name: "Resume" })).toBeTruthy();
    expect(service.setBindingPaused).toHaveBeenCalledTimes(1);
  });
});
