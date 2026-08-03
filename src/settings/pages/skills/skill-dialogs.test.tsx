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
        operationPending={false}
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

  it("offers rendered and source views for the loaded SKILL.md", async () => {
    const user = userEvent.setup();
    render(
      <SkillDialogs
        editConflict={false}
        editError={null}
        onClose={vi.fn()}
        onCreate={vi.fn()}
        onImport={vi.fn()}
        onReloadEdit={vi.fn()}
        onUpdate={vi.fn()}
        operationPending={false}
        reloadingEdit={false}
        scope="global"
        state={{ mode: null, skill: null, preview: { id: skill.id, scope: "global", workspacePath: null, path: skill.skillMdPath, content: "---\nid: conflicted-skill\n---\n# Conflicted Skill\n\nBody" } }}
        workspacePath={null}
      />,
    );

    expect(screen.getByRole("tab", { name: "渲染结果" })).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "源内容" }));
    expect(screen.getByText(/id: conflicted-skill/)).toBeTruthy();
  });

  it("locks submission and dismissal while a dialog operation is pending", () => {
    render(
      <SkillDialogs
        editConflict={false}
        editError={null}
        onClose={vi.fn()}
        onCreate={vi.fn()}
        onImport={vi.fn()}
        onReloadEdit={vi.fn()}
        onUpdate={vi.fn()}
        operationPending
        reloadingEdit={false}
        scope="global"
        state={{ mode: "create", skill: null, preview: null }}
        workspacePath={null}
      />,
    );

    expect(screen.getByRole("status").textContent).toContain("处理中");
    expect((screen.getByRole("button", { name: "取消" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "保存" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
