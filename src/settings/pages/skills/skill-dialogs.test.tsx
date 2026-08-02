// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import type { Skill } from "../../../types/skill";
import { SkillDialogs } from "./skill-dialogs";

const skill: Skill = {
  id: "conflicted-skill",
  scope: "global",
  workspacePath: null,
  source: "user",
  enabled: true,
  skillDir: "~/.vanehub/skills/conflicted-skill",
  skillMdPath: "~/.vanehub/skills/conflicted-skill/SKILL.md",
  contentHash: "stale-hash",
  metadata: {
    id: "conflicted-skill",
    name: "Conflicted Skill",
    description: "Fixture",
    category: "testing",
    version: "1.0.0",
    triggers: [],
  },
  boundAgentIds: [],
  bindings: [],
  createdAt: "now",
  updatedAt: "now",
};

describe("SkillDialogs", () => {
  it("keeps an edit conflict visible and offers to reload the latest document", async () => {
    const reload = vi.fn();
    const user = userEvent.setup();
    render(
      <SkillDialogs
        editConflict
        editError="validation error: Skill changed since it was loaded: conflicted-skill"
        onClose={vi.fn()}
        onCreate={vi.fn()}
        onImport={vi.fn()}
        onReloadEdit={reload}
        onRestore={vi.fn()}
        onUpdate={vi.fn()}
        reloadingEdit={false}
        restoreCandidates={[]}
        scope="global"
        state={{ mode: "edit", skill, preview: null, editBody: "Draft body" }}
        workspacePath={null}
      />,
    );

    expect(screen.getByRole("alert").textContent).toContain("Skill changed since it was loaded");
    await user.click(screen.getByRole("button", { name: "重新加载最新内容" }));
    expect(reload).toHaveBeenCalledWith(skill);
    expect(screen.getByDisplayValue("Draft body")).toBeTruthy();
  });
});
