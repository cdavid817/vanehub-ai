/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import { sessionShellService } from "../services/runtime-session-shell-client";
import { FilesToolbar } from "./files-toolbar";


let reveal: ReturnType<typeof vi.spyOn>;
let createShell: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  reveal = vi
    .spyOn(agentService, "openSessionFolder")
    .mockResolvedValue({ status: "opened", openerId: "file-explorer", reason: null });
  createShell = vi.spyOn(sessionShellService, "createSessionShell").mockResolvedValue({
    shellId: "shell-1",
    sessionId: "session-1",
    state: "running",
    title: "Shell 1",
    revision: 1,
    createdAt: "2026-08-26T10:00:00Z",
    updatedAt: "2026-08-26T10:00:00Z",
  } as never);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function render(overrides: Partial<Parameters<typeof FilesToolbar>[0]> = {}) {
  return renderWithAppProviders(
    <FilesToolbar
      isRemote={false}
      onContentSearch={vi.fn()}
      onQuickOpen={vi.fn()}
      onShellOpened={vi.fn()}
      selectedPath={null}
      sessionId="session-1"
      {...overrides}
    />,
  );
}

function button(pattern: RegExp) {
  return screen.getByRole("button", { name: pattern });
}

describe("FilesToolbar", () => {
  it("opens the two search surfaces", () => {
    const onQuickOpen = vi.fn();
    const onContentSearch = vi.fn();
    render({ onContentSearch, onQuickOpen });

    fireEvent.click(button(/Quick Open|快速打开/));
    fireEvent.click(button(/Search in files|在文件中搜索/));

    expect(onQuickOpen).toHaveBeenCalled();
    expect(onContentSearch).toHaveBeenCalled();
  });

  it("cannot copy a path before anything is selected", () => {
    render({ selectedPath: null });

    // Disabled rather than absent: the action exists, it simply has nothing to copy yet, and a
    // control that appeared only after a selection would look like it had been added.
    expect(button(/Copy path|复制路径/)).toHaveProperty("disabled", true);
  });

  it("reveals the selection's directory rather than the file", async () => {
    render({ selectedPath: "src/main.rs" });

    fireEvent.click(button(/Reveal|文件管理器/));

    // A file manager opens directories. Handing it the file would either fail or open an editor,
    // neither of which is what "reveal" means.
    await waitFor(() =>
      expect(reveal).toHaveBeenCalledWith("session-1", "file-explorer", "src"),
    );
  });

  it("cannot reveal a workspace that is on another machine", () => {
    render({ isRemote: true, selectedPath: "src/main.rs" });

    // Visibly unavailable rather than hidden: a control that vanishes makes a reader think they
    // misremembered where it was.
    expect(button(/Reveal|文件管理器/)).toHaveProperty("disabled", true);
  });

  it("opens a Shell in the selection's directory", async () => {
    render({ selectedPath: "src/main.rs" });

    fireEvent.click(button(/Open Shell|打开 Shell/));

    await waitFor(() =>
      expect(createShell).toHaveBeenCalledWith(
        expect.objectContaining({ workingDirectory: "src", sessionId: "session-1" }),
      ),
    );
    // A request id, so this is an explicit "open another one" rather than a claim on the session's
    // default Shell — which starts at the root and is not what was asked for.
    expect(createShell.mock.calls[0]?.[0]).toHaveProperty("requestId");
  });

  it("opens a Shell at the root when nothing is selected", async () => {
    render({ selectedPath: null });

    fireEvent.click(button(/Open Shell|打开 Shell/));

    await waitFor(() =>
      expect(createShell).toHaveBeenCalledWith(expect.objectContaining({ workingDirectory: "" })),
    );
  });
});
