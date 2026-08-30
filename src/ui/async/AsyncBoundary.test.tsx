// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { AsyncBoundary } from "./AsyncBoundary";
import type { AsyncViewState } from "./async-view-state";

function state(overrides: Partial<AsyncViewState<string[]>>): AsyncViewState<string[]> {
  return { initialLoading: false, refreshing: false, stale: false, ...overrides };
}

describe("AsyncBoundary", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows a loading state before any data has arrived", () => {
    render(
      <AsyncBoundary state={state({ initialLoading: true })}>
        {() => <p>content</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByRole("status").textContent).toContain("Loading");
    expect(screen.queryByText("content")).toBeNull();
  });

  it("shows a retryable error with a working retry action", () => {
    const retry = vi.fn();
    render(
      <AsyncBoundary onRetry={retry} state={state({ error: { kind: "error", message: "Could not load runs.", retryable: true } })}>
        {() => <p>content</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByRole("alert").textContent).toContain("Could not load runs.");
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledOnce();
  });

  it("hides the retry action when the error is not retryable", () => {
    render(
      <AsyncBoundary onRetry={vi.fn()} state={state({ error: { kind: "error", message: "Permanently rejected.", retryable: false } })}>
        {() => <p>content</p>}
      </AsyncBoundary>,
    );
    expect(screen.queryByRole("button", { name: "Retry" })).toBeNull();
  });

  it("renders the unavailable empty state for an unavailable target", () => {
    render(
      <AsyncBoundary state={state({ error: { kind: "unavailable", message: "gone", retryable: false } })}>
        {() => <p>content</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByText("Not available")).toBeTruthy();
  });

  it("renders the restricted empty state for a permission-denied target", () => {
    render(
      <AsyncBoundary state={state({ error: { kind: "restricted", message: "denied", retryable: false } })}>
        {() => <p>content</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByText("Restricted")).toBeTruthy();
  });

  it("renders the configured empty state when isEmpty matches, and content otherwise", () => {
    const view = render(
      <AsyncBoundary
        emptyState={{ title: "No runs yet" }}
        isEmpty={(data) => data.length === 0}
        state={state({ data: [] })}
      >
        {(data) => <p>{data.length} items</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByText("No runs yet")).toBeTruthy();
    view.rerender(
      <AsyncBoundary emptyState={{ title: "No runs yet" }} isEmpty={(data) => data.length === 0} state={state({ data: ["a"] })}>
        {(data) => <p>{data.length} items</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByText("1 items")).toBeTruthy();
  });

  it("distinguishes filtered-empty from no-data using the filtered flag", () => {
    render(
      <AsyncBoundary
        emptyState={{ title: "No runs yet" }}
        filtered
        filteredEmptyState={{ title: "No runs match your filters" }}
        isEmpty={(data) => data.length === 0}
        state={state({ data: [] })}
      >
        {() => <p>content</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByText("No runs match your filters")).toBeTruthy();
    expect(screen.queryByText("No runs yet")).toBeNull();
  });

  it("shows a non-blocking refresh indicator while background-refreshing without hiding content", () => {
    render(
      <AsyncBoundary state={state({ data: ["a"], refreshing: true })}>
        {() => <p>content</p>}
      </AsyncBoundary>,
    );
    expect(screen.getByText("content")).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("Refreshing");
  });
});
