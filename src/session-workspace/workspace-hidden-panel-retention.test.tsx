// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    exportSessionLogs: vi.fn(),
    getSessionGitDiff: vi.fn(),
    getSessionGitStatus: vi.fn(),
    listSessionDirectory: vi.fn(),
    listSessionLogs: vi.fn(),
    readSessionFile: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));

const { ChangesTab } = await import("./changes-tab");
const { FilesTab } = await import("./files-tab");
const { LogsTab } = await import("./logs-tab");

function mount(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  const wrap = (element: ReactElement) => (
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>{element}</QueryClientProvider>
    </I18nextProvider>
  );
  const rendered = render(wrap(ui));
  return { rerenderPanel: (next: ReactElement) => rendered.rerender(wrap(next)) };
}

/** jest-dom matchers are not installed here, so the input value is read through the element. */
function searchValue(): string | null {
  const input = screen.getByLabelText("Search redacted logs");
  return input instanceof HTMLInputElement ? input.value : null;
}

describe("hidden mounted panels", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps what the user typed into the Logs filters", async () => {
    const user = userEvent.setup();
    mockAgentService.listSessionLogs.mockResolvedValue({
      items: [],
      nextCursor: null,
      truncated: false,
    });
    const { rerenderPanel } = mount(<LogsTab isVisible sessionId="session-1" />);
    await waitFor(() => expect(mockAgentService.listSessionLogs).toHaveBeenCalled());

    const search = screen.getByLabelText("Search redacted logs");
    await user.type(search, "timeout");
    await user.click(screen.getByRole("button", { name: "Debug" }));
    const readsBeforeHiding = mockAgentService.listSessionLogs.mock.calls.length;

    rerenderPanel(<LogsTab isVisible={false} sessionId="session-1" />);

    // A half-typed filter is work in progress. Losing it because the user glanced at another tab
    // is the failure this retention exists to prevent.
    expect(searchValue()).toBe("timeout");
    expect(screen.getByRole("button", { name: "Debug" }).getAttribute("aria-pressed")).toBe("false");

    rerenderPanel(<LogsTab isVisible sessionId="session-1" />);

    expect(searchValue()).toBe("timeout");
    expect(mockAgentService.listSessionLogs.mock.calls.length).toBeGreaterThanOrEqual(
      readsBeforeHiding,
    );
  });

  it("keeps the Files selection and its preview while the tree stops being read", async () => {
    const user = userEvent.setup();
    mockAgentService.listSessionDirectory.mockResolvedValue({
      items: [{ name: "main.rs", path: "main.rs", kind: "file" as const, size: 12 }],
      truncated: false,
      nextCursor: null,
      context: { availability: "available" as const, rootName: "project", reason: null },
      path: "",
    });
    mockAgentService.readSessionFile.mockResolvedValue({
      path: "main.rs",
      status: "text" as const,
      content: "fn main() {}",
    });
    const { rerenderPanel } = mount(<FilesTab isVisible sessionId="session-1" />);

    await waitFor(() => expect(screen.getByText("main.rs")).toBeTruthy());
    await user.click(screen.getByText("main.rs"));
    await waitFor(() => expect(screen.getByText("fn main() {}")).toBeTruthy());
    const readsBeforeHiding = mockAgentService.listSessionDirectory.mock.calls.length;

    rerenderPanel(<FilesTab isVisible={false} sessionId="session-1" />);

    expect(screen.getByText("fn main() {}")).toBeTruthy();
    expect(mockAgentService.listSessionDirectory.mock.calls.length).toBe(readsBeforeHiding);

    rerenderPanel(<FilesTab isVisible sessionId="session-1" />);

    // Selection, not just cache: the panel comes back to the file the user was reading.
    expect(screen.getByText("fn main() {}")).toBeTruthy();
  });

  it("keeps the Changes selection and diff source while the working tree stops being read", async () => {
    const user = userEvent.setup();
    mockAgentService.getSessionGitStatus.mockResolvedValue({
      isGit: true,
      branch: "main",
      truncated: false,
      items: [
        { path: "src/a.rs", indexStatus: "M", worktreeStatus: " ", previousPath: null },
        { path: "src/b.rs", indexStatus: " ", worktreeStatus: "M", previousPath: null },
      ],
    });
    mockAgentService.getSessionGitDiff.mockResolvedValue({ files: [], truncated: false });
    const { rerenderPanel } = mount(<ChangesTab isVisible sessionId="session-1" />);

    await waitFor(() => expect(screen.getByText("src/b.rs")).toBeTruthy());
    await user.click(screen.getByText("src/b.rs"));
    await user.click(screen.getByRole("button", { name: "Staged" }));
    const readsBeforeHiding = mockAgentService.getSessionGitStatus.mock.calls.length;

    rerenderPanel(<ChangesTab isVisible={false} sessionId="session-1" />);

    expect(screen.getAllByText("src/b.rs").length).toBeGreaterThan(0);
    expect(mockAgentService.getSessionGitStatus.mock.calls.length).toBe(readsBeforeHiding);

    rerenderPanel(<ChangesTab isVisible sessionId="session-1" />);
    await user.click(screen.getByText("src/a.rs"));

    // Asserted through the next request rather than through a CSS class: the diff for the newly
    // selected file is still asked for against the index, which is only true if the toggle the
    // user set before hiding survived.
    await waitFor(() =>
      expect(mockAgentService.getSessionGitDiff).toHaveBeenLastCalledWith(
        "session-1",
        "src/a.rs",
        "staged",
      ),
    );
  });
});
