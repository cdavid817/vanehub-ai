// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEffect, useRef, type ReactNode } from "react";
import { describe, expect, it } from "vitest";
import {
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import {
  WorkspaceEvidenceScopeProvider,
  useWorkspaceEvidenceScope,
} from "./workspace-evidence-scope";

const sessionA = evidenceSessionIdSchema.parse("session-a");
const sessionB = evidenceSessionIdSchema.parse("session-b");
const seatId = evidenceSeatIdSchema.parse("seat-1");
const runId = evidenceRunIdSchema.parse("run-1");
const traceId = evidenceTraceIdSchema.parse("trace-1");
const spanId = evidenceSpanIdSchema.parse("span-1");

/** Records the scope every render saw, which is where a one-frame stale filter shows up. */
function ScopeProbe({ onRender }: { onRender?: (key: string) => void }) {
  const { activeTab, clearScope, navigate, navigationRevision, patchScope, scope } =
    useWorkspaceEvidenceScope();
  const key = JSON.stringify(scope);
  onRender?.(key);
  return (
    <div>
      <output data-testid="scope">{key}</output>
      <output data-testid="tab">{activeTab}</output>
      <output data-testid="revision">{navigationRevision}</output>
      <button
        onClick={() => navigate({ tab: "logs", scope: { sessionId: sessionA, runId, spanId } })}
        type="button"
      >
        open logs
      </button>
      <button onClick={() => patchScope({ traceId })} type="button">
        filter trace
      </button>
      <button onClick={() => clearScope(["runId"])} type="button">
        clear run
      </button>
    </div>
  );
}

function mount(children: ReactNode, sessionId = sessionA, seatIds: readonly string[] = [seatId]) {
  const view = render(
    <WorkspaceEvidenceScopeProvider seatIds={seatIds} sessionId={sessionId}>
      {children}
    </WorkspaceEvidenceScopeProvider>,
  );
  return {
    ...view,
    reopen: (next: typeof sessionId, nextSeats: readonly string[] = seatIds) =>
      view.rerender(
        <WorkspaceEvidenceScopeProvider seatIds={nextSeats} sessionId={next}>
          {children}
        </WorkspaceEvidenceScopeProvider>,
      ),
  };
}

describe("WorkspaceEvidenceScopeProvider", () => {
  it("moves tab and scope together", async () => {
    const user = userEvent.setup();
    mount(<ScopeProbe />);

    await user.click(screen.getByRole("button", { name: "open logs" }));

    expect(screen.getByTestId("tab").textContent).toBe("logs");
    expect(JSON.parse(screen.getByTestId("scope").textContent ?? "null")).toEqual({
      sessionId: sessionA,
      runId,
      spanId,
    });
    expect(screen.getByTestId("revision").textContent).toBe("1");
  });

  it("never renders a query scope carrying the previous session", async () => {
    const user = userEvent.setup();
    const seen: string[] = [];
    const view = mount(<ScopeProbe onRender={(key) => seen.push(key)} />);

    await user.click(screen.getByRole("button", { name: "open logs" }));
    seen.length = 0;
    view.reopen(sessionB);

    // Every frame after the switch, not just the settled one. An effect-based reset would leave
    // one render holding session A's run id, and that render is a real request.
    for (const key of seen) {
      const scope: { sessionId?: string; runId?: string } = JSON.parse(key);
      expect(scope.sessionId).toBe(sessionB);
      expect(scope.runId).toBeUndefined();
    }
    expect(seen.length).toBeGreaterThan(0);
    expect(screen.getByTestId("tab").textContent).toBe("chat");
  });

  it("drops a seat filter when that seat leaves the session", async () => {
    const user = userEvent.setup();
    const view = mount(<ScopeProbe />);

    await user.click(screen.getByRole("button", { name: "open logs" }));
    await user.click(screen.getByRole("button", { name: "filter trace" }));
    view.reopen(sessionA, []);

    const scope: { traceId?: string; seatId?: string } = JSON.parse(
      screen.getByTestId("scope").textContent ?? "null",
    );
    expect(scope.seatId).toBeUndefined();
    // Only the seat: re-validating the roster must not clear an unrelated filter.
    expect(scope.traceId).toBe(traceId);
  });

  it("clears a field together with what it owns", async () => {
    const user = userEvent.setup();
    mount(<ScopeProbe />);

    await user.click(screen.getByRole("button", { name: "open logs" }));
    await user.click(screen.getByRole("button", { name: "clear run" }));

    expect(JSON.parse(screen.getByTestId("scope").textContent ?? "null")).toEqual({
      sessionId: sessionA,
    });
  });

  it("refuses to answer outside the provider", () => {
    function Orphan() {
      useWorkspaceEvidenceScope();
      return null;
    }
    // Answering with an empty scope would render the whole session and look correct.
    expect(() => render(<Orphan />)).toThrow(/WorkspaceEvidenceScopeProvider/);
  });

  it("keeps the navigation callbacks stable across scope changes", async () => {
    const user = userEvent.setup();
    const identities: unknown[] = [];
    function CallbackProbe() {
      const { navigate } = useWorkspaceEvidenceScope();
      const first = useRef(navigate);
      useEffect(() => {
        identities.push(navigate === first.current);
      });
      return (
        <button
          onClick={() => navigate({ tab: "traces", scope: { sessionId: sessionA, runId } })}
          type="button"
        >
          open traces
        </button>
      );
    }
    mount(<CallbackProbe />);

    await user.click(screen.getByRole("button", { name: "open traces" }));

    // A callback that changed identity on every scope change would re-run every panel effect that
    // depends on it, which is how a "navigate" turns into a refetch storm.
    expect(identities.every(Boolean)).toBe(true);
  });
});
