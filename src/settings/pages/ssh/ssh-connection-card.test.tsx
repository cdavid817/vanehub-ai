// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import type { SshConnection } from "../../../types/ssh-connection";
import { SshConnectionCard } from "./ssh-connection-card";

function fixture(overrides: Partial<SshConnection> = {}): SshConnection {
  return {
    id: "ssh-1",
    name: "构建服务器",
    host: "build.example.com",
    port: 22,
    user: "vane",
    defaultPath: "/srv/app",
    authMode: "key",
    keyPath: "/home/vane/.ssh/id_ed25519",
    hasPassword: false,
    revision: 1,
    hostTrust: null,
    testStatus: "not-tested",
    lastConnectedAt: null,
    lastError: null,
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
    ...overrides,
  };
}

function renderCard(overrides: Partial<SshConnection> = {}) {
  return render(
    <SshConnectionCard
      connection={fixture(overrides)}
      deleteState={undefined}
      onDelete={vi.fn()}
      onEdit={vi.fn()}
      onTest={vi.fn()}
      testState={undefined}
    />,
  );
}

describe("SshConnectionCard", () => {
  it("renders the connection's name, host summary, and status badge", () => {
    renderCard();
    expect(screen.getByText("构建服务器")).toBeTruthy();
    expect(screen.getByText("未测试")).toBeTruthy();
    expect(screen.getByRole("button", { name: "构建服务器的操作" })).toBeTruthy();
  });

  /**
   * 20.15: this card had zero test coverage before this pass. Its name/host-summary/path all
   * already carry `truncate` (unlike work-board-card.tsx's title, which did not -- see that file's
   * own 20.15 fix) -- these prove that pre-existing mechanism actually holds for the long-string
   * classes the task names, next to the status badge and the row's ActionMenu trigger, rather than
   * leaving it an unverified claim.
   */
  describe("long content safety (20.15)", () => {
    const GERMAN_LIKE_NAME = "Konfigurationsverwaltungsoberflächenkomponente";
    const CJK_NAME = "这是一个非常非常非常长的连接名称用来验证界面不会与状态徽章或操作菜单重叠";
    const LONG_HOST = "ip-10-42-17-233.ap-northeast-1.compute.internal.production.example-corp.internal";

    it("truncates a long German-like connection name next to the status badge and row actions", () => {
      renderCard({ name: GERMAN_LIKE_NAME });
      const heading = screen.getByText(GERMAN_LIKE_NAME);
      expect(heading.tagName).toBe("H3");
      expect(heading.className).toContain("truncate");
      // Both neighbors are real, separate elements -- not swallowed or displaced.
      expect(screen.getByText("未测试")).toBeTruthy();
      expect(screen.getByRole("button", { name: `${GERMAN_LIKE_NAME}的操作` })).toBeTruthy();
    });

    it("truncates a long CJK connection name the same way", () => {
      renderCard({ name: CJK_NAME });
      expect(screen.getByText(CJK_NAME).className).toContain("truncate");
    });

    it("truncates a long real host label in the user@host:port summary line", () => {
      renderCard({ host: LONG_HOST });
      const summary = screen.getByTitle(`vane@${LONG_HOST}:22`);
      expect(summary.className).toContain("truncate");
    });

    it("truncates a long real default path", () => {
      const longPath = "/srv/apps/monorepo/packages/backend-service/deploy/releases/current/config";
      renderCard({ defaultPath: longPath });
      expect(screen.getByTitle(longPath).className).toContain("truncate");
    });
  });

  /**
   * 20.16: host/user/path are all externally sourced (typed by whoever configured the connection,
   * or resolved from the remote machine itself), unlike this card's own translated chrome -- wrapped
   * in `<bdi>` so a strong-RTL or mixed-script segment cannot read the "@"/":" separators or the
   * port number out of order. Real, DOM-structural proof: a fixture host containing an actual RTL
   * character, asserting the isolation boundary wraps exactly that text.
   */
  describe("bidi isolation (20.16)", () => {
    it("wraps the user@host summary in a bdi boundary when the host carries an RTL character", () => {
      const rtlHost = "بالخادم.example.com";
      renderCard({ host: rtlHost });
      const summary = screen.getByTitle(`vane@${rtlHost}:22`);
      const isolated = summary.querySelector("bdi");
      expect(isolated).not.toBeNull();
      expect(isolated?.textContent).toBe(`vane@${rtlHost}`);
    });

    it("wraps a default path containing an RTL character in its own bdi boundary", () => {
      const rtlPath = "/srv/אבג/app";
      renderCard({ defaultPath: rtlPath });
      const pathNode = screen.getByTitle(rtlPath);
      const isolated = pathNode.querySelector("bdi");
      expect(isolated).not.toBeNull();
      expect(isolated?.textContent).toBe(rtlPath);
    });
  });
});
