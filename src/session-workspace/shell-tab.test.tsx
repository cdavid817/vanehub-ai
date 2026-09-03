// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { SessionShellDescriptor } from "../types/session-workspace-shell-frames";
import { ShellTab } from "./shell-tab";

const { mockShellService } = vi.hoisted(() => ({
  mockShellService: {
    listSessionShells: vi.fn(),
    createSessionShell: vi.fn(),
    attachSessionShell: vi.fn(),
    detachSessionShell: vi.fn(),
    writeSessionShell: vi.fn(),
    resizeSessionShell: vi.fn(),
    renameSessionShell: vi.fn(),
    closeSessionShell: vi.fn(),
  },
}));

vi.mock("../services/runtime-session-shell-client", () => ({
  sessionShellService: mockShellService,
}));

// xterm needs a real canvas; the tab's behaviour under test is lifecycle, not rendering.
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    rows = 24;
    cols = 80;
    options: Record<string, unknown> = {};
    written: string[] = [];
    open() {}
    loadAddon() {}
    write(data: string) {
      this.written.push(data);
    }
    writeln(data: string) {
      this.written.push(`${data}\n`);
    }
    onData() {
      return { dispose() {} };
    }
    dispose() {}
  },
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
  },
}));
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

function descriptor(overrides: Partial<SessionShellDescriptor> = {}): SessionShellDescriptor {
  return {
    shellId: "shell-1",
    generation: 1,
    sessionId: "session-1",
    title: "Shell 1",
    runtime: {
      kind: "native",
      supportsResize: true,
      supportsReplay: true,
      supportsReconnect: false,
    },
    state: "running",
    createdAt: "2026-08-22T09:00:00Z",
    lastActivityAt: "2026-08-22T09:00:00Z",
    revision: 1,
    foregroundProcess: "unknown",
    ...overrides,
  };
}

const detach = vi.fn();

function attachment(overrides: Record<string, unknown> = {}) {
  return {
    attachmentId: "attach-1",
    descriptor: descriptor(),
    replay: [],
    nextSequence: 1,
    detach,
    ...overrides,
  };
}

beforeAll(async () => {
  await activateAppLanguage("zh-CN");
  // ResizeObserver is not implemented in jsdom, and the surface observes its host on mount.
  globalThis.ResizeObserver = class {
    observe() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
});

beforeEach(() => {
  vi.clearAllMocks();
  detach.mockResolvedValue(undefined);
  mockShellService.listSessionShells.mockResolvedValue([descriptor()]);
  mockShellService.attachSessionShell.mockResolvedValue(attachment());
  mockShellService.closeSessionShell.mockResolvedValue(undefined);
  mockShellService.renameSessionShell.mockResolvedValue(descriptor({ title: "构建", revision: 2 }));
  mockShellService.createSessionShell.mockResolvedValue(
    descriptor({ shellId: "shell-2", title: "Shell 2" }),
  );
});

describe("ShellTab", () => {
  it("detaches rather than closes when the tab is hidden", async () => {
    const { rerender } = renderWithAppProviders(
      <ShellTab isVisible sessionId="session-1" />,
    );
    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalled());

    rerender(<ShellTab isVisible={false} sessionId="session-1" />);

    // Glancing at another tab is not a request to end a build.
    await waitFor(() => expect(detach).toHaveBeenCalled());
    expect(mockShellService.closeSessionShell).not.toHaveBeenCalled();
  });

  it("detaches rather than closes when it unmounts", async () => {
    const { unmount } = renderWithAppProviders(<ShellTab isVisible sessionId="session-1" />);
    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalled());

    unmount();

    await waitFor(() => expect(detach).toHaveBeenCalled());
    expect(mockShellService.closeSessionShell).not.toHaveBeenCalled();
  });

  it("reattaches from the sequence it already consumed", async () => {
    mockShellService.attachSessionShell.mockResolvedValueOnce(
      attachment({
        descriptor: descriptor({ title: "已附加" }),
        replay: [
          {
            shellId: "shell-1",
            sequence: 7,
            occurredAt: "2026-08-22T09:00:01Z",
            stream: "pty" as const,
            data: "done\n",
          },
        ],
        nextSequence: 8,
      }),
    );
    const { rerender } = renderWithAppProviders(<ShellTab isVisible sessionId="session-1" />);
    // Waiting for the attach to have landed rather than merely to have been called: a view hidden
    // while the snapshot is still in flight has consumed nothing, and asking from 0 again is then
    // the correct behaviour rather than the bug this test is looking for.
    await screen.findByRole("tab", { name: /已附加/ });

    rerender(<ShellTab isVisible={false} sessionId="session-1" />);
    await waitFor(() => expect(detach).toHaveBeenCalled());
    rerender(<ShellTab isVisible sessionId="session-1" />);

    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalledTimes(2));
    // Asking from 0 again would replay output the terminal already holds; asking from a later
    // sequence would skip what happened while nobody was watching.
    expect(mockShellService.attachSessionShell.mock.calls[1][0]).toEqual({
      shellId: "shell-1",
      afterSequence: 7,
    });
  });

  it("opens no shell while the tab has never been shown", async () => {
    mockShellService.listSessionShells.mockResolvedValue([]);

    renderWithAppProviders(<ShellTab isVisible={false} sessionId="session-1" />);

    await waitFor(() => expect(mockShellService.listSessionShells).toHaveBeenCalled());
    expect(mockShellService.createSessionShell).not.toHaveBeenCalled();
  });

  it("confirms before it closes and says what it cannot know about running work", async () => {
    const user = userEvent.setup();
    renderWithAppProviders(<ShellTab isVisible sessionId="session-1" />);
    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalled());

    await user.click(screen.getByRole("button", { name: "关闭" }));

    // An opaque runtime reports `unknown`, and the dialog says so rather than claiming nothing is
    // running about a shell that might be midway through a deploy.
    expect(await screen.findByRole("dialog")).toBeDefined();
    expect(screen.getByRole("alert").textContent).toContain("无法报告");
    expect(mockShellService.closeSessionShell).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "关闭 Shell" }));
    await waitFor(() => expect(mockShellService.closeSessionShell).toHaveBeenCalledWith("shell-1"));
  });

  it("warns concretely when the runtime can see foreground work", async () => {
    const busy = descriptor({ foregroundProcess: "present" });
    mockShellService.listSessionShells.mockResolvedValue([busy]);
    // The attach snapshot is the authoritative descriptor: the registry re-reads the runtime's
    // foreground answer when a view claims the Shell.
    mockShellService.attachSessionShell.mockResolvedValue(attachment({ descriptor: busy }));
    const user = userEvent.setup();
    renderWithAppProviders(<ShellTab isVisible sessionId="session-1" />);
    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalled());

    await user.click(screen.getByRole("button", { name: "关闭" }));

    expect(screen.getByRole("alert").textContent).toContain("仍有命令在运行");
  });

  it("adds a second shell without disturbing the first", async () => {
    const user = userEvent.setup();
    renderWithAppProviders(<ShellTab isVisible sessionId="session-1" />);
    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalled());

    await user.click(screen.getByRole("button", { name: "新建 Shell" }));

    await waitFor(() => expect(screen.getAllByRole("tab")).toHaveLength(2));
    expect(mockShellService.closeSessionShell).not.toHaveBeenCalled();
    expect(mockShellService.createSessionShell.mock.calls[0][0].requestId).toBeTruthy();
  });

  it("renames through the registry rather than locally", async () => {
    const user = userEvent.setup();
    renderWithAppProviders(<ShellTab isVisible sessionId="session-1" />);
    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalled());

    await user.click(screen.getByRole("button", { name: "重命名" }));
    const input = screen.getByLabelText("Shell 名称");
    await user.clear(input);
    await user.type(input, "构建");
    await user.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() =>
      expect(mockShellService.renameSessionShell).toHaveBeenCalledWith({
        shellId: "shell-1",
        title: "构建",
      }),
    );
  });

  it("marks evicted output rather than presenting a shortened scrollback as continuous", async () => {
    mockShellService.attachSessionShell.mockResolvedValue(
      attachment({
        gap: { fromSequence: 1, toSequence: 40, reason: "shell_replay_evicted" },
        nextSequence: 41,
      }),
    );

    renderWithAppProviders(<ShellTab isVisible sessionId="session-1" />);

    await waitFor(() => expect(mockShellService.attachSessionShell).toHaveBeenCalled());
    // The surface asks from 0 and is told the registry no longer holds sequences 1-40, so the view
    // resumes from 40 rather than replaying output that is gone.
    await waitFor(() => expect(screen.getAllByRole("log")).toHaveLength(1));
  });
});
