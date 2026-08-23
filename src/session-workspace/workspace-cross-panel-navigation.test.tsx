// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import {
  evidenceCommandIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { WorkspaceEvidenceTarget } from "../types/session-workspace-evidence";
import { evidenceTabOf } from "./workspace-evidence-reducer";
import {
  WorkspaceEvidenceScopeProvider,
  useWorkspaceEvidenceScope,
} from "./workspace-evidence-scope";
import { WorkspaceEvidenceScopeChips } from "./workspace-evidence-scope-chips";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const seatId = evidenceSeatIdSchema.parse("seat-1");
const runId = evidenceRunIdSchema.parse("run-124");
const traceId = evidenceTraceIdSchema.parse("trace-9");
const spanId = evidenceSpanIdSchema.parse("span-7");
const commandId = evidenceCommandIdSchema.parse("command-3");
const relativePath = "src/main.rs";

/**
 * Every cross-panel jump the console offers, as the target its source row hands over.
 *
 * Written as data because the property under test is the same for all of them: the destination
 * changes together with the filter, and the filter that arrives is the one the source named — not
 * that filter merged with whatever the previous destination happened to be showing.
 */
const JOURNEYS: Array<{
  name: string;
  target: WorkspaceEvidenceTarget;
  chips: string[];
  ignored: string[];
}> = [
  {
    name: "a log row opens its span",
    target: { tab: "traces", scope: { sessionId, traceId, spanId }, focus: "detail" },
    chips: ["Trace", "Span"],
    ignored: [],
  },
  {
    name: "a span opens the command it ran",
    target: { tab: "terminal", scope: { sessionId, runId, commandId }, focus: "row" },
    chips: ["Run", "Command"],
    ignored: [],
  },
  {
    name: "a command opens the file it changed",
    target: {
      tab: "changes",
      scope: { sessionId, relativePath, hunkFingerprint: "hunk-2" },
      focus: "row",
    },
    chips: ["File", "Hunk"],
    ignored: [],
  },
  {
    name: "a review finding opens the run that produced it",
    target: { tab: "traces", scope: { sessionId, runId }, focus: "row" },
    chips: ["Run"],
    ignored: [],
  },
  {
    name: "a summary row opens the tab that owns it",
    target: { tab: "logs", scope: { sessionId, seatId }, focus: "filter" },
    chips: ["Seat"],
    ignored: [],
  },
  {
    name: "a command row opens the file tree, which cannot filter by command",
    target: { tab: "files", scope: { sessionId, commandId, relativePath }, focus: "row" },
    chips: ["File"],
    ignored: ["Command"],
  },
];

/** Renders the destination's chips, so an assertion reads what the user would see. */
function Workspace({ target }: { target: WorkspaceEvidenceTarget }) {
  const { activeTab, focus, navigate, navigationRevision } = useWorkspaceEvidenceScope();
  const destination = evidenceTabOf(activeTab);
  return (
    <div>
      <button onClick={() => navigate(target)} type="button">
        open
      </button>
      <span data-testid="tab">{activeTab}</span>
      <span data-testid="focus">{focus ?? "-"}</span>
      <span data-testid="revision">{navigationRevision}</span>
      {destination === null ? null : <WorkspaceEvidenceScopeChips tab={destination} />}
    </div>
  );
}

function mount(target: WorkspaceEvidenceTarget) {
  return render(
    <I18nextProvider i18n={i18n}>
      <WorkspaceEvidenceScopeProvider seatIds={[seatId]} sessionId={sessionId}>
        <Workspace target={target} />
      </WorkspaceEvidenceScopeProvider>
    </I18nextProvider>,
  );
}

describe("cross-panel evidence navigation", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  for (const journey of JOURNEYS) {
    it(journey.name, async () => {
      const user = userEvent.setup();
      mount(journey.target);

      expect(screen.getByTestId("tab").textContent).toBe("chat");
      await user.click(screen.getByRole("button", { name: "open" }));

      expect(screen.getByTestId("tab").textContent).toBe(journey.target.tab);
      expect(screen.getByTestId("focus").textContent).toBe(journey.target.focus ?? "-");
      // The revision is what lets an already-open destination re-focus on a second jump.
      expect(screen.getByTestId("revision").textContent).toBe("1");

      const chips = screen.getByTestId("workspace-scope-chips");
      for (const label of journey.chips) expect(within(chips).getByText(label)).toBeTruthy();
      for (const label of journey.ignored) {
        // Present in the target, absent from the chips, and named in the notice: a destination
        // that quietly dropped it would render everything and look filtered.
        expect(within(chips).queryByText(label)).toBeNull();
        expect(screen.getByRole("status").textContent).toContain(label);
      }
    });
  }

  it("carries no trace of the panel the user came from", async () => {
    const user = userEvent.setup();
    function Chain() {
      const { activeTab, correlation, navigate } = useWorkspaceEvidenceScope();
      return (
        <div>
          <button
            onClick={() => navigate({ tab: "traces", scope: { sessionId, runId, traceId, spanId } })}
            type="button"
          >
            open span
          </button>
          <button
            onClick={() => navigate({ tab: "terminal", scope: { sessionId, commandId } })}
            type="button"
          >
            open command
          </button>
          <span data-testid="tab">{activeTab}</span>
          <span data-testid="correlation">{JSON.stringify(correlation)}</span>
        </div>
      );
    }
    render(
      <I18nextProvider i18n={i18n}>
        <WorkspaceEvidenceScopeProvider seatIds={[seatId]} sessionId={sessionId}>
          <Chain />
        </WorkspaceEvidenceScopeProvider>
      </I18nextProvider>,
    );

    await user.click(screen.getByRole("button", { name: "open span" }));
    await user.click(screen.getByRole("button", { name: "open command" }));

    // A merge would make "show me this command" mean "this command, still inside that trace".
    expect(JSON.parse(screen.getByTestId("correlation").textContent ?? "null")).toEqual({
      commandId,
    });
    expect(screen.getByTestId("tab").textContent).toBe("terminal");
  });
});
