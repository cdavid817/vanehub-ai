// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import {
  evidenceCommandIdSchema,
  evidenceRunIdSchema,
  evidenceSessionIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { WorkspaceEvidenceTabId } from "../types/session-workspace-evidence";
import {
  WorkspaceEvidenceScopeProvider,
  useWorkspaceEvidenceScope,
} from "./workspace-evidence-scope";
import { WorkspaceEvidenceScopeChips } from "./workspace-evidence-scope-chips";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const runId = evidenceRunIdSchema.parse("run-1");
const traceId = evidenceTraceIdSchema.parse("trace-1");
const commandId = evidenceCommandIdSchema.parse("command-1");

/** Applies one navigation on mount so the chips describe a real destination, not a fixture. */
function Navigator({ tab }: { tab: WorkspaceEvidenceTabId }) {
  const { navigate, scope } = useWorkspaceEvidenceScope();
  return (
    <div>
      <button
        onClick={() =>
          navigate({
            tab,
            scope: { sessionId, runId, traceId, commandId, relativePath: "src/main.rs" },
          })
        }
        type="button"
      >
        go
      </button>
      {/* A span, not an output: `output` carries the status role the unsupported notice uses. */}
      <span data-testid="scope">{JSON.stringify(scope)}</span>
    </div>
  );
}

function mount(children: ReactNode) {
  return render(
    <I18nextProvider i18n={i18n}>
      <WorkspaceEvidenceScopeProvider seatIds={[]} sessionId={sessionId}>
        {children}
      </WorkspaceEvidenceScopeProvider>
    </I18nextProvider>,
  );
}

describe("WorkspaceEvidenceScopeChips", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("renders nothing while no filter is applied", () => {
    mount(<WorkspaceEvidenceScopeChips tab="logs" />);
    expect(screen.queryByTestId("workspace-scope-chips")).toBeNull();
  });

  it("shows only the fields the destination applies", async () => {
    const user = userEvent.setup();
    mount(
      <>
        <Navigator tab="traces" />
        <WorkspaceEvidenceScopeChips tab="traces" />
      </>,
    );

    await user.click(screen.getByRole("button", { name: "go" }));

    const chips = screen.getByTestId("workspace-scope-chips");
    expect(within(chips).getByText("Run")).toBeTruthy();
    expect(within(chips).getByText("Trace")).toBeTruthy();
    // Traces filters by neither, so claiming them would describe the last navigation rather than
    // this panel — and nothing on screen would say which was true.
    expect(within(chips).queryByText("Command")).toBeNull();
    expect(within(chips).queryByText("File")).toBeNull();
  });

  it("names the field and the value in each clear button", async () => {
    const user = userEvent.setup();
    mount(
      <>
        <Navigator tab="traces" />
        <WorkspaceEvidenceScopeChips tab="traces" />
      </>,
    );

    await user.click(screen.getByRole("button", { name: "go" }));

    // Without the value, a row of chips presents as several identically named buttons.
    expect(screen.getByRole("button", { name: "Clear the Run filter run-1" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Clear the Trace filter trace-1" })).toBeTruthy();
  });

  it("clears one field and everything that field owns", async () => {
    const user = userEvent.setup();
    mount(
      <>
        <Navigator tab="logs" />
        <WorkspaceEvidenceScopeChips tab="logs" />
      </>,
    );

    await user.click(screen.getByRole("button", { name: "go" }));
    await user.click(screen.getByRole("button", { name: "Clear the Run filter run-1" }));

    const scope: { runId?: string; traceId?: string } = JSON.parse(
      screen.getByTestId("scope").textContent ?? "null",
    );
    expect(scope.runId).toBeUndefined();
    expect(scope.traceId).toBeUndefined();
  });

  it("clears every filter at once", async () => {
    const user = userEvent.setup();
    mount(
      <>
        <Navigator tab="logs" />
        <WorkspaceEvidenceScopeChips tab="logs" />
      </>,
    );

    await user.click(screen.getByRole("button", { name: "go" }));
    await user.click(screen.getByRole("button", { name: "Clear all filters" }));

    expect(JSON.parse(screen.getByTestId("scope").textContent ?? "null")).toEqual({ sessionId });
    expect(screen.queryByTestId("workspace-scope-chips")).toBeNull();
  });

  it("says which filters the destination will not apply", async () => {
    const user = userEvent.setup();
    mount(
      <>
        <Navigator tab="files" />
        <WorkspaceEvidenceScopeChips tab="files" />
      </>,
    );

    await user.click(screen.getByRole("button", { name: "go" }));

    const notice = screen.getByRole("status");
    expect(notice.textContent).toContain("does not filter by");
    expect(notice.textContent).toContain("Run");
    expect(notice.textContent).toContain("Command");
    expect(screen.getByTestId("workspace-scope-chips").textContent).toContain("src/main.rs");
  });
});
