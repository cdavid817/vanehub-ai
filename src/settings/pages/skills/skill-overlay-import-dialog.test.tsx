// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillOverlayDetail, SkillOverlayImportReview } from "../../../types/skill-overlay";
import { SkillOverlayImportDialog } from "./skill-overlay-import-dialog";

const target = { skillId: "developer", scope: "user" } as const;
const detail = overlayDetail();
const review: SkillOverlayImportReview = {
  sourceSummary: "team-overlay.zip",
  revision: 4,
  documentHash: "reviewed-document-hash",
  scan: { scannerVersion: "overlay-text-v1", passed: true, safeRuleIds: [], ruleIdsTruncated: false },
  diff: {
    baseHash: "base-hash", effectiveHash: "reviewed-effective-hash", addedCharacters: 8, removedCharacters: 4,
    hunks: [{ label: "instructions", before: bounded("Base"), after: bounded("Reviewed") }], hunksTruncated: false,
  },
  mutations: [{ id: "learn-1", kind: "learnedGuidance", scope: "user", state: "active", createdAt: "now", updatedAt: "now" }],
  mutationsTruncated: false,
  resources: [{ mutationId: "file-1", logicalPath: "references/team.md", mediaType: "text/markdown", sizeBytes: 12, contentHash: "resource-content-hash", effectiveScope: "user", state: "active", shadowed: [], shadowedTruncated: false }],
  resourcesTruncated: false,
  conflicts: [{ id: "conflict-1", mutationId: "learn-1", safeReason: "exact-target-missing", state: "active", resolutionRevision: null }],
  conflictsTruncated: false,
};

afterEach(() => vi.restoreAllMocks());

describe("SkillOverlayImportDialog", () => {
  it("keeps imports quarantined until the exact displayed witnesses are acknowledged", async () => {
    const user = userEvent.setup({ applyAccept: false });
    const importSpy = vi.spyOn(agentService, "importSkillOverlay").mockResolvedValue(review);
    const promoteSpy = vi.spyOn(agentService, "promoteSkillOverlay").mockResolvedValue({ summary: detail.summary, committedRevision: 5, diff: review.diff });
    const onCommitted = vi.fn();
    renderDialog(onCommitted);

    await user.upload(screen.getByLabelText(/Overlay ZIP 包/), new File([new Uint8Array([80, 75, 3, 4])], "team-overlay.zip", { type: "application/zip" }));
    expect(screen.getByText("team-overlay.zip")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "导入隔离区" }));

    await waitFor(() => expect(importSpy).toHaveBeenCalledWith({
      target,
      witnesses: expect.objectContaining({ expectedOverlayRevision: 3, expectedBaseInstructionHash: "base-hash" }),
      sourceName: "team-overlay.zip",
      archive: [80, 75, 3, 4],
    }));
    expect(onCommitted).toHaveBeenCalledOnce();
    expect(await screen.findByText("不可信 · 未生效")).toBeTruthy();
    expect(screen.getAllByText("reviewed-document-hash").length).toBeGreaterThan(0);
    expect(screen.getByText("overlay-text-v1")).toBeTruthy();
    expect(screen.getByText("references/team.md")).toBeTruthy();
    expect(screen.getByText("exact-target-missing")).toBeTruthy();
    expect(screen.getByText("Reviewed")).toBeTruthy();

    const promote = screen.getByRole("button", { name: "提升已审查修订" });
    expect((promote as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("checkbox", { name: /我已核对/ }));
    expect((promote as HTMLButtonElement).disabled).toBe(false);
    await user.click(promote);

    await waitFor(() => expect(promoteSpy).toHaveBeenCalledWith({
      target,
      witnesses: {
        expectedOverlayRevision: 4,
        expectedBaseInstructionHash: "base-hash",
        expectedBasePackageHash: "package-hash",
        expectedPayloadHash: "reviewed-document-hash",
        expectedPinned: false,
      },
      reviewedRevision: 4,
      reviewedDocumentHash: "reviewed-document-hash",
      reviewedScan: review.scan,
    }));
  });

  it("never enables trust promotion for a failed deterministic scan", async () => {
    const user = userEvent.setup();
    vi.spyOn(agentService, "importSkillOverlay").mockResolvedValue({
      ...review,
      scan: { ...review.scan, passed: false, safeRuleIds: ["prompt-authority-override"] },
    });
    const promoteSpy = vi.spyOn(agentService, "promoteSkillOverlay");
    renderDialog();
    await user.upload(screen.getByLabelText(/Overlay ZIP 包/), new File(["safe"], "team-overlay.zip", { type: "application/zip" }));
    await user.click(screen.getByRole("button", { name: "导入隔离区" }));
    await user.click(await screen.findByRole("checkbox", { name: /我已核对/ }));

    expect(screen.getByText("prompt-authority-override")).toBeTruthy();
    expect((screen.getByRole("button", { name: "提升已审查修订" }) as HTMLButtonElement).disabled).toBe(true);
    expect(promoteSpy).not.toHaveBeenCalled();
  });

  it("rejects oversized archives before reading or importing them", async () => {
    const user = userEvent.setup({ applyAccept: false });
    const importSpy = vi.spyOn(agentService, "importSkillOverlay");
    renderDialog();
    const archive = new File(["small"], "large.zip", { type: "application/zip" });
    Object.defineProperty(archive, "size", { value: 8 * 1_048_576 + 1 });
    await user.upload(screen.getByLabelText(/Overlay ZIP 包/), archive);

    expect((await screen.findByRole("alert")).textContent).toContain("8.00 MiB");
    expect((screen.getByRole("button", { name: "导入隔离区" }) as HTMLButtonElement).disabled).toBe(true);
    expect(importSpy).not.toHaveBeenCalled();
  });
});

function renderDialog(onCommitted = vi.fn()) {
  return render(<SkillOverlayImportDialog detail={detail} onClose={vi.fn()} onCommitted={onCommitted} onRefresh={vi.fn()} returnFocus={null} target={target} />);
}

function overlayDetail(): SkillOverlayDetail {
  return {
    summary: {
      canonicalSkillId: "developer", baseLayer: "system", status: "healthy", needsReconcile: false,
      pinned: false, baseInstructionHash: "base-hash", basePackageHash: "package-hash", effectiveHash: "effective-hash",
      lastHealthyScope: "user", scopes: [{ scope: "user", revision: 3, trust: "trusted", status: "applied", activeMutationCount: 1, conflictCount: 0, baseHashChanged: false, needsReconcile: false }], scopesTruncated: false,
    },
    baseInstructions: bounded("Base"), effectiveInstructions: bounded("Base"), diff: emptyDiff(), scopeDiffs: [], scopeDiffsTruncated: false,
    mutations: [], mutationsTruncated: false, resources: [], resourcesTruncated: false, conflicts: [], conflictsTruncated: false,
  };
}

function bounded(content: string) {
  return { content, totalCharacters: content.length, truncated: false };
}

function emptyDiff() {
  return { baseHash: "base-hash", effectiveHash: "effective-hash", addedCharacters: 0, removedCharacters: 0, hunks: [], hunksTruncated: false };
}
