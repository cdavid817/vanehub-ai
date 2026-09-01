// @vitest-environment jsdom

import { act, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { useDraftNavigationGuard, type DraftNavigationOutcome } from "./use-draft-navigation-guard";

function Harness({ onOutcome }: { onOutcome: (outcome: DraftNavigationOutcome) => void }) {
  const { requestDecision, navigationGuardDialog } = useDraftNavigationGuard();
  return (
    <div>
      <button
        onClick={() => {
          void requestDecision({ canSave: true, dirtyCount: 2, title: "Unsaved changes" }).then(onOutcome);
        }}
        type="button"
      >
        trigger
      </button>
      {navigationGuardDialog}
    </div>
  );
}

describe("useDraftNavigationGuard", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing until a decision is requested", () => {
    render(<Harness onOutcome={vi.fn()} />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("resolves stay, discard, and save from their respective buttons", async () => {
    const onOutcome = vi.fn();
    render(<Harness onOutcome={onOutcome} />);

    await act(async () => screen.getByRole("button", { name: "trigger" }).click());
    await act(async () => screen.getByRole("button", { name: "Stay" }).click());
    expect(onOutcome).toHaveBeenLastCalledWith("stay");

    await act(async () => screen.getByRole("button", { name: "trigger" }).click());
    await act(async () => screen.getByRole("button", { name: "Discard changes" }).click());
    expect(onOutcome).toHaveBeenLastCalledWith("discard");

    await act(async () => screen.getByRole("button", { name: "trigger" }).click());
    await act(async () => screen.getByRole("button", { name: "Save & leave" }).click());
    expect(onOutcome).toHaveBeenLastCalledWith("save");
  });

  it("disables Save without hiding it when canSave is false", async () => {
    function Disabled() {
      const { requestDecision, navigationGuardDialog } = useDraftNavigationGuard();
      return (
        <div>
          <button onClick={() => void requestDecision({ canSave: false, dirtyCount: 1, title: "Unsaved changes" })} type="button">
            trigger
          </button>
          {navigationGuardDialog}
        </div>
      );
    }
    render(<Disabled />);
    await act(async () => screen.getByRole("button", { name: "trigger" }).click());
    expect((screen.getByRole("button", { name: "Save & leave" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Discard changes" }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("falls back to the pluralized dirty-count description when none is given explicitly", async () => {
    render(<Harness onOutcome={vi.fn()} />);
    await act(async () => screen.getByRole("button", { name: "trigger" }).click());
    expect(screen.getByText("Leaving now will discard 2 unsaved changes.")).toBeTruthy();
  });

  it("resolves a superseded request as stay rather than stranding its promise", async () => {
    function DoubleRequest() {
      const { requestDecision, navigationGuardDialog } = useDraftNavigationGuard();
      return (
        <div>
          <button
            onClick={() => {
              void requestDecision({ canSave: true, dirtyCount: 1, title: "First" });
              void requestDecision({ canSave: true, dirtyCount: 1, title: "Second" });
            }}
            type="button"
          >
            trigger
          </button>
          {navigationGuardDialog}
        </div>
      );
    }
    render(<DoubleRequest />);
    await act(async () => screen.getByRole("button", { name: "trigger" }).click());
    // Only the second request's dialog is left open; the first was silently resolved "stay".
    expect(screen.getByRole("heading", { name: "Second" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "First" })).toBeNull();
  });
});
