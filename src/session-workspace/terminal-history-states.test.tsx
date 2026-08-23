// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { EvidenceCoverageState, QueryCoverage } from "../types/session-workspace-evidence";
import {
  CoverageNotice,
  emptyStateFor,
  LegacySourceNotice,
  TerminalHistoryEmpty,
} from "./terminal-history-states";

function coverage(state: EvidenceCoverageState, truncated = false): QueryCoverage {
  return { state, reasonCodes: [], truncated };
}

function show(node: Parameters<typeof render>[0]) {
  return render(<I18nextProvider i18n={i18n}>{node}</I18nextProvider>);
}

describe("terminal history empty states", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("never calls a partial result a definitive empty", () => {
    // Under partial coverage the list has not seen everything, so "nothing happened" and "your
    // filter matched nothing" are both claims about rows it never read.
    expect(emptyStateFor({ coverage: coverage("partial"), filtered: false, hasError: false, loading: false })).toBe("partial");
    expect(emptyStateFor({ coverage: coverage("partial"), filtered: true, hasError: false, loading: false })).toBe("partial");
  });

  it("never calls an indexing result no activity", () => {
    expect(emptyStateFor({ coverage: coverage("indexing"), filtered: false, hasError: false, loading: false })).toBe("indexing");
    expect(emptyStateFor({ coverage: coverage("indexing"), filtered: true, hasError: false, loading: false })).toBe("indexing");
  });

  it("never calls an unavailable result zero", () => {
    expect(emptyStateFor({ coverage: coverage("unavailable"), filtered: false, hasError: false, loading: false })).toBe("unavailable");
    expect(emptyStateFor({ coverage: null, filtered: false, hasError: true, loading: false })).toBe("unavailable");
    expect(emptyStateFor({ coverage: null, filtered: false, hasError: false, loading: false })).toBe("unavailable");
  });

  it("distinguishes nothing happened from nothing matched, but only under complete coverage", () => {
    expect(emptyStateFor({ coverage: coverage("complete"), filtered: false, hasError: false, loading: false })).toBe("complete-empty");
    expect(emptyStateFor({ coverage: coverage("complete"), filtered: true, hasError: false, loading: false })).toBe("no-filter-match");
  });

  it("reports loading before it reports an answer", () => {
    expect(emptyStateFor({ coverage: null, filtered: false, hasError: false, loading: true })).toBe("loading");
    expect(emptyStateFor({ coverage: coverage("complete"), filtered: false, hasError: false, loading: true })).toBe("loading");
  });

  it("gives each state its own sentence", () => {
    const seen = new Set<string>();
    for (const state of ["complete-empty", "no-filter-match", "partial", "indexing", "unavailable"] as const) {
      const view = show(<TerminalHistoryEmpty state={state} />);
      const text = view.container.textContent ?? "";
      expect(text.length, state).toBeGreaterThan(0);
      // A shared empty state would let a reader act on "nothing happened" when the truth was one
      // of the other four.
      expect(seen.has(text), state).toBe(false);
      seen.add(text);
      view.unmount();
    }
  });
});

describe("coverage notices", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("says nothing when the answer is complete", () => {
    const view = show(<CoverageNotice coverage={coverage("complete")} />);
    expect(view.container.textContent).toBe("");
  });

  it("states partial and indexing coverage over rows that are on screen", () => {
    show(<CoverageNotice coverage={coverage("partial")} />);
    expect(screen.getByTestId("execution-records-coverage-partial").textContent).toContain(
      "not the whole record",
    );
  });

  it("names how many observations were dropped", () => {
    show(<CoverageNotice coverage={{ ...coverage("partial"), droppedCount: 4 }} />);
    expect(screen.getByTestId("execution-records-coverage-partial").textContent).toContain("4");
  });

  it("says where legacy activity came from and whether the window was short", () => {
    const view = show(<LegacySourceNotice coverage={coverage("partial")} />);
    expect(screen.getByTestId("legacy-source-notice").textContent).toContain(
      "not from recorded evidence",
    );
    expect(screen.getByTestId("legacy-source-notice").textContent).not.toContain(
      "loaded message window",
    );
    view.unmount();

    show(<LegacySourceNotice coverage={coverage("partial", true)} />);
    expect(screen.getByTestId("legacy-source-notice").textContent).toContain(
      "loaded message window",
    );
  });
});
