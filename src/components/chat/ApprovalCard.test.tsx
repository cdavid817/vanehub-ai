// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { activateAppLanguage } from "../../i18n";
import { ApprovalCard } from "./ApprovalCard";
import { permissionsService } from "../../services/runtime-permissions-client";
import type { ApprovalResolutionOutcome, PendingApprovalEntry } from "../../types/permissions";

vi.mock("../../services/runtime-permissions-client", () => ({
  permissionsService: {
    listPendingApprovals: vi.fn(),
    resolvePendingApproval: vi.fn(),
  },
}));

const listPendingApprovals = vi.mocked(permissionsService.listPendingApprovals);
const resolvePendingApproval = vi.mocked(permissionsService.resolvePendingApproval);

const pending: PendingApprovalEntry = {
  id: "approval-1",
  agentId: "agent-1",
  sessionId: "session-1",
  callId: "call-1",
  action: "file.write",
  resource: "src/lib.rs",
  riskLevel: "L2",
  createdAt: "0",
};

/** Renders the card and waits for its mount-time pull to settle. */
async function renderCard() {
  render(<ApprovalCard callId="call-1" sessionId="session-1" />);
  await screen.findByText("file.write");
}

function approveButton(): HTMLButtonElement {
  return screen.getByRole("button", { name: /approve/i }) as HTMLButtonElement;
}

function denyButton(): HTMLButtonElement {
  return screen.getByRole("button", { name: /deny/i }) as HTMLButtonElement;
}

function outcome(): string | null {
  return screen.queryByTestId("approval-outcome")?.getAttribute("data-outcome") ?? null;
}

beforeEach(async () => {
  vi.clearAllMocks();
  listPendingApprovals.mockResolvedValue([pending]);
  // The default language follows the host locale, so an assertion on English text would fail on a
  // machine set to anything else.
  await activateAppLanguage("en");
});

afterEach(async () => {
  await activateAppLanguage("zh-CN");
});

describe("ApprovalCard", () => {
  it("submits one decision for a double click", async () => {
    const user = userEvent.setup();
    // Held open so the second click lands while the first is still in flight, which is the
    // interleaving a real double click produces.
    let release: (value: ApprovalResolutionOutcome) => void = () => {};
    resolvePendingApproval.mockReturnValue(
      new Promise<ApprovalResolutionOutcome>((resolve) => {
        release = resolve;
      }),
    );
    await renderCard();

    await user.click(approveButton());
    await user.click(approveButton());
    release("delivered");

    await waitFor(() => expect(outcome()).toBe("delivered"));
    expect(resolvePendingApproval).toHaveBeenCalledTimes(1);
  });

  it("keeps the controls closed once the decision is durable but undelivered", async () => {
    const user = userEvent.setup();
    resolvePendingApproval.mockResolvedValue("delivery_failed");
    await renderCard();

    await user.click(approveButton());

    await waitFor(() => expect(outcome()).toBe("delivery_failed"));
    // The decision exists. Offering the buttons back would invite a second one for a request that
    // already has an answer.
    expect(approveButton().disabled).toBe(true);
    expect(denyButton().disabled).toBe(true);
    expect(screen.getByText(/did not reach the agent/i)).toBeTruthy();
  });

  it("says nothing ran when the request had already ended", async () => {
    const user = userEvent.setup();
    resolvePendingApproval.mockResolvedValue("stale");
    await renderCard();

    await user.click(denyButton());

    await waitFor(() => expect(outcome()).toBe("stale"));
    expect(screen.getByText(/recorded but not applied/i)).toBeTruthy();
    expect(denyButton().disabled).toBe(true);
  });

  it("reconciles an ambiguous response by pulling the pending list", async () => {
    const user = userEvent.setup();
    // A dropped response: the decision may or may not have committed, and only the list can say.
    resolvePendingApproval.mockRejectedValue(new Error("transport closed"));
    listPendingApprovals.mockResolvedValueOnce([pending]).mockResolvedValueOnce([]);
    await renderCard();

    await user.click(approveButton());

    await waitFor(() => expect(outcome()).toBe("already_resolved"));
    expect(approveButton().disabled).toBe(true);
  });

  it("reopens the controls when reconciliation shows the request is still waiting", async () => {
    const user = userEvent.setup();
    resolvePendingApproval.mockRejectedValueOnce(new Error("transport closed"));
    listPendingApprovals.mockResolvedValue([pending]);
    await renderCard();

    await user.click(approveButton());

    // Still in the list, so nobody has answered it — the user must be able to try again.
    await waitFor(() => expect(outcome()).toBe("resolving"));
    expect(approveButton().disabled).toBe(false);

    resolvePendingApproval.mockResolvedValue("delivered");
    await user.click(approveButton());
    await waitFor(() => expect(outcome()).toBe("delivered"));
  });

  it("treats an outcome it cannot name as unresolved rather than as success", async () => {
    const user = userEvent.setup();
    // A native build newer than this frontend. Rendering it as delivered would tell the user a tool
    // ran when nothing here knows that.
    resolvePendingApproval.mockResolvedValue("unknown");
    listPendingApprovals.mockResolvedValue([pending]);
    await renderCard();

    await user.click(approveButton());

    await waitFor(() => expect(outcome()).toBe("resolving"));
    expect(screen.queryByText(/has resumed/i)).toBeNull();
  });

  it("sends the selected remembered scope and then locks the scope controls", async () => {
    const user = userEvent.setup();
    resolvePendingApproval.mockResolvedValue("delivered");
    await renderCard();

    await user.click(screen.getByRole("button", { name: "This project" }));
    await user.click(approveButton());

    await waitFor(() => expect(outcome()).toBe("delivered"));
    expect(resolvePendingApproval).toHaveBeenCalledWith("approval-1", true, "project");
    // Changing the scope after the decision landed would suggest it still applied to something.
    expect((screen.getByRole("button", { name: "Always" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
