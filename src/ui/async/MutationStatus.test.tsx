// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { MutationStatus } from "./MutationStatus";

describe("MutationStatus", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing for an untracked target", () => {
    const { container } = render(<MutationStatus state={undefined} />);
    expect(container.firstChild).toBeNull();
  });

  it("shows a pending indicator without disabling anything outside itself", () => {
    render(<MutationStatus state={{ targetKey: "run-1", pending: true }} />);
    expect(screen.getByRole("status").textContent).toContain("Saving");
  });

  it("shows the error message with a working retry action when retryable", () => {
    const retry = vi.fn();
    render(
      <MutationStatus
        onRetry={retry}
        state={{ targetKey: "run-1", pending: false, error: { kind: "error", message: "Update failed.", retryable: true } }}
      />,
    );
    expect(screen.getByRole("alert").textContent).toContain("Update failed.");
    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledOnce();
  });

  it("hides retry for a non-retryable error but still offers dismiss", () => {
    const dismiss = vi.fn();
    render(
      <MutationStatus
        onDismiss={dismiss}
        state={{ targetKey: "run-1", pending: false, error: { kind: "error", message: "Rejected.", retryable: false } }}
      />,
    );
    expect(screen.queryByRole("button", { name: "Retry" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(dismiss).toHaveBeenCalledOnce();
  });
});
