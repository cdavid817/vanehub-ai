// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { SkillDriftBanner } from "./skill-drift-banner";

const drift = { scope: "global" as const, workspacePath: null, issues: [], driftHash: "clean" };

describe("SkillDriftBanner", () => {
  it("renders healthy drift as a compact indicator", () => {
    render(<SkillDriftBanner drift={drift} onSync={vi.fn()} syncResult={null} syncing={false} />);
    expect(screen.getByText("Skill 配置已同步。").className).toContain("w-fit");
  });

  it("keeps backup results reviewable until dismissed", async () => {
    const dismiss = vi.fn();
    const user = userEvent.setup();
    render(<SkillDriftBanner
      drift={drift}
      onDismiss={dismiss}
      onSync={vi.fn()}
      syncResult={{
        mounted: [], unmounted: [], overwritten: ["skill"], restored: [], failed: [],
        backedUp: [{ originalPath: "skill", backupPath: "backup/skill" }], resolvedFrom: drift,
      }}
      syncing={false}
    />);
    expect(screen.getByText(/已备份 1/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "关闭同步结果" }));
    expect(dismiss).toHaveBeenCalledOnce();
  });
});
