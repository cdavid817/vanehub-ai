// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode, useRef, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { EvidenceSessionId } from "../types/session-workspace-evidence";
import { useMountedWorkspaceTabs } from "./use-mounted-workspace-tabs";
import {
  WorkspaceEvidenceScopeProvider,
  useWorkspaceEvidenceScope,
} from "./workspace-evidence-scope";

const sessionA = evidenceSessionIdSchema.parse("session-a");
const sessionB = evidenceSessionIdSchema.parse("session-b");
const seatId = evidenceSeatIdSchema.parse("seat-1");
const runId = evidenceRunIdSchema.parse("run-1");
const traceId = evidenceTraceIdSchema.parse("trace-1");

/**
 * Every scope the children were rendered with, in order.
 *
 * The reset happens during the provider's render, and the property under test is about frames that
 * never reach a settled state — asserting only the final value would pass on an implementation
 * that showed the previous session's filters for one frame and then corrected itself.
 *
 * Exercises two primary surfaces (Changes/Report) rather than a Runtime Panel surface: since the
 * split into `activePrimarySurface`/`activeRuntimeSurface`, `useMountedWorkspaceTabs` only ever
 * tracks the primary four — a runtime surface's own mount/unmount lifecycle belongs to
 * `RuntimePanel`'s internal state instead, which is out of scope for this reset invariant.
 */
function Probe({ onRender, sessionId }: { onRender: (frame: string) => void; sessionId: EvidenceSessionId }) {
  const { activePrimarySurface, activateSurface, navigate, scope } = useWorkspaceEvidenceScope();
  const { mount, mountedTabs } = useMountedWorkspaceTabs(sessionId, activePrimarySurface);
  const renders = useRef(0);
  renders.current += 1;
  onRender(JSON.stringify({ activeSurface: activePrimarySurface, mounted: [...mountedTabs], scope }));
  return (
    <div>
      <span data-testid="tab">{activePrimarySurface}</span>
      <span data-testid="mounted">{[...mountedTabs].join(",")}</span>
      <span data-testid="scope">{JSON.stringify(scope)}</span>
      <span data-testid="renders">{renders.current}</span>
      <button
        onClick={() => {
          mount("changes");
          navigate({ tab: "changes", scope: { sessionId, runId, traceId }, focus: "row" });
        }}
        type="button"
      >
        open changes
      </button>
      <button onClick={() => { mount("report"); activateSurface("report"); }} type="button">
        open report
      </button>
    </div>
  );
}

function mount(sessionId: EvidenceSessionId, onRender: (frame: string) => void, strict: boolean) {
  const tree = (id: EvidenceSessionId): ReactElement => (
    <WorkspaceEvidenceScopeProvider seatIds={[seatId]} sessionId={id}>
      <Probe onRender={onRender} sessionId={id} />
    </WorkspaceEvidenceScopeProvider>
  );
  const wrap = (id: EvidenceSessionId) =>
    strict ? <StrictMode>{tree(id)}</StrictMode> : tree(id);
  const view = render(wrap(sessionId));
  return { ...view, reopen: (id: EvidenceSessionId) => view.rerender(wrap(id)) };
}

describe("workspace session reset", () => {
  let warnings: string[];
  let errors: string[];

  beforeEach(() => {
    warnings = [];
    errors = [];
    vi.spyOn(console, "warn").mockImplementation((...args) => warnings.push(args.join(" ")));
    vi.spyOn(console, "error").mockImplementation((...args) => errors.push(args.join(" ")));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("updates during render without React objecting, under StrictMode", async () => {
    const user = userEvent.setup();
    const view = mount(sessionA, () => undefined, true);

    await user.click(screen.getByRole("button", { name: "open changes" }));
    view.reopen(sessionB);
    view.reopen(sessionA);

    // A render-phase update to another component is the thing React warns about, and it is the
    // mistake this pattern is one keystroke away from.
    const complaints = [...warnings, ...errors].filter((line) =>
      /Cannot update a component|update.*while rendering/i.test(line),
    );
    expect(complaints).toEqual([]);
  });

  it("does not re-dispatch for a session it already holds", () => {
    const view = mount(sessionA, () => undefined, false);
    const before = Number(screen.getByTestId("renders").textContent);

    view.reopen(sessionA);
    view.reopen(sessionA);

    // Three renders of the same session cost three renders, not six: an unconditional dispatch
    // would re-render once more for each, and a self-feeding one would never settle.
    expect(Number(screen.getByTestId("renders").textContent)).toBe(before + 2);
  });

  it("settles rather than oscillating between the scope and the mounted tabs", async () => {
    const user = userEvent.setup();
    const view = mount(sessionA, () => undefined, false);
    await user.click(screen.getByRole("button", { name: "open changes" }));
    const before = Number(screen.getByTestId("renders").textContent);

    view.reopen(sessionB);

    // The two pieces of state reset independently during one render. If either fed the other, the
    // switch would cost an unbounded number of renders instead of a bounded handful.
    const cost = Number(screen.getByTestId("renders").textContent) - before;
    expect(cost).toBeGreaterThan(0);
    expect(cost).toBeLessThanOrEqual(3);
  });

  it("renders no frame pairing a new session with the previous one's filters", async () => {
    const user = userEvent.setup();
    const frames: string[] = [];
    const view = mount(sessionA, (frame) => frames.push(frame), false);

    await user.click(screen.getByRole("button", { name: "open changes" }));
    frames.length = 0;
    view.reopen(sessionB);

    expect(frames.length).toBeGreaterThan(0);
    for (const frame of frames) {
      const state: { activeSurface: string; mounted: string[]; scope: { sessionId?: string; runId?: string } } =
        JSON.parse(frame);
      expect(state.scope.sessionId).toBe(sessionB);
      // The filter, the surface, and the mounted set all belong to the new session on every frame.
      expect(state.scope.runId).toBeUndefined();
      expect(state.activeSurface).toBe("work");
      expect(state.mounted).toEqual(["work"]);
    }
  });

  it("returns a session to a clean scope rather than to the one it was left in", async () => {
    const user = userEvent.setup();
    const view = mount(sessionA, () => undefined, false);
    await user.click(screen.getByRole("button", { name: "open changes" }));
    expect(screen.getByTestId("tab").textContent).toBe("changes");

    view.reopen(sessionB);
    view.reopen(sessionA);

    // Coming back is not resuming: the correlation belonged to a view of the session that the
    // workspace has since torn down, and re-applying it would filter by a run the panels never
    // fetched.
    expect(screen.getByTestId("tab").textContent).toBe("work");
    expect(JSON.parse(screen.getByTestId("scope").textContent ?? "null")).toEqual({
      sessionId: sessionA,
    });
    expect(screen.getByTestId("mounted").textContent).toBe("work");
  });
});
