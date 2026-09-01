// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import "../../i18n";
import { applyLifecycleUpdate, imErrorMessage, ImPage } from "./im-page";
import type { ImConnectorView } from "../../contracts/im";

const statusTelegramConnector: ImConnectorView = {
  descriptor: { kind: "telegram", supportsQrAuthorization: false, experimental: false, maxOutboundChars: 4096 },
  config: { kind: "telegram", enabled: true, publicConfig: {} },
  health: { kind: "telegram", lifecycle: "connected", generation: 1, updatedAt: "2026-01-01" },
  hasCredentials: true,
};

vi.mock("../../services/runtime-im-client", () => ({
  imService: {
    listConnectors: vi.fn(() => Promise.resolve([statusTelegramConnector])),
    subscribeLifecycle: vi.fn(() => Promise.resolve(() => undefined)),
  },
}));

const translate = ((key: string) => {
  const messages: Record<string, string> = {
    "im.errors.repositoryFailed": "IM 配置读取失败",
    "im.errors.repositoryUnavailable": "IM 配置存储暂不可用",
  };
  return messages[key] ?? key;
}) as Parameters<typeof imErrorMessage>[1];

describe("imErrorMessage", () => {
  it("maps communications repository errors to user-facing messages", () => {
    expect(imErrorMessage(new Error("communications-repository-failed"), translate)).toBe("IM 配置读取失败");
    expect(imErrorMessage("communications-repository-unavailable", translate)).toBe("IM 配置存储暂不可用");
  });

  it("preserves unknown errors for diagnostics", () => {
    expect(imErrorMessage(new Error("custom-im-error"), translate)).toBe("custom-im-error");
  });
});

describe("applyLifecycleUpdate", () => {
  const connector = {
    descriptor: { kind: "telegram", supportsQrAuthorization: false, experimental: false, maxOutboundChars: 4096 },
    config: { kind: "telegram", enabled: true, publicConfig: {} },
    health: { kind: "telegram", lifecycle: "connecting", generation: 4, updatedAt: "2026-01-01" },
    hasCredentials: true,
  } satisfies ImConnectorView;

  it("applies current-generation transitions and ignores stale events", () => {
    expect(applyLifecycleUpdate([connector], {
      kind: "telegram", lifecycle: "connected", generation: 4, updatedAt: "2026-01-02",
    })[0].health.lifecycle).toBe("connected");
    expect(applyLifecycleUpdate([connector], {
      kind: "telegram", lifecycle: "error", generation: 3, updatedAt: "2026-01-03",
    })[0]).toBe(connector);
  });
});

describe("IM settings structure", () => {
  it("keeps connector configuration independent from Agent and project routing", () => {
    // Plain CWD-relative paths, not `new URL(..., import.meta.url)`: jsdom (needed below for the
    // status-reporting test) shadows the global `URL` with its own implementation, which
    // `readFileSync` rejects with "The URL must be of scheme file".
    const page = readFileSync("src/settings/pages/im-page.tsx", "utf8");
    const row = readFileSync("src/settings/pages/im/im-connector-row.tsx", "utf8");

    expect(page).not.toContain("ImRoutingSection");
    expect(page).not.toContain("getRouting()");
    expect(page).not.toContain("listKnownProjects()");
    expect(row).not.toContain("routingReady");
  });
});

describe("ImPage status reporting (task 12.16)", () => {
  it("reports an error status for its nav entry once a refresh fails, and null while healthy under the desktop runtime", async () => {
    const { imService } = await import("../../services/runtime-im-client");
    const onStatusChange = vi.fn();
    const originalTauriInternals = window.__TAURI_INTERNALS__;
    // isWebRuntime (im-page.tsx) reads window.__TAURI_INTERNALS__ directly -- without it jsdom's
    // default "web-mock" runtime kind would always make the healthy/null case unreachable here.
    window.__TAURI_INTERNALS__ = {};
    try {
      render(<ImPage onStatusChange={onStatusChange} searchTerm="" />);

      await screen.findByText("Telegram");
      await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith(null));

      vi.mocked(imService.listConnectors).mockRejectedValueOnce(new Error("boom"));
      fireEvent.click(screen.getByRole("button", { name: "刷新" }));

      await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
        kind: "error",
        labelKey: "im.pageStatus.error",
      }));
    } finally {
      window.__TAURI_INTERNALS__ = originalTauriInternals;
    }
  });
});
