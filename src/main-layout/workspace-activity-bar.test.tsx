import { Children, isValidElement, type ButtonHTMLAttributes, type ReactElement, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { WorkspaceActivityBar, type WorkspaceActivityBarLabels } from "./workspace-activity-bar";

const labels: WorkspaceActivityBarLabels = {
  navigation: "Workspace navigation",
  sessions: "Sessions",
  expandSessions: "Expand sessions",
  collapseSessions: "Collapse sessions",
  projects: "Projects & Workspaces",
  runs: "Runs",
  plan: "Plan",
  quality: "Quality",
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
  it("renders exactly the five business domains plus the utility group", () => {
    const html = renderToStaticMarkup(
      <WorkspaceActivityBar activeDestination="sessions" labels={labels} onHelp={vi.fn()} onOpenSettings={vi.fn()} onPlan={vi.fn()} onProjects={vi.fn()} onQuality={vi.fn()} onRuns={vi.fn()} onSessions={vi.fn()} sessionSidebarExpanded />,
    );

    expect(html).toContain('aria-label="Workspace navigation"');
    expect(html).toContain('data-activity-group="primary"');
    expect(html).toContain('data-activity-group="utility"');
    // Loops/Board/Goals/Evaluations/Mission Control/Scheduled tasks are no longer primary entries.
    expect(html).not.toContain('title="Loops"');
    expect(html).not.toContain('title="Todo Board"');
    expect(html).not.toContain('title="Scheduled tasks"');
    expect(html).not.toContain('title="Mission Control"');
    // SSR escapes "&" to "&amp;" in attribute values.
    expect(html).toContain('title="Projects &amp; Workspaces"');
    expect(html).toContain('title="Runs"');
    expect(html).toContain('title="Plan"');
    expect(html).toContain('title="Quality"');
    expect(html.indexOf('title="Collapse sessions"')).toBeLessThan(html.indexOf('title="Runs"'));
    expect(html.indexOf('title="Settings"')).toBeLessThan(html.indexOf('title="Help"'));
    expect(html).toContain('aria-controls="workspace-session-sidebar"');
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain('data-testid="desktop-smoke-settings"');
    expect(html).not.toContain(">Sessions<");
  });

  it("exposes the collapsed action and forwards each domain callback", () => {
    const onSessions = vi.fn();
    const onProjects = vi.fn();
    const onRuns = vi.fn();
    const onPlan = vi.fn();
    const onQuality = vi.fn();
    const onOpenSettings = vi.fn();
    const onHelp = vi.fn();
    const element = WorkspaceActivityBar({
      activeDestination: "runs", labels, onHelp, onOpenSettings, onPlan, onProjects, onQuality, onRuns, onSessions, sessionSidebarExpanded: false,
    });
    const destinationButtons = groupButtons(element, 0);
    const utilityButtons = groupButtons(element, 1);

    destinationButtons[0].props.onClick?.({} as never);
    destinationButtons[1].props.onClick?.({} as never);
    destinationButtons[2].props.onClick?.({} as never);
    destinationButtons[3].props.onClick?.({} as never);
    destinationButtons[4].props.onClick?.({} as never);
    utilityButtons[0].props.onClick?.({} as never);
    utilityButtons[1].props.onClick?.({} as never);

    expect(onSessions).toHaveBeenCalledOnce();
    expect(onProjects).toHaveBeenCalledOnce();
    expect(onRuns).toHaveBeenCalledOnce();
    expect(onPlan).toHaveBeenCalledOnce();
    expect(onQuality).toHaveBeenCalledOnce();
    expect(onOpenSettings).toHaveBeenCalledOnce();
    expect(onHelp).toHaveBeenCalledOnce();
    expect(renderToStaticMarkup(element)).toContain('title="Expand sessions"');
    expect(utilityButtons[1].props.title).toBe("Help");
  });

  it("marks the active domain without shifting the other entries (border/background only)", () => {
    const active = renderToStaticMarkup(
      <WorkspaceActivityBar activeDestination="plan" labels={labels} onHelp={vi.fn()} onOpenSettings={vi.fn()} onPlan={vi.fn()} onProjects={vi.fn()} onQuality={vi.fn()} onRuns={vi.fn()} onSessions={vi.fn()} sessionSidebarExpanded />,
    );
    // Attribute order follows JSX declaration order (className before title in the component).
    expect(active).toMatch(/class="[^"]*border-primary[^"]*"[^>]*title="Plan"/);
  });
});
