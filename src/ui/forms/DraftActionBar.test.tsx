// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { DraftActionBar } from "./DraftActionBar";

describe("DraftActionBar", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing when there are no unsaved changes", () => {
    const { container } = render(<DraftActionBar dirtyCount={0} onDiscard={vi.fn()} onSave={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it("shows a localized, correctly pluralized unsaved-change count", () => {
    const single = render(<DraftActionBar dirtyCount={1} onDiscard={vi.fn()} onSave={vi.fn()} />);
    expect(single.getByRole("region").textContent).toContain("1 unsaved change");
    single.unmount();

    render(<DraftActionBar dirtyCount={3} onDiscard={vi.fn()} onSave={vi.fn()} />);
    expect(screen.getByRole("region").textContent).toContain("3 unsaved changes");
  });

  it("calls onSave and onDiscard from their respective actions", () => {
    const onSave = vi.fn();
    const onDiscard = vi.fn();
    render(<DraftActionBar dirtyCount={2} onDiscard={onDiscard} onSave={onSave} />);
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSave).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(onDiscard).toHaveBeenCalledOnce();
  });

  it("disables both actions while a save is pending and shows the pending state", () => {
    render(<DraftActionBar dirtyCount={2} onDiscard={vi.fn()} onSave={vi.fn()} pending />);
    expect((screen.getByRole("button", { name: /Save/ }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Discard" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("disables only Save, not Discard, when saveDisabled is set", () => {
    render(<DraftActionBar dirtyCount={2} onDiscard={vi.fn()} onSave={vi.fn()} saveDisabled />);
    expect((screen.getByRole("button", { name: /Save/ }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Discard" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("shows a save error message", () => {
    render(
      <DraftActionBar
        dirtyCount={2}
        error={{ kind: "error", message: "Could not save your changes.", retryable: true }}
        onDiscard={vi.fn()}
        onSave={vi.fn()}
      />,
    );
    expect(screen.getByText("Could not save your changes.")).toBeTruthy();
  });
});
