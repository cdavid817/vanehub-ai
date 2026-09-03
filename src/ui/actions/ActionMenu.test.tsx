// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { ActionMenu, type ActionMenuItem } from "./ActionMenu";

describe("ActionMenu", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("opens on trigger click and closes on Escape, returning focus to the trigger", () => {
    render(<ActionMenu items={[{ id: "rename", label: "Rename", onSelect: vi.fn() }]} triggerLabel="More actions" />);
    const trigger = screen.getByRole("button", { name: "More actions" });
    fireEvent.click(trigger);
    expect(screen.getByRole("menu")).toBeTruthy();

    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("activates a plain action directly, with no confirmation step", () => {
    const onSelect = vi.fn();
    render(<ActionMenu items={[{ id: "rename", label: "Rename", onSelect }]} triggerLabel="More actions" />);
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Rename" }));
    expect(onSelect).toHaveBeenCalledOnce();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("blocks a destructive action behind confirmation, and does nothing if cancelled", async () => {
    const onSelect = vi.fn();
    const items: ActionMenuItem[] = [{
      id: "delete",
      label: "Delete session",
      onSelect,
      tone: "destructive",
      confirmation: { title: "Delete this session?", description: "This cannot be undone." },
    }];
    render(<ActionMenu items={items} triggerLabel="More actions" />);
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete session" }));

    expect(await screen.findByRole("dialog")).toBeTruthy();
    expect(screen.getByText("This cannot be undone.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("runs the destructive action once the user confirms", async () => {
    const onSelect = vi.fn();
    const items: ActionMenuItem[] = [{
      id: "delete",
      label: "Delete session",
      onSelect,
      tone: "destructive",
      confirmation: { title: "Delete this session?" },
    }];
    render(<ActionMenu items={items} triggerLabel="More actions" />);
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete session" }));

    fireEvent.click(await screen.findByRole("button", { name: "Confirm" }));
    await waitFor(() => expect(onSelect).toHaveBeenCalledOnce());
  });

  it("moves real DOM focus through items with arrow keys, wrapping at both ends, and jumps via Home/End", () => {
    const items: ActionMenuItem[] = [
      { id: "rename", label: "Rename", onSelect: vi.fn() },
      { id: "duplicate", label: "Duplicate", onSelect: vi.fn() },
      { id: "archive", label: "Archive", onSelect: vi.fn() },
    ];
    render(<ActionMenu items={items} triggerLabel="More actions" />);
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    const menu = screen.getByRole("menu");
    const [rename, , archive] = screen.getAllByRole("menuitem");
    expect(document.activeElement).toBe(rename);

    fireEvent.keyDown(menu, { key: "ArrowUp" });
    expect(document.activeElement).toBe(archive);

    fireEvent.keyDown(menu, { key: "ArrowDown" });
    expect(document.activeElement).toBe(rename);

    fireEvent.keyDown(menu, { key: "End" });
    expect(document.activeElement).toBe(archive);

    fireEvent.keyDown(menu, { key: "Home" });
    expect(document.activeElement).toBe(rename);
  });

  it("keeps a disabled item keyboard-reachable and explains why it cannot run", () => {
    const onSelect = vi.fn();
    const items: ActionMenuItem[] = [
      { id: "rename", label: "Rename", onSelect: vi.fn() },
      { id: "archive", label: "Archive", onSelect, disabled: true, disabledReason: "Session is still running." },
    ];
    render(<ActionMenu items={items} triggerLabel="More actions" />);
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.keyDown(screen.getByRole("menu"), { key: "ArrowDown" });

    const archiveItem = screen.getByRole("menuitem", { name: /Archive/ });
    expect(archiveItem.getAttribute("aria-disabled")).toBe("true");
    expect(screen.getByText("Session is still running.")).toBeTruthy();
    fireEvent.click(archiveItem);
    expect(onSelect).not.toHaveBeenCalled();
  });
});
