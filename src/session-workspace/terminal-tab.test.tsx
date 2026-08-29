// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Fragment, type ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import type { ChatMessage } from "../types/chat";
import { WorkspaceEvidenceScopeProvider } from "./workspace-evidence-scope";

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    listExecutionRecords: vi.fn(),
    getExecutionRecord: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));
// The repo's convention for jsdom: a measured virtualizer renders nothing when every element
// reports a zero height. The bound it actually enforces is asserted separately, against a real
// virtualizer with measurements shimmed in.
vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => unknown; testId?: string }) => (
    <div data-testid={testId}>
      {items.map((item, index) => (
        <Fragment key={index}>{renderItem(item, index) as never}</Fragment>
      ))}
    </div>
  ),
}));

const { TerminalTab } = await import("./terminal-tab");

const sessionId = evidenceSessionIdSchema.parse("session-1");

function message(id: string, seatId: string | undefined, toolName: string): ChatMessage {
  return {
    id,
    sessionId: "session-1",
    role: "assistant",
    speakerSeatId: seatId,
    content: "",
    status: "completed",
    toolUse: [{ id: `${id}-tool`, name: toolName, status: "completed" }],
    createdAt: "2026-08-22T10:00:00.000Z",
    updatedAt: "2026-08-22T10:00:00.000Z",
    sessionSequence: 1,
    executionRunId: null,
  };
}

const messages = [
  message("m1", "seat-planner", "planner_tool"),
  message("m2", "seat-builder", "builder_tool"),
  message("m3", undefined, "unattributed_tool"),
];

function mount(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  return render(
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>
        <WorkspaceEvidenceScopeProvider seatIds={[]} sessionId={sessionId}>
          {ui}
        </WorkspaceEvidenceScopeProvider>
      </QueryClientProvider>
    </I18nextProvider>,
  );
}

async function openLegacy(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByTestId("execution-record-view-legacy"));
  await waitFor(() => expect(screen.getByTestId("legacy-source-notice")).toBeTruthy());
}

describe("TerminalTab", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockAgentService.listExecutionRecords.mockResolvedValue({
      items: [],
      coverage: { state: "complete", reasonCodes: [], truncated: false },
    });
  });

  it("shows every seat's activity when no seat is selected", async () => {
    const user = userEvent.setup();
    mount(<TerminalTab messages={messages} partial={false} sessionId="session-1" />);
    await openLegacy(user);

    for (const name of ["planner_tool", "builder_tool", "unattributed_tool"]) {
      expect(screen.getByText(name)).toBeDefined();
    }
  });

  it("shows only the selected seat's activity", async () => {
    const user = userEvent.setup();
    mount(
      <TerminalTab messages={messages} partial={false} seatId="seat-builder" sessionId="session-1" />,
    );
    await openLegacy(user);

    expect(screen.getByText("builder_tool")).toBeDefined();
    expect(screen.queryByText("planner_tool")).toBeNull();
  });

  // Attributing an unlabelled message to whichever seat is selected would invent evidence.
  it("does not attribute unattributed activity to the selected seat", async () => {
    const user = userEvent.setup();
    mount(
      <TerminalTab messages={messages} partial={false} seatId="seat-builder" sessionId="session-1" />,
    );
    await openLegacy(user);

    expect(screen.queryByText("unattributed_tool")).toBeNull();
  });

  it("says where legacy activity came from every time it shows it", async () => {
    const user = userEvent.setup();
    mount(<TerminalTab messages={messages} partial sessionId="session-1" />);
    await openLegacy(user);

    const notice = screen.getByTestId("legacy-source-notice");
    // The rows are rendered by the same list as native records, so without this the reader has no
    // way to tell an observation from an assistant's account of one.
    expect(notice.textContent).toContain("not from recorded evidence");
    expect(notice.textContent).toContain("loaded message window");
    expect(screen.getAllByTestId("execution-record-legacy-source").length).toBeGreaterThan(0);
  });

  it("asks the record query only for the kinds the chosen view owns", async () => {
    const user = userEvent.setup();
    mount(<TerminalTab messages={[]} partial={false} sessionId="session-1" />);
    await waitFor(() => expect(mockAgentService.listExecutionRecords).toHaveBeenCalled());
    expect(mockAgentService.listExecutionRecords.mock.calls[0][0].filters.kinds).toEqual([
      "command",
      "tool",
      "delegation",
      "verification",
    ]);

    await user.click(screen.getByTestId("execution-record-view-commands"));

    await waitFor(() => {
      const last = mockAgentService.listExecutionRecords.mock.calls.at(-1);
      expect(last?.[0].filters.kinds).toEqual(["command"]);
    });
  });

  it("reads nothing from the record query while showing legacy activity", async () => {
    const user = userEvent.setup();
    mount(<TerminalTab messages={messages} partial={false} sessionId="session-1" />);
    await waitFor(() => expect(mockAgentService.listExecutionRecords).toHaveBeenCalled());
    const before = mockAgentService.listExecutionRecords.mock.calls.length;

    await openLegacy(user);

    // Legacy activity is projected from messages the caller already holds; a query here would be
    // asking the journal for rows it was never given.
    expect(mockAgentService.listExecutionRecords.mock.calls.length).toBe(before);
  });

  it("opens a record's details and closes them again", async () => {
    const user = userEvent.setup();
    mount(<TerminalTab messages={messages} partial={false} sessionId="session-1" />);
    await openLegacy(user);

    await user.click(screen.getAllByTestId("execution-record-row")[0]);
    const drawer = screen.getByTestId("execution-record-detail");
    expect(within(drawer).getByText("unattributed_tool")).toBeTruthy();
    // Legacy rows carry no correlation, so there is no destination to offer and no button.
    expect(screen.queryByTestId("execution-record-actions")).toBeNull();

    await user.click(screen.getByRole("button", { name: "Close record details" }));
    expect(screen.queryByTestId("execution-record-detail")).toBeNull();
  });

  it("renders unavailable rather than an empty list without a session", () => {
    mount(<TerminalTab messages={messages} partial={false} />);
    expect(screen.queryByTestId("execution-record-view-all")).toBeNull();
  });
});
