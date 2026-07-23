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
  scheduledTasks: "Scheduled tasks",
  settings: "Settings",
  help: "Help",
};

function groupButtons(element: ReactElement, groupIndex: number) {
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
      <WorkspaceActivityBar activeDestination="sessions" labels={labels} onLoops={vi.fn()} onOpenSettings={vi.fn()} onScheduledTasks={vi.fn()} onSessions={vi.fn()} sessionSidebarExpanded />,
    );

    expect(html).toContain('aria-label="Workspace navigation"');
    expect(html).toContain('data-activity-group="primary"');
    expect(html).toContain('data-activity-group="utility"');
    expect(html.indexOf('title="Collapse sessions"')).toBeLessThan(html.indexOf('title="Scheduled tasks"'));
    expect(html.indexOf('title="Settings"')).toBeLessThan(html.indexOf('title="Help"'));
    expect(html).toContain('aria-controls="workspace-session-sidebar"');
    expect(html).toContain('aria-expanded="true"');
    expect(html).not.toContain(">Sessions<");
  });

  it("exposes the collapsed action and forwards activity callbacks", () => {
    const onSessions = vi.fn();
    const onLoops = vi.fn();
    const onScheduledTasks = vi.fn();
    const onOpenSettings = vi.fn();
    const element = WorkspaceActivityBar({ activeDestination: "loops", labels, onLoops, onOpenSettings, onScheduledTasks, onSessions, sessionSidebarExpanded: false });
    const primaryButtons = groupButtons(element, 0);
    const utilityButtons = groupButtons(element, 1);

    primaryButtons[0].props.onClick?.({} as never);
    primaryButtons[1].props.onClick?.({} as never);
    primaryButtons[2].props.onClick?.({} as never);
    utilityButtons[0].props.onClick?.({} as never);

    expect(onSessions).toHaveBeenCalledOnce();
    expect(onLoops).toHaveBeenCalledOnce();
    expect(onScheduledTasks).toHaveBeenCalledOnce();
    expect(onOpenSettings).toHaveBeenCalledOnce();
    expect(renderToStaticMarkup(element)).toContain('title="Expand sessions"');
    expect(utilityButtons[1].props.title).toBe("Help");
  });
});
