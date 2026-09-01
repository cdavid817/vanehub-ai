// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import type { SessionSeat } from "../types/agent";
import { SeatAvatarGroup, type SeatAvatarGroupProps } from "./seat-avatar-group";

const claude: SessionSeat = { agentId: "claude-code", roleId: null, seatId: "seat-1" };
const codex: SessionSeat = { agentId: "codex-cli", roleId: null, seatId: "seat-2" };
const gemini: SessionSeat = { agentId: "gemini-cli", leftAt: "2026-01-01T00:00:00Z", roleId: null, seatId: "seat-3" };
const seats = [claude, codex];

function renderGroup(overrides: Partial<SeatAvatarGroupProps> = {}) {
  const onSelect = vi.fn();
  const utils = render(
    <I18nextProvider i18n={i18n}>
      <SeatAvatarGroup
        departedSeats={[]}
        onSelect={onSelect}
        roles={[]}
        seats={seats}
        selectedIndex={null}
        turnStatus={null}
        {...overrides}
      />
    </I18nextProvider>,
  );
  return { onSelect, ...utils };
}

describe("SeatAvatarGroup", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing for a single-seat session, matching the tab strip it replaces", () => {
    const { container } = renderGroup({ seats: [claude] });
    expect(container.firstChild).toBeNull();
  });

  it("shows the selected seat's name on the trigger without opening the popover", () => {
    renderGroup({ selectedIndex: 1 });
    expect(screen.getByRole("button", { name: /codex-cli/ })).toBeTruthy();
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("shows an All seats indicator on the trigger when nothing is selected", () => {
    renderGroup({ selectedIndex: null });
    expect(screen.getByRole("button", { name: "All seats" })).toBeTruthy();
  });

  it("opens on trigger click and closes on Escape, returning focus to the trigger", () => {
    renderGroup();
    const trigger = screen.getByRole("button", { name: "All seats" });
    fireEvent.click(trigger);
    expect(screen.getByRole("listbox")).toBeTruthy();

    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("closes on an outside pointerdown without changing the selection", () => {
    const { onSelect } = renderGroup();
    fireEvent.click(screen.getByRole("button", { name: "All seats" }));
    expect(screen.getByRole("listbox")).toBeTruthy();

    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("opens with roving focus on the first row and wraps at both ends", () => {
    renderGroup();
    fireEvent.click(screen.getByRole("button", { name: "All seats" }));
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(3); // All seats, claude-code, codex-cli
    expect(document.activeElement).toBe(options[0]);

    fireEvent.keyDown(screen.getByRole("listbox"), { key: "ArrowUp" });
    expect(document.activeElement).toBe(options[2]);

    fireEvent.keyDown(screen.getByRole("listbox"), { key: "ArrowDown" });
    expect(document.activeElement).toBe(options[0]);
  });

  it("selects the roving-focused seat with Enter and closes the popover", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderGroup();
    await user.click(screen.getByRole("button", { name: "All seats" }));
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(screen.getAllByRole("option")[1]);

    await user.keyboard("{Enter}");
    expect(onSelect).toHaveBeenCalledWith(0);
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("selects the roving-focused seat with Space and closes the popover", async () => {
    const user = userEvent.setup();
    const { onSelect } = renderGroup();
    await user.click(screen.getByRole("button", { name: "All seats" }));
    await user.keyboard("{ArrowDown}{ArrowDown}");
    expect(document.activeElement).toBe(screen.getAllByRole("option")[2]);

    await user.keyboard("{ }");
    expect(onSelect).toHaveBeenCalledWith(1);
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("renders a departed seat visibly-marked rather than hidden, and never selects it", () => {
    const { onSelect } = renderGroup({ departedSeats: [gemini] });
    fireEvent.click(screen.getByRole("button", { name: "All seats" }));

    const departedOption = screen.getByRole("option", { name: /gemini-cli/ });
    expect(departedOption.getAttribute("aria-disabled")).toBe("true");
    expect(within(departedOption).getByText(i18n.t("session.departed"))).toBeTruthy();

    fireEvent.click(departedOption);
    expect(onSelect).not.toHaveBeenCalled();
    expect(screen.getByRole("listbox")).toBeTruthy();
  });

  it("marks the current speaker from turnStatus and leaves a non-speaking seat unmarked", () => {
    const turnStatus: TurnStatus = { depth: 1, holderName: "codex-cli", kind: "agent", maxDepth: 6, seatId: "seat-2" };
    renderGroup({ turnStatus });
    fireEvent.click(screen.getByRole("button", { name: "All seats" }));

    const speakingOption = screen.getByRole("option", { name: /codex-cli/ });
    const idleOption = screen.getByRole("option", { name: /claude-code/ });
    expect(within(speakingOption).getByText(i18n.t("session.seatSpeaking"))).toBeTruthy();
    expect(within(idleOption).queryByText(i18n.t("session.seatSpeaking"))).toBeNull();
  });
});
