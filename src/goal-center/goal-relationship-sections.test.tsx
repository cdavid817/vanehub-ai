// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { GoalLink } from "../contracts/goal";
import { GoalRelationshipSections } from "./goal-relationship-sections";

// Labels below are zh-CN: this codebase's test harness defaults to zh-CN, not English
// (goal-center.test.tsx's own established convention -- see e.g. its "已失效"/"不计入" assertions,
// reused verbatim below since this component renders the same `goals.linkProgress.*` keys).
function links(count: number, overrides: Partial<GoalLink> = {}): GoalLink[] {
  return Array.from({ length: count }, (_, index) => ({
    targetKind: "loop", targetId: `loop-${index + 1}`, progress: "active", ...overrides,
  }));
}

describe("GoalRelationshipSections", () => {
  it("groups links by kind with a per-group count in the header", () => {
    render(<GoalRelationshipSections links={[
      { targetKind: "loop", targetId: "loop-1", progress: "active" },
      { targetKind: "work_item", targetId: "item-1", progress: "terminal" },
    ]} onUnlink={vi.fn()} pending={false} />);

    expect(screen.getByText("循环")).toBeTruthy();
    expect(screen.getByText("看板项")).toBeTruthy();
    expect(screen.getByText("loop-1")).toBeTruthy();
    expect(screen.getByText("item-1")).toBeTruthy();
  });

  it("renders an unresolvable link explicitly rather than dropping it from the view", () => {
    render(<GoalRelationshipSections links={[
      { targetKind: "loop", targetId: "loop-gone", progress: "unresolvable" },
    ]} onUnlink={vi.fn()} pending={false} />);

    expect(screen.getByText("loop-gone")).toBeTruthy();
    expect(screen.getByText("已失效")).toBeTruthy();
  });

  it("marks a session link as not counted rather than showing its raw progress state", () => {
    render(<GoalRelationshipSections links={[
      { targetKind: "session", targetId: "session-1", progress: "active" },
    ]} onUnlink={vi.fn()} pending={false} />);

    expect(screen.getByText("不计入")).toBeTruthy();
  });

  it("calls onUnlink with the link's own kind and id", () => {
    const onUnlink = vi.fn();
    render(<GoalRelationshipSections links={[{ targetKind: "loop", targetId: "loop-1", progress: "active" }]} onUnlink={onUnlink} pending={false} />);

    fireEvent.click(screen.getByRole("button", { name: "解除关联" }));
    expect(onUnlink).toHaveBeenCalledWith("loop", "loop-1");
  });

  it("disables every unlink control while a mutation is pending", () => {
    render(<GoalRelationshipSections links={[{ targetKind: "loop", targetId: "loop-1", progress: "active" }]} onUnlink={vi.fn()} pending={true} />);
    expect((screen.getByRole("button", { name: "解除关联" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("caps a large group at 20 rows and offers to show the rest on request", () => {
    render(<GoalRelationshipSections links={links(25)} onUnlink={vi.fn()} pending={false} />);

    const group = screen.getByText("循环").closest("div") as HTMLElement;
    expect(within(group).getAllByText(/^loop-\d+$/)).toHaveLength(20);
    expect(screen.queryByText("loop-25")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "显示另外 5 项" }));

    expect(screen.getByText("loop-25")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /显示另外 \d+ 项/ })).toBeNull();
  });

  it("does not offer to show more when a group is already at or under the cap", () => {
    render(<GoalRelationshipSections links={links(20)} onUnlink={vi.fn()} pending={false} />);
    expect(screen.queryByRole("button", { name: /显示另外 \d+ 项/ })).toBeNull();
  });
});

/**
 * 20.17: goes beyond the visual (pixel-screenshot) theme-parity coverage added elsewhere this
 * session -- proves the disabled-while-pending unlink control is structurally, not just visually,
 * identical between `futuristic` and `minimal`. This is forced as a real, deterministic structural
 * assertion rather than attempted live in Playwright: `webGoalClient.unlinkGoalTarget`
 * (web-goal-client.ts) has no artificial delay, so the real pending window is a bare microtask --
 * far too fast for a live browser's polling `expect()` to reliably observe -- so `pending` is
 * forced directly as a prop here instead, the same "force the state jsdom/timing can't reliably
 * reach" technique `MissionControlSectionNavView`'s own test seam and `DataTableBody`'s own compact
 * prop already use in this codebase for their own respective jsdom/ResizeObserver limitations.
 *
 * This component itself never reads a theme at all (confirmed by grep: no `useTheme`/`data-theme`
 * reference anywhere under src/goal-center/) -- theming in this codebase is pure CSS custom-property
 * scoping on `:root[data-theme]` (styles.css), never a second JSX tree (design.md Decision 19: "禁止
 * 为主题建立两套 JSX"). Rendering the identical props under each theme's `data-theme` ancestor and
 * diffing the unlink button's own `outerHTML` turns that architectural guarantee into a real,
 * regression-guarding assertion instead of a claim only a source-code grep backs.
 */
describe("GoalRelationshipSections theme parity (20.17)", () => {
  function renderThemed(theme: "futuristic" | "minimal", pending: boolean) {
    const container = document.createElement("div");
    container.dataset.theme = theme;
    document.body.appendChild(container);
    return render(
      <GoalRelationshipSections links={[{ targetKind: "loop", targetId: "loop-1", progress: "active" }]} onUnlink={vi.fn()} pending={pending} />,
      { container },
    );
  }

  it("renders a structurally identical disabled unlink control under both themes while a mutation is pending", () => {
    const futuristic = renderThemed("futuristic", true);
    const futuristicButton = futuristic.getByRole("button", { name: "解除关联" }) as HTMLButtonElement;
    expect(futuristicButton.disabled).toBe(true);
    const futuristicHtml = futuristicButton.outerHTML;
    futuristic.unmount();

    const minimal = renderThemed("minimal", true);
    const minimalButton = minimal.getByRole("button", { name: "解除关联" }) as HTMLButtonElement;
    expect(minimalButton.disabled).toBe(true);
    const minimalHtml = minimalButton.outerHTML;
    minimal.unmount();

    // Same tag, same role, same aria-label, same disabled attribute, same class list -- byte for
    // byte, not just "both look disabled."
    expect(minimalHtml).toBe(futuristicHtml);
  });

  it("renders a structurally identical enabled unlink control under both themes once idle", () => {
    const futuristic = renderThemed("futuristic", false);
    const futuristicButton = futuristic.getByRole("button", { name: "解除关联" }) as HTMLButtonElement;
    expect(futuristicButton.disabled).toBe(false);
    const futuristicHtml = futuristicButton.outerHTML;
    futuristic.unmount();

    const minimal = renderThemed("minimal", false);
    const minimalButton = minimal.getByRole("button", { name: "解除关联" }) as HTMLButtonElement;
    expect(minimalButton.disabled).toBe(false);
    const minimalHtml = minimalButton.outerHTML;
    minimal.unmount();

    expect(minimalHtml).toBe(futuristicHtml);
  });
});
