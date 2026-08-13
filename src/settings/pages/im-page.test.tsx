import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import "../../i18n";
import { applyLifecycleUpdate, imErrorMessage } from "./im-page";
import type { ImConnectorView } from "../../contracts/im";

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
    const page = readFileSync(new URL("./im-page.tsx", import.meta.url), "utf8");
    const row = readFileSync(new URL("./im/im-connector-row.tsx", import.meta.url), "utf8");

    expect(page).not.toContain("ImRoutingSection");
    expect(page).not.toContain("getRouting()");
    expect(page).not.toContain("listKnownProjects()");
    expect(row).not.toContain("routingReady");
  });
});
