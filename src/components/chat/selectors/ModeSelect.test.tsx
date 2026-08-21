// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { ModeSelect } from "./ModeSelect";

describe("OnePiece mode selector", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("distinguishes read-only Plan from write-capable Agent with persistent text", () => {
    const change = vi.fn();
    const close = vi.fn();
    render(<ModeSelect availableModes={["plan", "execute"]} emphasizeCapabilities onChange={change} onClose={close} onOpen={vi.fn()} open value="plan" />);
    const trigger = screen.getByTitle(/Plan · Read-only/);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(trigger.className).toContain("focus-visible:ring-2");
    expect(screen.getAllByText("Plan · Read-only").length).toBeGreaterThan(0);
    expect(screen.getByText(/without making changes/)).toBeTruthy();
    expect(screen.getByText("Agent · Can edit")).toBeTruthy();
    expect(screen.getByText(/edit files/i)).toBeTruthy();
    fireEvent.click(screen.getByText("Agent · Can edit"));
    expect(change).toHaveBeenCalledWith("execute");
    fireEvent.keyDown(document, { key: "Escape" });
    expect(close).toHaveBeenCalled();
  });
});
