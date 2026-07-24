// @vitest-environment jsdom
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RemoteTerminalCapturePanel, RemoteTerminalHistoryPanel, RemoteTerminalPurgeButton, RemoteTerminalSearchPanel, RemoteTerminalTemplatePanel, RemoteTerminalTrustPanel } from "./remote-terminal-panels";

describe("remote terminal panels", () => {
  it("exposes accessible trust, template, search, capture and purge actions", () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    const handler = vi.fn();
    render(<><RemoteTerminalTrustPanel required onConfirm={handler} /><RemoteTerminalTemplatePanel names={["Build"]} onInsert={handler} onRun={handler} /><RemoteTerminalSearchPanel query="" onChange={handler} /><RemoteTerminalCapturePanel enabled onToggle={handler} /><RemoteTerminalHistoryPanel count={1} page={0} onNext={handler} /><RemoteTerminalPurgeButton onPurge={handler} /></>);
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    fireEvent.click(screen.getByRole("button", { name: "Insert" }));
    fireEvent.click(screen.getByRole("button", { name: "Run" }));
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "error" } });
    fireEvent.click(screen.getByRole("button", { name: /Capture/ }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Purge capture" }));
    expect(handler).toHaveBeenCalled();
    expect(confirm).toHaveBeenCalled();
    confirm.mockRestore();
  });
});
