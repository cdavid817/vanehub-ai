import { Children, isValidElement, type ButtonHTMLAttributes, type ReactElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { WorkspaceActivityBar, type WorkspaceActivityBarLabels } from "./workspace-activity-bar";

const labels: WorkspaceActivityBarLabels = {
  navigation: "Workspace navigation",
  sessions: "Sessions",
  expandSessions: "Expand sessions",
  collapseSessions: "Collapse sessions",
  loops: "Loops",
  plans: "Plans",
  scheduledTasks: "Scheduled tasks",
  todoBoard: "Todo Board",
  goals: "Goals",
  evaluations: "Evaluations",
  settings: "Settings",
  help: "Help",
};

function groupButtons(element: ReactElement<{ children: ReactNode }>, groupIndex: number) {
  const group = Children.toArray(element.props.children as ReactNode)[groupIndex];
  if (!isValidElement<{ children: ReactNode }>(group)) throw new Error("Expected activity group");
  return Children.toArray(group.props.children).map((child) => {
    if (!isValidElement<ButtonHTMLAttributes<HTMLButtonElement>>(child)) throw new Error("Expected activity button");
    return child;
  });
}

describe("WorkspaceActivityBar", () => {
  it("renders icon-only primary and utility groups with accessible state", () => {
    const html = renderToStaticMarkup(
      <WorkspaceActivityBar activeDestination="sessions" labels={labels} onEvaluations={vi.fn()} onHelp={vi.fn()} onLoops={vi.fn()} onOpenSettings={vi.fn()} onPlans={vi.fn()} onScheduledTasks={vi.fn()} onSessions={vi.fn()} onGoals={vi.fn()} onWorkBoard={vi.fn()} sessionSidebarExpanded />,
    );

    expect(html).toContain('aria-label="Workspace navigation"');
    expect(html).toContain('data-activity-group="primary"');
    // Scheduled tasks opens a dialog, so it is grouped away from the destination entries.
    expect(html).toContain('data-activity-group="tools"');
    expect(html).toContain('data-activity-group="utility"');
    expect(html.indexOf('title="Todo Board"')).toBeLessThan(html.indexOf('title="Scheduled tasks"'));
    expect(html.indexOf('title="Collapse sessions"')).toBeLessThan(html.indexOf('title="Scheduled tasks"'));
    expect(html.indexOf('title="Settings"')).toBeLessThan(html.indexOf('title="Help"'));
    expect(html).toContain('aria-controls="workspace-session-sidebar"');
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain('data-testid="desktop-smoke-settings"');
    expect(html).not.toContain(">Sessions<");
  });

  it("exposes the collapsed action and forwards activity callbacks", () => {
    const onSessions = vi.fn();
    const onLoops = vi.fn();
    const onPlans = vi.fn();
    const onScheduledTasks = vi.fn();
    const onWorkBoard = vi.fn();
    const onGoals = vi.fn();
    const onEvaluations = vi.fn();
    const onOpenSettings = vi.fn();
    const onHelp = vi.fn();
    const element = WorkspaceActivityBar({ activeDestination: "loops", labels, onEvaluations, onGoals, onHelp, onLoops, onOpenSettings, onPlans, onScheduledTasks, onSessions, onWorkBoard, sessionSidebarExpanded: false });
    const destinationButtons = groupButtons(element, 0);
    const toolButtons = groupButtons(element, 1);
    const utilityButtons = groupButtons(element, 2);

    destinationButtons[0].props.onClick?.({} as never);
    destinationButtons[1].props.onClick?.({} as never);
    destinationButtons[2].props.onClick?.({} as never);
    destinationButtons[3].props.onClick?.({} as never);
    destinationButtons[5].props.onClick?.({} as never);
    toolButtons[0].props.onClick?.({} as never);
    utilityButtons[0].props.onClick?.({} as never);
    utilityButtons[1].props.onClick?.({} as never);

    expect(onSessions).toHaveBeenCalledOnce();
    expect(onLoops).toHaveBeenCalledOnce();
    expect(onPlans).toHaveBeenCalledOnce();
    expect(onScheduledTasks).toHaveBeenCalledOnce();
    expect(onWorkBoard).toHaveBeenCalledOnce();
    expect(onEvaluations).toHaveBeenCalledOnce();
    expect(onOpenSettings).toHaveBeenCalledWith();
    expect(renderToStaticMarkup(element)).toContain('title="Expand sessions"');
    expect(utilityButtons[1].props.title).toBe("Help");
    expect(onHelp).toHaveBeenCalledOnce();
  });
});
