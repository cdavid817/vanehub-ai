import { describe, expect, it } from "vitest";
import type { PersonalizationPolicy, PersonalizationPolicyRef } from "../../../types/personalization";
import {
  beginSave,
  canSave,
  discardDraft,
  draftFromPolicy,
  editDraft,
  isDirty,
  keepMine,
  mergePolicies,
  saveConflicted,
  saveFailed,
  saveSucceeded,
  scopeKeyOf,
  takeTheirs,
  type InstructionDraftMap,
} from "./instruction-drafts";

const GLOBAL: PersonalizationPolicyRef = { scopeKind: "global" };
const AGENT: PersonalizationPolicyRef = { scopeKind: "agent", agentId: "onepiece" };

function policy(overrides: Partial<PersonalizationPolicy> = {}): PersonalizationPolicy {
  return {
    scopeKind: "global",
    scopeKey: "",
    revision: 3,
    instructionMergeMode: "append",
    aboutUser: "Backend engineer.",
    styleRules: "Lead with the conclusion.",
    memoryReadMode: "enabled",
    explicitSaveMode: "enabled",
    automaticExtractionMode: "enabled",
    globalMemoryAccessMode: "enabled",
    ...overrides,
  };
}

function seeded(): InstructionDraftMap {
  return { [scopeKeyOf(GLOBAL)]: draftFromPolicy(GLOBAL, policy()) };
}

describe("instruction drafts", () => {
  it("tells apart scopes whose keys contain the same characters", () => {
    // A delimiter-joined key would collide here, and two scopes sharing a draft is how one
    // layer's half-typed text ends up in another's field.
    const left = scopeKeyOf({ scopeKind: "workspace-agent", agentId: "a", workspaceKey: "b|c" });
    const right = scopeKeyOf({ scopeKind: "workspace-agent", agentId: "a|b", workspaceKey: "c" });

    expect(left).not.toBe(right);
  });

  it("treats a never-written layer as revision zero rather than as missing", () => {
    const draft = draftFromPolicy(AGENT, null);

    expect(draft.baseRevision).toBe(0);
    expect(draft.values.instructionMergeMode).toBe("inherit");
    expect(isDirty(draft)).toBe(false);
  });

  it("refuses to save a draft that changed nothing", () => {
    const drafts = seeded();

    expect(canSave(drafts[scopeKeyOf(GLOBAL)])).toBe(false);
  });

  it("keeps drafts apart by scope", () => {
    let drafts = seeded();
    drafts = { ...drafts, [scopeKeyOf(AGENT)]: draftFromPolicy(AGENT, null) };
    drafts = editDraft(drafts, GLOBAL, { aboutUser: "typed into global" });

    expect(drafts[scopeKeyOf(AGENT)].values.aboutUser).toBe("");
    expect(drafts[scopeKeyOf(GLOBAL)].values.aboutUser).toBe("typed into global");
  });

  it("lets a clean draft follow a refetch", () => {
    const drafts = mergePolicies(seeded(), [
      { scope: GLOBAL, policy: policy({ revision: 4, aboutUser: "edited elsewhere" }) },
    ]);

    expect(drafts[scopeKeyOf(GLOBAL)].values.aboutUser).toBe("edited elsewhere");
    expect(drafts[scopeKeyOf(GLOBAL)].conflict).toBeNull();
  });

  it("never overwrites a dirty draft with a refetch", () => {
    const edited = editDraft(seeded(), GLOBAL, { aboutUser: "half typed" });

    const drafts = mergePolicies(edited, [
      { scope: GLOBAL, policy: policy({ revision: 9, aboutUser: "someone else's text" }) },
    ]);

    expect(drafts[scopeKeyOf(GLOBAL)].values.aboutUser).toBe("half typed");
    expect(drafts[scopeKeyOf(GLOBAL)].conflict).toEqual({
      stored: {
        aboutUser: "someone else's text",
        styleRules: "Lead with the conclusion.",
        instructionMergeMode: "append",
      },
      storedRevision: 9,
      attemptedRevision: 3,
    });
  });

  it("leaves a dirty draft alone when the refetch changed nothing", () => {
    const edited = editDraft(seeded(), GLOBAL, { aboutUser: "half typed" });

    const drafts = mergePolicies(edited, [{ scope: GLOBAL, policy: policy() }]);

    expect(drafts[scopeKeyOf(GLOBAL)]).toEqual(edited[scopeKeyOf(GLOBAL)]);
  });

  it("keeps the draft when a save fails for any other reason", () => {
    const edited = editDraft(seeded(), GLOBAL, { styleRules: "Answer in Chinese." });

    const drafts = saveFailed(beginSave(edited, GLOBAL), GLOBAL, "personalization-storage-unavailable");
    const draft = drafts[scopeKeyOf(GLOBAL)];

    // The failed attempt is over; the text the user typed is the only copy of it there is.
    expect(draft.values.styleRules).toBe("Answer in Chinese.");
    expect(draft.saving).toBe(false);
    expect(draft.error).toBe("personalization-storage-unavailable");
  });

  it("clears a stale error when the user resumes typing but keeps the conflict", () => {
    let drafts = saveFailed(seeded(), GLOBAL, "personalization-storage-unavailable");
    drafts = saveConflicted(drafts, GLOBAL, policy({ revision: 7 }));

    drafts = editDraft(drafts, GLOBAL, { aboutUser: "still typing" });
    const draft = drafts[scopeKeyOf(GLOBAL)];

    expect(draft.error).toBeNull();
    // Typing does not change what the store holds, so it cannot resolve a conflict either.
    expect(draft.conflict?.storedRevision).toBe(7);
  });

  it("refuses to save while a conflict is unresolved", () => {
    let drafts = editDraft(seeded(), GLOBAL, { aboutUser: "mine" });
    drafts = saveConflicted(drafts, GLOBAL, policy({ revision: 7, aboutUser: "theirs" }));

    expect(canSave(drafts[scopeKeyOf(GLOBAL)])).toBe(false);
  });

  it("lets the user keep their text, retargeted at the revision that refused it", () => {
    let drafts = editDraft(seeded(), GLOBAL, { aboutUser: "mine" });
    drafts = saveConflicted(drafts, GLOBAL, policy({ revision: 7, aboutUser: "theirs" }));

    drafts = keepMine(drafts, GLOBAL);
    const draft = drafts[scopeKeyOf(GLOBAL)];

    expect(draft.values.aboutUser).toBe("mine");
    // Retargeted, so the retry can land -- and it lands because the user chose it, not because a
    // race decided which response arrived last.
    expect(draft.baseRevision).toBe(7);
    expect(draft.conflict).toBeNull();
    expect(canSave(draft)).toBe(true);
  });

  it("lets the user take the stored text and end up clean", () => {
    let drafts = editDraft(seeded(), GLOBAL, { aboutUser: "mine" });
    drafts = saveConflicted(drafts, GLOBAL, policy({ revision: 7, aboutUser: "theirs" }));

    drafts = takeTheirs(drafts, GLOBAL);
    const draft = drafts[scopeKeyOf(GLOBAL)];

    expect(draft.values.aboutUser).toBe("theirs");
    expect(draft.baseRevision).toBe(7);
    expect(isDirty(draft)).toBe(false);
    expect(draft.conflict).toBeNull();
  });

  it("rebases on the saved policy so a second save is not a stale write", () => {
    let drafts = editDraft(seeded(), GLOBAL, { aboutUser: "mine" });
    drafts = beginSave(drafts, GLOBAL);

    drafts = saveSucceeded(drafts, GLOBAL, policy({ revision: 4, aboutUser: "mine" }));
    const draft = drafts[scopeKeyOf(GLOBAL)];

    expect(draft.baseRevision).toBe(4);
    expect(isDirty(draft)).toBe(false);
    expect(draft.saving).toBe(false);
  });

  it("discards back to the stored text without touching the store", () => {
    let drafts = editDraft(seeded(), GLOBAL, { aboutUser: "mine", styleRules: "also mine" });

    drafts = discardDraft(drafts, GLOBAL);
    const draft = drafts[scopeKeyOf(GLOBAL)];

    expect(draft.values).toEqual(draft.baseline);
    expect(draft.baseRevision).toBe(3);
  });

  it("saves one scope without disturbing another that is also in flight", () => {
    let drafts = { ...seeded(), [scopeKeyOf(AGENT)]: draftFromPolicy(AGENT, null) };
    drafts = editDraft(drafts, GLOBAL, { aboutUser: "global text" });
    drafts = editDraft(drafts, AGENT, { aboutUser: "agent text" });
    drafts = beginSave(beginSave(drafts, GLOBAL), AGENT);

    drafts = saveFailed(drafts, GLOBAL, "personalization-storage-unavailable");

    expect(drafts[scopeKeyOf(AGENT)].saving).toBe(true);
    expect(drafts[scopeKeyOf(AGENT)].error).toBeNull();
    expect(drafts[scopeKeyOf(GLOBAL)].saving).toBe(false);
  });

  it("ignores an edit aimed at a scope it has never loaded", () => {
    const drafts = editDraft(seeded(), AGENT, { aboutUser: "nowhere" });

    expect(drafts[scopeKeyOf(AGENT)]).toBeUndefined();
    expect(Object.keys(drafts)).toEqual([scopeKeyOf(GLOBAL)]);
  });
});
