// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import "../i18n";
import { loopRunFixture } from "../test/loop-fixtures";
import type { LoopRun } from "../types/loop";
import { LoopAcceptancePanel } from "./loop-acceptance-panel";

function renderPanel(run: LoopRun) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  return render(<QueryClientProvider client={client}><LoopAcceptancePanel run={run} /></QueryClientProvider>);
}

describe("LoopAcceptancePanel", () => {
  it("renders nothing once the run leaves awaiting-acceptance", () => {
    const { container } = renderPanel(loopRunFixture("running"));
    expect(container.firstChild).toBeNull();
  });

  it("sticks beneath the run header at every one of its own real breakpoint-height tiers", () => {
    renderPanel(loopRunFixture("awaiting-acceptance"));
    const panel = screen.getByLabelText("人工验收");
    // Task 17.13: mirrors LoopRunHeader's own sticky treatment (loop-run-header.tsx:14). The three
    // top tiers are that header's own real measured height (not a guess) at each of its <dl>
    // breakpoints -- see this component's own comment for how the numbers were derived.
    expect(panel.className).toContain("sticky");
    expect(panel.className).toContain("top-[196px]");
    expect(panel.className).toContain("sm:top-[144px]");
    expect(panel.className).toContain("lg:top-[96px]");
    expect(panel.className).toContain("z-20");
    // Occludes iteration rows scrolling underneath once stuck -- the pre-17.13 5%-opacity warning
    // wash would let that content bleed through.
    expect(panel.className).toContain("bg-background/95");
  });

  it("shows remaining and elapsed budget without an exhausted warning while time remains", () => {
    const run = loopRunFixture("awaiting-acceptance", { startedAt: new Date().toISOString() });
    run.definitionSnapshot.limits.totalTimeoutSeconds = 3600;
    renderPanel(run);
    const panel = within(screen.getByLabelText("人工验收"));
    expect(panel.getByText("预算")).toBeTruthy();
    expect(panel.getByText("剩余预算")).toBeTruthy();
    expect(panel.getByText("已用时间")).toBeTruthy();
    expect(panel.queryByText("本次运行的时间预算已耗尽。")).toBeNull();
  });

  it("shows the exhausted warning once the definition's own time budget is used up", () => {
    const run = loopRunFixture("awaiting-acceptance");
    run.definitionSnapshot.limits.totalTimeoutSeconds = 0;
    renderPanel(run);
    const panel = within(screen.getByLabelText("人工验收"));
    expect(panel.getByText("本次运行的时间预算已耗尽。")).toBeTruthy();
  });

  it("still reports budget when the run has no iteration evidence yet", () => {
    const run = loopRunFixture("awaiting-acceptance", { iterations: [] });
    run.definitionSnapshot.limits.totalTimeoutSeconds = 3600;
    renderPanel(run);
    const panel = within(screen.getByLabelText("人工验收"));
    expect(panel.getByText("现有证据不足，尚未评估")).toBeTruthy();
    expect(panel.getByText("预算")).toBeTruthy();
  });
});
