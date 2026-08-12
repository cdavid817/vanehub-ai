// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillOverlayDetail, SkillOverlayHistoryEntry, SkillOverlayHistoryPage } from "../../../types/skill-overlay";
import { SkillOverlayHistory } from "./skill-overlay-history";

const target = { skillId: "developer", scope: "user" } as const;

afterEach(() => vi.restoreAllMocks());

describe("SkillOverlayHistory", () => {
  it("shows verified event evidence and loads bounded pages", async () => {
    const user = userEvent.setup();
    const historySpy = vi.spyOn(agentService, "getSkillOverlayHistory")
      .mockResolvedValueOnce(page([event("event-2", "import", 1, 2)], "older"))
      .mockResolvedValueOnce(page([event("event-1", "conflict", null, 1)], null));
    renderHistory();

    expect(await screen.findAllByText("导入已隔离")).toHaveLength(2);
    expect(screen.getByText("完整性已验证")).toBeTruthy();
    expect(screen.getByText("修订 1 → 2")).toBeTruthy();
    expect(screen.getByText("系统")).toBeTruthy();
    expect(screen.getByText("用户 Overlay")).toBeTruthy();
    expect(screen.getByText("safe-diff:event-2")).toBeTruthy();
    await user.click(screen.getByText("完整性证据"));
    expect(screen.getByText("document:event-2")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "加载更早事件" }));
    expect(await screen.findAllByText("已记录冲突")).toHaveLength(2);
    expect(historySpy).toHaveBeenNthCalledWith(2, { target: { ...target, workspacePath: null }, cursor: "older", limit: 20 });
  });

  it("keeps unverifiable events visible and disables revert", async () => {
    vi.spyOn(agentService, "getSkillOverlayHistory").mockResolvedValue(page(
      [event("event-bad", "patch", null, 1)],
      null,
      "failed:segment-link-mismatch",
    ));
    renderHistory();

    expect(await screen.findByText("已提交精确补丁")).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toContain("segment-link-mismatch");
    expect((screen.getByRole("button", { name: "回退" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("reverts a current mutation by appending and refreshing a new revision", async () => {
    const user = userEvent.setup();
    vi.spyOn(agentService, "getSkillOverlayHistory").mockResolvedValue(page([event("event-3", "patch", 2, 3)], null));
    const revertSpy = vi.spyOn(agentService, "revertSkillOverlayMutation").mockResolvedValue({
      summary: detail.summary,
      committedRevision: 4,
      diff: detail.diff,
    });
    const onCommitted = vi.fn();
    renderHistory(onCommitted);

    await screen.findByText("已提交精确补丁");
    await user.click(screen.getByRole("button", { name: "回退" }));
    expect(screen.getByRole("dialog").textContent).toContain("保留所有已有事件");
    await user.click(screen.getByRole("button", { name: "创建回退修订" }));

    await waitFor(() => expect(revertSpy).toHaveBeenCalledWith({
      target: { ...target, workspacePath: null },
      witnesses: {
        expectedOverlayRevision: 3,
        expectedBaseInstructionHash: "base-instruction",
        expectedBasePackageHash: "base-package",
        expectedPayloadHash: null,
        expectedPinned: false,
      },
      mutationId: "patch-1",
      mutationKind: "patch",
    }));
    expect(onCommitted).toHaveBeenCalledOnce();
    expect(await screen.findByText(/已创建回退修订 4/)).toBeTruthy();
  });
});

function renderHistory(onCommitted = vi.fn()) {
  return render(<SkillOverlayHistory detail={detail} onCommitted={onCommitted} onRefresh={vi.fn()} target={target} />);
}

function page(entries: SkillOverlayHistoryEntry[], nextCursor: string | null, integrity: SkillOverlayHistoryPage["integrity"] = "verified"): SkillOverlayHistoryPage {
  return { entries, nextCursor, integrity };
}

function event(eventId: string, action: SkillOverlayHistoryEntry["action"], priorRevision: number | null, nextRevision: number): SkillOverlayHistoryEntry {
  return {
    eventId,
    canonicalSkillId: "developer",
    scope: "user",
    priorRevision,
    nextRevision,
    actor: "system",
    action,
    timestamp: "2026-08-11T08:00:00.000Z",
    priorDocumentHash: priorRevision == null ? null : `prior-document:${eventId}`,
    nextDocumentHash: `document:${eventId}`,
    scannerVersion: "scanner-v2",
    safeOutcome: `safe-diff:${eventId}`,
    priorEventHash: priorRevision == null ? null : `prior-event:${eventId}`,
    eventHash: `hash:${eventId}`,
  };
}

const detail: SkillOverlayDetail = {
  summary: {
    canonicalSkillId: "developer",
    baseLayer: "system",
    status: "healthy",
    needsReconcile: false,
    pinned: false,
    baseInstructionHash: "base-instruction",
    basePackageHash: "base-package",
    effectiveHash: "effective",
    lastHealthyScope: "user",
    scopes: [{
      scope: "user", revision: 3, trust: "trusted", status: "applied", activeMutationCount: 1,
      conflictCount: 0, baseHashChanged: false, needsReconcile: false,
    }],
    scopesTruncated: false,
  },
  baseInstructions: { content: "base", totalCharacters: 4, truncated: false },
  effectiveInstructions: { content: "effective", totalCharacters: 9, truncated: false },
  diff: { baseHash: "base", effectiveHash: "effective", addedCharacters: 5, removedCharacters: 0, hunks: [], hunksTruncated: false },
  scopeDiffs: [],
  scopeDiffsTruncated: false,
  mutations: [{ id: "patch-1", kind: "patch", scope: "user", state: "active", createdAt: "now", updatedAt: "now" }],
  mutationsTruncated: false,
  resources: [],
  resourcesTruncated: false,
  conflicts: [],
  conflictsTruncated: false,
};
