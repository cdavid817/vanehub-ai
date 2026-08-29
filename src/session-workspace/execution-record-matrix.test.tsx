// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import {
  evidenceCommandIdSchema,
  evidenceRecordIdSchema,
  evidenceRunIdSchema,
  evidenceSessionIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type {
  CommandExecutionRecord,
  EvidenceFidelity,
  EvidenceStatus,
  ExecutionRecord,
  QueryCoverage,
} from "../types/session-workspace-evidence";
import { ExecutionRecordDetailDrawer } from "./execution-record-detail-drawer";
import { ExecutionRecordRow } from "./execution-record-row";
import { WorkspaceEvidenceScopeProvider } from "./workspace-evidence-scope";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const runId = evidenceRunIdSchema.parse("run-1");
const traceId = evidenceTraceIdSchema.parse("trace-1");
const complete: QueryCoverage = { state: "complete", reasonCodes: [], truncated: false };

function command(overrides: Partial<CommandExecutionRecord> = {}): CommandExecutionRecord {
  return {
    id: evidenceRecordIdSchema.parse("command:cmd-1"),
    kind: "command",
    sessionId,
    status: "succeeded",
    fidelity: "native",
    coverage: complete,
    commandId: evidenceCommandIdSchema.parse("cmd-1"),
    runtimeKind: "local-shell",
    outputAvailability: "merged",
    outputTruncated: false,
    ...overrides,
  };
}

function showRow(record: ExecutionRecord) {
  return render(
    <I18nextProvider i18n={i18n}>
      <ExecutionRecordRow isSelected={false} onSelect={() => undefined} record={record} />
    </I18nextProvider>,
  );
}

function showDrawer(record: ExecutionRecord) {
  return render(
    <I18nextProvider i18n={i18n}>
      <WorkspaceEvidenceScopeProvider seatIds={[]} sessionId={sessionId}>
        <ExecutionRecordDetailDrawer onClose={() => undefined} record={record} />
      </WorkspaceEvidenceScopeProvider>
    </I18nextProvider>,
  );
}

describe("execution record fidelity and status matrix", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("shows each fidelity exactly as reported", () => {
    const expected: Record<EvidenceFidelity, string> = {
      native: "Native",
      proxied: "Proxied",
      inferred: "Inferred",
      opaque: "Opaque",
    };
    for (const [fidelity, label] of Object.entries(expected) as [EvidenceFidelity, string][]) {
      const view = showRow(command({ fidelity }));
      expect(screen.getByTestId(`execution-record-fidelity-${fidelity}`).textContent).toBe(label);
      view.unmount();
    }
  });

  it("shows each status as its own word", () => {
    const seen = new Set<string>();
    for (const status of [
      "queued",
      "running",
      "succeeded",
      "failed",
      "cancelled",
      "incomplete",
    ] as EvidenceStatus[]) {
      const view = showRow(command({ status }));
      const text = view.container.textContent ?? "";
      const word = text.match(/Queued|Running|Succeeded|Failed|Cancelled|Incomplete/)?.[0] ?? "";
      expect(word.length, status).toBeGreaterThan(0);
      expect(seen.has(word), status).toBe(false);
      seen.add(word);
      view.unmount();
    }
  });

  it("renders a completion-only record without inventing a start", () => {
    // The runtime saw the work finish and never saw it begin. Every one of `endedAt`,
    // `durationMs`, and the status could be subtracted into a plausible start, and each would be
    // an observation nobody made.
    showDrawer(command({ endedAt: "2026-08-23T10:00:12.000Z", durationMs: 12_000, status: "incomplete" }));
    const drawer = screen.getByTestId("execution-record-detail");
    const started = within(drawer).getByText("Started").closest("div");
    expect(started?.textContent).toContain("not observed");
  });

  it("says a merged terminal stream is merged rather than offering two streams", () => {
    showDrawer(command({ outputAvailability: "merged" }));
    const drawer = screen.getByTestId("execution-record-detail");
    expect(within(drawer).getByText("Output").closest("div")?.textContent).toContain(
      "merged terminal stream",
    );
  });

  it("distinguishes output that was never captured from output that was redacted", () => {
    const unavailable = showDrawer(command({ outputAvailability: "unavailable" }));
    expect(screen.getByTestId("execution-record-detail").textContent).toContain("unavailable");
    unavailable.unmount();

    showDrawer(command({ outputAvailability: "redacted" }));
    expect(screen.getByTestId("execution-record-detail").textContent).toContain("redacted");
  });

  it("states that truncated output was truncated", () => {
    const view = showDrawer(command({ outputTruncated: false }));
    expect(screen.queryByTestId("execution-record-output-truncated")).toBeNull();
    view.unmount();

    showDrawer(command({ outputTruncated: true }));
    expect(screen.getByTestId("execution-record-output-truncated").textContent).toContain(
      "truncated",
    );
  });

  it("never shows a missing exit code as zero", () => {
    const missing = showRow(command({ status: "failed" }));
    expect(missing.container.textContent).toContain("not observed");
    expect(missing.container.textContent).not.toMatch(/code\s*0/);
    missing.unmount();

    const zero = showRow(command({ exitCode: 0 }));
    expect(zero.container.textContent).toContain("code 0");
  });

  it("shows a redacted command as redacted rather than as an empty line", () => {
    const view = showRow(command({ redactedDisplay: undefined }));
    expect(view.container.textContent).toContain("redacted");
    view.unmount();

    const shown = showRow(command({ redactedDisplay: "npm test" }));
    expect(shown.container.textContent).toContain("npm test");
  });

  it("marks a legacy row as coming from message history", () => {
    showRow({
      id: evidenceRecordIdSchema.parse("legacy:m1:t1"),
      kind: "legacy",
      sessionId,
      status: "succeeded",
      fidelity: "inferred",
      coverage: { state: "partial", reasonCodes: [], truncated: true },
      label: "shell toolUse",
      source: "message-history",
      messageId: "m1",
    });

    expect(screen.getByTestId("execution-record-legacy-source").textContent).toContain(
      "message history",
    );
    expect(screen.getByTestId("execution-record-fidelity-inferred")).toBeTruthy();
  });

  it("reaches a row and its details by keyboard alone", async () => {
    const user = userEvent.setup();
    showDrawer(command({ runId, traceId }));

    // The drawer takes focus when it opens, so a keyboard reader is not left on a row behind it.
    expect(document.activeElement?.getAttribute("aria-label")).toBe("Close record details");

    await user.tab();
    expect(screen.getByTestId("execution-record-action-trace")).toBe(document.activeElement);
  });

  it("offers a cross-panel action only where the destination can be built", () => {
    const withRun = showDrawer(command({ runId, traceId }));
    expect(screen.getByTestId("execution-record-actions")).toBeTruthy();
    withRun.unmount();

    showDrawer(command());
    // Nothing to correlate on, so no button — rather than a button that opens an unfiltered panel.
    expect(screen.queryByTestId("execution-record-actions")).toBeNull();
  });

  it("uses semantic tokens rather than a fixed palette", () => {
    const view = showRow(command({ status: "failed" }));
    const markup = view.container.innerHTML;
    // Both themes recolour through the same tokens; a literal hex or a Tailwind palette shade
    // would look right in whichever theme it was written against and wrong in the other.
    expect(markup).not.toMatch(/#[0-9a-f]{6}/i);
    expect(markup).not.toMatch(/\b(?:bg|text|border)-(?:zinc|slate|gray|red|green)-\d{2,3}\b/);
    expect(markup).toContain("text-destructive");
  });
});
