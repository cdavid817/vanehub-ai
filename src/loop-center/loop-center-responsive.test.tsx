import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import "../i18n";
import { LoopCenter } from "./loop-center";
import { loopRunFixture } from "../test/loop-fixtures";
import { LoopInspector } from "./loop-inspector";
import { LoopTimeline } from "./loop-timeline";

describe("LoopCenter responsive navigation", () => {
  /**
   * 17.3: the fixed three-column CSS grid + hand-rolled drawer/backdrop/focus-trap this used to
   * assert on (literal `min-[1024px]:grid-cols-` strings, a `translate-x-full` closed-transform
   * class) is gone, replaced by the shared `DestinationLayout`/`HorizontalPaneRegion` primitive
   * Session Work already uses, with the inspector column itself now the shared `Inspector`
   * component (Piece B) rather than the former bespoke `LoopInspector` render. No SSR/jsdom
   * `ResizeObserver` ever fires (see `DestinationLayout.test.tsx`'s own note, and
   * `DestinationLayoutBody.test.tsx` for tier composition covered generically rather than
   * re-proven per caller), so this always renders at its initial "wide" tier -- both panes inline
   * with a real resize gutter each, no Sheet trigger needed yet.
   */
  it("renders navigation and inspector as labelled, bounded, resizable panes at the default (wide) tier", () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const html = renderToStaticMarkup(
      <QueryClientProvider client={queryClient}>
        <LoopCenter />
      </QueryClientProvider>,
    );

    expect(html).toContain('id="loop-navigation-drawer"');
    // One real `SplitPane` resize gutter per pane (role="separator"), each labelled with what it
    // resizes and bounded to this destination's own former fixed-grid min/max
    // (loop-center-regions.tsx's LOOP_NAVIGATION_PANE_BOUNDS/LOOP_INSPECTOR_PANE_BOUNDS) --
    // replaces the old CSS `minmax()` track with a real, user-draggable equivalent.
    expect(html).toContain('aria-label="循环工程" aria-orientation="vertical" aria-valuemax="280" aria-valuemin="220"');
    expect(html).toContain('aria-label="检查器" aria-orientation="vertical" aria-valuemax="340" aria-valuemin="260"');
    expect(html).toContain('role="separator"');

    // The inspector column is the shared Inspector shell (no run selected yet, so it is in
    // overview mode showing Loop Center's own reused "no selection" copy) -- not a bespoke render.
    expect(html).toContain('data-testid="workbench-inspector"');
    expect(html).toContain("未选择运行记录");
    expect(html).toContain("选择一条运行记录后");

    // Both panes are inline at the wide tier -- no Sheet-open trigger exists yet to find.
    expect(html).not.toContain('title="打开循环列表"');
    expect(html).not.toContain('title="打开循环检查器"');
  });

  it("keeps primary run actions in the responsive center surface instead of the inspector", () => {
    const queryClient = new QueryClient();
    const run = loopRunFixture("running");
    const center = renderToStaticMarkup(<QueryClientProvider client={queryClient}><LoopTimeline run={run} /></QueryClientProvider>);
    const inspector = renderToStaticMarkup(
      <MemoryRouter>
        <QueryClientProvider client={queryClient}><LoopInspector loading={false} run={run} /></QueryClientProvider>
      </MemoryRouter>,
    );

    expect(center).toContain("sticky -top-3");
    expect(center).toContain("暂停");
    // 17.8: Stop is no longer its own always-visible button (Pause is the one primary action for
    // a running Loop) -- it moved into the closed-by-default More menu, so only the menu's own
    // trigger is checked here, not "停止" itself. loop-run-controls.test.tsx exercises what's
    // actually inside the menu once opened.
    expect(center).toContain('aria-haspopup="menu"');
    expect(inspector).not.toContain("运行控制");
    expect(inspector).not.toContain(">暂停<");
  });
});
