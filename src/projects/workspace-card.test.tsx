// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import { localGitWorkspace } from "../testing/fixtures/workspace-fixtures";
import { WorkspaceCard } from "./workspace-card";

function renderCard(overrides: Parameters<typeof localGitWorkspace>[0] = {}, selected = false) {
  return render(<WorkspaceCard onSelect={vi.fn()} selected={selected} workspace={localGitWorkspace(overrides)} />);
}

describe("WorkspaceCard", () => {
  it("renders the workspace's display name, path, and availability badge", () => {
    renderCard();
    expect(screen.getByText("vanehub-ai")).toBeTruthy();
    expect(screen.getByText("可用")).toBeTruthy();
  });

  /**
   * 20.15: this card's name (`min-w-0 truncate`) and path (`truncate`) already carry the
   * established mechanism -- these prove it holds for the long-string classes the task names, next
   * to the availability Badge and the leading kind icon, using a real long path (not a synthetic
   * one), matching the task's own "real long path" ask. jsdom has no real layout engine, so this
   * checks class presence, not pixels -- the same documented limitation as the other 20.15 tests
   * added this pass.
   */
  describe("long content safety (20.15)", () => {
    const GERMAN_LIKE_NAME = "Konfigurationsverwaltungsoberflächenkomponente";
    const CJK_NAME = "这是一个非常非常非常长的工作区名称用来验证界面不会与状态徽章或图标重叠";
    const REAL_LONG_PATH = "D:/workspace/monorepo-repository/packages/frontend-web-application/src/features/session-workspace/components/inspector";

    it("truncates a long German-like display name next to the availability badge", () => {
      renderCard({ displayName: GERMAN_LIKE_NAME });
      const name = screen.getByText(GERMAN_LIKE_NAME);
      expect(name.className).toContain("truncate");
      expect(screen.getByText("可用")).toBeTruthy();
    });

    it("truncates a long CJK display name the same way", () => {
      renderCard({ displayName: CJK_NAME });
      expect(screen.getByText(CJK_NAME).className).toContain("truncate");
    });

    it("truncates a real long workspace path underneath the name", () => {
      renderCard({ displayPath: REAL_LONG_PATH });
      const path = screen.getByTitle(REAL_LONG_PATH);
      expect(path.className).toContain("truncate");
    });
  });

  /**
   * 20.16: `displayPath` is filesystem-sourced (local disk, or a resolved SSH `user@host:path`),
   * rendered verbatim -- wrapped in `<bdi>` so a real path segment containing a strong-RTL or
   * mixed-script character cannot read this row's own fixed-direction chrome out of order.
   */
  it("wraps a display path containing an RTL character in a bdi isolation boundary", () => {
    const rtlPath = "D:/workspace/פרויקט-לדוגמה/app";
    renderCard({ displayPath: rtlPath });
    const path = screen.getByTitle(rtlPath);
    const isolated = path.querySelector("bdi");
    expect(isolated).not.toBeNull();
    expect(isolated?.textContent).toBe(rtlPath);
  });
});
