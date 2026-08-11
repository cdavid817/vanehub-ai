import { beforeEach, describe, expect, it } from "vitest";
import type { SkillOverlayTargetInput, SkillOverlayWitnesses } from "../types/skill-overlay";
import { createWebSkillOverlayRuntime } from "./web-skill-overlay-runtime";
import { webOverlayHash, type WebOverlayBase } from "./web-skill-overlay-support";

const target: SkillOverlayTargetInput = { skillId: "developer", scope: "user", workspacePath: null };
let base: WebOverlayBase;
let runtime: ReturnType<typeof createWebSkillOverlayRuntime>;

function witnesses(
  expectedOverlayRevision: number | null,
  expectedPayloadHash: string | null = null,
  expectedPinned = false,
): SkillOverlayWitnesses {
  return {
    expectedOverlayRevision,
    expectedBaseInstructionHash: base.instructionHash,
    expectedBasePackageHash: base.packageHash,
    expectedPayloadHash,
    expectedPinned,
  };
}

beforeEach(() => {
  base = {
    skillId: "developer",
    layer: "system",
    instructions: "Alpha old guidance Omega",
    instructionHash: webOverlayHash("Alpha old guidance Omega"),
    packageHash: "package-v1",
    pinned: false,
  };
  runtime = createWebSkillOverlayRuntime(() => base);
});

describe("Web Skill Overlay runtime", () => {
  it("previews without persistence, commits revision one, and rejects stale witnesses", () => {
    const input = {
      target,
      witnesses: witnesses(null),
      oldString: "old guidance",
      newString: "reviewed guidance",
      replaceAll: false,
    };

    expect(runtime.preview({
      target,
      witnesses: input.witnesses,
      mutation: { kind: "exactPatch", ...input },
    })).toMatchObject({ tentativeRevision: 1, canCommit: true });
    expect(runtime.getSummary(target).status).toBe("none");

    expect(runtime.createPatch(input)).toMatchObject({ committedRevision: 1, summary: { status: "healthy" } });
    expect(runtime.getDetail(target).effectiveInstructions.content).toContain("reviewed guidance");
    expect(() => runtime.createPatch(input)).toThrow(expect.objectContaining({
      kind: "stale",
      code: "stale-witnesses",
      expectedRevision: null,
      currentRevision: 1,
    }));
    expect(runtime.getSummary(target).scopes[0]?.revision).toBe(1);
  });

  it("quarantines imports until exact review witnesses are promoted", () => {
    const review = runtime.importOverlay({
      target,
      witnesses: witnesses(null),
      sourceName: "team-overlay.zip",
      archive: [80, 75, 3, 4],
    });

    expect(review).toMatchObject({ revision: 1, sourceSummary: "team-overlay.zip", scan: { passed: true } });
    expect(runtime.getSummary(target).status).toBe("untrusted");
    expect(runtime.getDetail(target).effectiveInstructions.content).toBe(base.instructions);

    const promoted = runtime.promote({
      target,
      witnesses: witnesses(1, review.documentHash),
      reviewedRevision: review.revision,
      reviewedDocumentHash: review.documentHash,
      reviewedScan: review.scan,
    });
    expect(promoted).toMatchObject({ committedRevision: 2, summary: { status: "healthy" } });
    expect(runtime.getDetail(target).effectiveInstructions.content).toContain("Reviewed import: team-overlay.zip");
  });

  it("freezes existing effective content while pinned and refuses every mutation", () => {
    runtime.createGuidance({ target, witnesses: witnesses(null), guidance: "Keep changes narrow." });
    runtime.setPinned(target, true);

    expect(runtime.getSummary(target).pinned).toBe(true);
    expect(runtime.getDetail(target).effectiveInstructions.content).toContain("Keep changes narrow.");
    expect(() => runtime.revert({
      target,
      witnesses: witnesses(1, null, true),
      mutationId: "mutation-1",
      mutationKind: "learnedGuidance",
    })).toThrow(expect.objectContaining({ kind: "pinned", code: "skill-pinned" }));
    expect(runtime.getSummary(target).scopes[0]?.revision).toBe(1);
  });

  it("records exact-match conflicts and reconciles them with a complete preview", () => {
    const conflicted = runtime.createPatch({
      target,
      witnesses: witnesses(null),
      oldString: "missing text",
      newString: "replacement",
      replaceAll: false,
    });
    expect(conflicted.summary).toMatchObject({ status: "needsReconciliation", needsReconcile: true });
    const conflict = runtime.getDetail(target).conflicts[0];
    expect(conflict).toMatchObject({ state: "active", safeReason: "exact-match-contract-failed" });

    const input = {
      target,
      witnesses: witnesses(1),
      choices: [{
        conflictId: conflict.id,
        resolution: "editPatch" as const,
        oldString: "old guidance",
        newString: "reconciled guidance",
        replaceAll: false,
      }],
    };
    expect(runtime.previewReconciliation(input)).toMatchObject({ canCommit: true, finalDiffComplete: true });
    expect(runtime.reconcile(input)).toMatchObject({ committedRevision: 2, summary: { status: "healthy" } });
    expect(runtime.getDetail(target).effectiveInstructions.content).toContain("reconciled guidance");
  });

  it("retains append-only history when reverting and reports verification failure", () => {
    runtime.createGuidance({ target, witnesses: witnesses(null), guidance: "Temporary guidance." });
    runtime.revert({
      target,
      witnesses: witnesses(1),
      mutationId: "mutation-1",
      mutationKind: "learnedGuidance",
    });

    expect(runtime.getDetail(target).effectiveInstructions.content).toBe(base.instructions);
    const firstPage = runtime.getHistory({ target, limit: 1 });
    expect(firstPage).toMatchObject({ integrity: "verified", nextCursor: "1" });
    expect(firstPage.entries[0]?.action).toBe("learn");
    expect(runtime.getHistory({ target, cursor: "1", limit: 10 }).entries[0]?.action).toBe("revert");

    runtime.corruptHistory(target);
    expect(runtime.getHistory({ target, limit: 10 }).integrity).toBe("failed:web-history-link");
  });

  it("manages resource revisions and detects base drift through CAS witnesses", () => {
    runtime.addFile({
      target,
      witnesses: witnesses(null),
      logicalPath: "references/team.md",
      mediaType: "text/markdown",
      content: [35, 32, 84, 101, 97, 109],
    });
    expect(runtime.getDetail(target).resources).toHaveLength(1);

    const stale = witnesses(1);
    base = { ...base, packageHash: "package-v2" };
    expect(() => runtime.replaceFile({
      target,
      witnesses: stale,
      logicalPath: "references/team.md",
      mediaType: "text/markdown",
      content: [117, 112, 100, 97, 116, 101, 100],
    })).toThrow(expect.objectContaining({ kind: "stale", baseChanged: true }));
    expect(runtime.getSummary(target)).toMatchObject({ needsReconcile: true, status: "needsReconciliation" });
    expect(runtime.getDetail(target).effectiveInstructions.content).toBe(base.instructions);
  });
});
