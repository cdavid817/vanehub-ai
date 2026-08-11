// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillOverlayDetail } from "../../../types/skill-overlay";
import { SkillOverlayOverview } from "./skill-overlay-overview";

const detail: SkillOverlayDetail = {
  summary: {
    canonicalSkillId: "developer",
    baseLayer: "system",
    status: "needsReconciliation",
    needsReconcile: true,
    pinned: true,
    baseInstructionHash: "base-instruction-hash-1234567890",
    basePackageHash: "base-package-hash-1234567890",
    effectiveHash: "effective-hash-1234567890",
    lastHealthyScope: "system",
    scopes: [{
      scope: "user",
      revision: 4,
      trust: "trusted",
      status: "needsReconciliation",
      activeMutationCount: 3,
      conflictCount: 1,
      baseHashChanged: true,
      needsReconcile: true,
    }],
    scopesTruncated: false,
  },
  baseInstructions: { content: "base", totalCharacters: 4, truncated: false },
  effectiveInstructions: { content: "effective", totalCharacters: 9, truncated: false },
  diff: { baseHash: "base", effectiveHash: "effective", addedCharacters: 5, removedCharacters: 0, hunks: [], hunksTruncated: false },
  scopeDiffs: [{
    scope: "user",
    revision: 4,
    inputHash: "base",
    outputHash: "effective",
    diff: {
      baseHash: "base",
      effectiveHash: "effective",
      addedCharacters: 5,
      removedCharacters: 0,
      hunks: [{
        label: "overlay-scope:user",
        before: { content: "base", totalCharacters: 4, truncated: false },
        after: { content: "effective", totalCharacters: 9, truncated: false },
      }],
      hunksTruncated: false,
    },
  }],
  scopeDiffsTruncated: false,
  mutations: [
    { id: "patch-1", kind: "patch", scope: "user", state: "active", createdAt: "now", updatedAt: "now" },
    { id: "guidance-1", kind: "learnedGuidance", scope: "user", state: "active", createdAt: "now", updatedAt: "now" },
    { id: "file-1", kind: "supportingFile", scope: "user", state: "active", createdAt: "now", updatedAt: "now" },
    { id: "patch-2", kind: "patch", scope: "user", state: "disabled", createdAt: "now", updatedAt: "now" },
  ],
  mutationsTruncated: false,
  resources: [{
    mutationId: "file-1",
    logicalPath: "references/team-guidance.md",
    mediaType: "text/markdown",
    sizeBytes: 42,
    contentHash: "resource-hash",
    effectiveScope: "user",
    state: "active",
    shadowed: [{ scope: null, baseLayer: "system", contentHash: "old-resource-hash" }],
    shadowedTruncated: false,
  }],
  resourcesTruncated: false,
  conflicts: [{ id: "conflict-1", mutationId: "patch-1", safeReason: "missing-target", state: "active", resolutionRevision: null }],
  conflictsTruncated: false,
};

afterEach(() => vi.restoreAllMocks());

describe("SkillOverlayOverview", () => {
  it("summarizes governance state without relying on color alone", () => {
    vi.spyOn(agentService, "getSkillOverlayHistory").mockResolvedValue({ entries: [], nextCursor: null, integrity: "verified" });
    render(<SkillOverlayOverview detail={detail} loading={false} onCommitted={vi.fn()} onRefresh={vi.fn()} target={{ skillId: "developer", scope: "user" }} />);

    const section = screen.getByRole("heading", { name: "Overlay 治理" }).closest("section");
    if (!section) throw new Error("Overlay section not found");
    expect(within(section).getAllByText("需要协调")).toHaveLength(2);
    expect(within(section).getByText("可信 Overlay")).toBeTruthy();
    expect(within(section).getByText("已固定")).toBeTruthy();
    expect(within(section).getAllByText("用户 Overlay")).toHaveLength(3);
    expect(within(section).getByText(/修订 4/)).toBeTruthy();
    expect(within(section).getByText("references/team-guidance.md")).toBeTruthy();
    expect(within(section).getByText("已遮蔽 1 个来源")).toBeTruthy();
    expect(within(section).getByRole("note").textContent).toContain("已提交的 Overlay 内容会继续生效并保持可见");
    expect(within(section).getByRole("heading", { name: "已验证历史" })).toBeTruthy();
    expect(within(section).getByText("有效内容哈希")).toBeTruthy();
    for (const name of ["添加精确补丁", "添加学习指导", "添加支撑资源", "导入 Overlay 包", "打开协调工作区", "替换", "禁用"]) {
      expect((within(section).getByRole("button", { name }) as HTMLButtonElement).disabled).toBe(true);
    }
    for (const button of within(section).getAllByRole("button", { name: "回退" })) {
      expect((button as HTMLButtonElement).disabled).toBe(true);
    }
  });

  it("explains the unchanged base state and exposes a retry on load failure", async () => {
    const onRetry = vi.fn();
    const props = { onCommitted: vi.fn(), onRefresh: vi.fn(), target: { skillId: "developer", scope: "user" } as const };
    const { rerender } = render(<SkillOverlayOverview detail={{ ...detail, summary: { ...detail.summary, status: "none", scopes: [], pinned: false } }} loading={false} {...props} />);
    expect(screen.getByText(/有效内容与解析后的基础包保持一致/)).toBeTruthy();
    expect(screen.getByText("基础包哈希")).toBeTruthy();
    expect((screen.getByRole("button", { name: "添加精确补丁" }) as HTMLButtonElement).disabled).toBe(false);

    rerender(<SkillOverlayOverview error="offline" loading={false} onRetry={onRetry} {...props} />);
    screen.getByRole("button", { name: "重试" }).click();
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("shows every Overlay scope and identifies a project scope blocked by an earlier conflict", () => {
    vi.spyOn(agentService, "getSkillOverlayHistory").mockResolvedValue({ entries: [], nextCursor: null, integrity: "verified" });
    const scopes: SkillOverlayDetail["summary"]["scopes"] = [
      { scope: "system", revision: 1, trust: "trusted", status: "applied", activeMutationCount: 1, conflictCount: 0, baseHashChanged: false, needsReconcile: false },
      { scope: "user", revision: 4, trust: "trusted", status: "needsReconciliation", activeMutationCount: 2, conflictCount: 1, baseHashChanged: true, needsReconcile: true },
      { scope: "project", revision: 2, trust: "trusted", status: "blockedByEarlierScope", activeMutationCount: 1, conflictCount: 0, baseHashChanged: false, needsReconcile: false },
    ];
    render(<SkillOverlayOverview
      detail={{ ...detail, summary: { ...detail.summary, pinned: false, scopes } }}
      loading={false}
      onCommitted={vi.fn()}
      onRefresh={vi.fn()}
      target={{ skillId: "developer", scope: "project", workspacePath: "D:/workspace" }}
    />);

    expect(screen.getAllByText("系统 Overlay").length).toBeGreaterThan(0);
    expect(screen.getAllByText("用户 Overlay").length).toBeGreaterThan(0);
    expect(screen.getAllByText("项目 Overlay").length).toBeGreaterThan(0);
    expect(screen.getByText("被前序作用域阻止")).toBeTruthy();
    expect(screen.getByText(/基础见证已变化/)).toBeTruthy();
  });
});
