// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { EvaluationArena, EvaluationAttempt, EvaluationCheck, EvaluationMetric, EvaluationOutcome } from "../types/evaluation";
import { EvaluationResultsTable, type EvaluationResultRow } from "./evaluation-results-table";

function buildRow(input: {
  arenaId: string;
  attemptId: string;
  agentId: string;
  outcome: EvaluationOutcome;
  passed: number;
  total: number;
  tokens: number | null;
  durationMs: number | null;
}): EvaluationResultRow {
  const checks: EvaluationCheck[] = Array.from({ length: input.total }, (_, index) => ({ checkId: `check-${index}`, passed: index < input.passed, summary: "ok" }));
  const metrics: EvaluationMetric[] = [
    { name: "input_tokens", value: input.tokens, unit: "tokens", quality: input.tokens == null ? "unavailable" : "reported", source: "provider" },
    { name: "duration", value: input.durationMs, unit: "ms", quality: input.durationMs == null ? "unavailable" : "reported", source: "runtime" },
  ];
  const attempt: EvaluationAttempt = {
    id: input.attemptId, arenaId: input.arenaId, canonicalRunId: `${input.attemptId}-run`, taskId: "task", taskVersion: 1,
    agent: { agentId: input.agentId, providerId: input.agentId, modelId: null, interactionMode: "cli", configurationFingerprint: "fp" },
    outcome: input.outcome, checks, metrics, contextEvidenceManifestId: null, artifactIds: [], timeline: [],
  };
  const arena: EvaluationArena = { id: input.arenaId, operationId: `${input.arenaId}-op`, taskId: "task", taskVersion: 1, rankingVersion: "v1", attempts: [attempt] };
  return { arena, attempt };
}

const ROW_A = buildRow({ arenaId: "arena-a", attemptId: "attempt-a", agentId: "agent-a", outcome: "succeeded", passed: 2, total: 2, tokens: 500, durationMs: 20_000 });
const ROW_B = buildRow({ arenaId: "arena-b", attemptId: "attempt-b", agentId: "agent-b", outcome: "task_failed", passed: 1, total: 2, tokens: 100, durationMs: 5_000 });

function rowIds() {
  return screen.getAllByTestId("evaluation-row").map((row) => row.getAttribute("data-attempt-id"));
}

describe("EvaluationResultsTable", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
    // jsdom does not implement ResizeObserver; this repo's convention (DataTable.test.tsx) is a
    // no-op stub, which also pins `useTableCompactMode` to its non-compact default for these tests.
    globalThis.ResizeObserver = class {
      observe() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
  });

  it("renders rows through the shared DataTable with formatted metrics and a localized outcome badge", () => {
    render(<EvaluationResultsTable filter="" onExportArena={vi.fn()} onFilterChange={vi.fn()} onSelectAttempt={vi.fn()} rows={[ROW_A, ROW_B]} />);
    expect(screen.getByRole("table", { name: "Results" })).toBeTruthy();
    expect(screen.getByText("agent-a")).toBeTruthy();
    expect(screen.getByText("Passed")).toBeTruthy();
    expect(screen.getByText("Task failed")).toBeTruthy();
    expect(screen.getByText("2/2")).toBeTruthy();
    expect(screen.getByText("500 tokens")).toBeTruthy();
    expect(screen.getByText("20.0 s")).toBeTruthy();
  });

  it("carries the data-testid/data-attempt-id/data-outcome contract the desktop suite reads and fires row activation on click", () => {
    const onSelectAttempt = vi.fn();
    render(<EvaluationResultsTable filter="" onExportArena={vi.fn()} onFilterChange={vi.fn()} onSelectAttempt={onSelectAttempt} rows={[ROW_A, ROW_B]} />);
    const rows = screen.getAllByTestId("evaluation-row");
    expect(rows).toHaveLength(2);
    expect(rows[0].getAttribute("data-attempt-id")).toBe("attempt-a");
    expect(rows[0].getAttribute("data-outcome")).toBe("succeeded");
    fireEvent.click(rows[1]);
    expect(onSelectAttempt).toHaveBeenCalledWith("attempt-b");
  });

  it("exports the clicked row's own arena without also selecting the row", () => {
    const onExportArena = vi.fn();
    const onSelectAttempt = vi.fn();
    render(<EvaluationResultsTable filter="" onExportArena={onExportArena} onFilterChange={vi.fn()} onSelectAttempt={onSelectAttempt} rows={[ROW_A, ROW_B]} />);
    fireEvent.click(screen.getAllByTestId("evaluation-export")[1]);
    expect(onExportArena).toHaveBeenCalledWith(ROW_B.arena);
    expect(onSelectAttempt).not.toHaveBeenCalled();
  });

  it("keeps the given row order until a sortable header is clicked", () => {
    render(<EvaluationResultsTable filter="" onExportArena={vi.fn()} onFilterChange={vi.fn()} onSelectAttempt={vi.fn()} rows={[ROW_A, ROW_B]} />);
    expect(rowIds()).toEqual(["attempt-a", "attempt-b"]);
  });

  it("sorts by tokens ascending then descending on repeated header clicks", () => {
    render(<EvaluationResultsTable filter="" onExportArena={vi.fn()} onFilterChange={vi.fn()} onSelectAttempt={vi.fn()} rows={[ROW_A, ROW_B]} />);
    fireEvent.click(screen.getByRole("button", { name: /Tokens/ }));
    expect(rowIds()).toEqual(["attempt-b", "attempt-a"]);
    fireEvent.click(screen.getByRole("button", { name: /Tokens/ }));
    expect(rowIds()).toEqual(["attempt-a", "attempt-b"]);
  });

  it("renders the controlled filter value and reports changes without owning the value itself", () => {
    const onFilterChange = vi.fn();
    render(<EvaluationResultsTable filter="codex" onExportArena={vi.fn()} onFilterChange={onFilterChange} onSelectAttempt={vi.fn()} rows={[ROW_A]} />);
    const input = screen.getByLabelText("Filter results") as HTMLInputElement;
    expect(input.value).toBe("codex");
    fireEvent.change(input, { target: { value: "next" } });
    expect(onFilterChange).toHaveBeenCalledWith("next");
  });

  it("shows the shared empty state instead of empty table headers when there are no rows", () => {
    render(<EvaluationResultsTable filter="" onExportArena={vi.fn()} onFilterChange={vi.fn()} onSelectAttempt={vi.fn()} rows={[]} />);
    expect(screen.getByText("Run a benchmark to compare results.")).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
  });
});
