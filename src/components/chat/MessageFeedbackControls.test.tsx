// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../../test/render";
import { agentService } from "../../services/runtime-agent-client";
import { MessageFeedbackControls } from "./MessageFeedbackControls";

vi.mock("../../services/runtime-agent-client", () => ({
  agentService: {
    saveMessageFeedback: vi.fn(),
    revokeReusableGuidanceAuthorization: vi.fn(),
  },
}));

const saveFeedback = vi.mocked(agentService.saveMessageFeedback);
const revokeAuthorization = vi.mocked(agentService.revokeReusableGuidanceAuthorization);

describe("MessageFeedbackControls", () => {
  beforeEach(() => {
    saveFeedback.mockReset();
    revokeAuthorization.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("saves helpful feedback with the current CAS revision", async () => {
    saveFeedback.mockResolvedValue({ state: "helpful", revision: 1 });
    const { user } = renderWithAppProviders(
      <MessageFeedbackControls messageId="message-1" />,
    );

    await user.click(screen.getByRole("button", { name: "有帮助" }));
    await waitFor(() => expect(saveFeedback).toHaveBeenCalledWith({
      messageId: "message-1",
      expectedRevision: 0,
      state: "helpful",
    }));
    expect(screen.getByRole("button", { name: "有帮助" }).getAttribute("aria-pressed")).toBe("true");
  });

  it("collects a bounded correction and confirms replacement", async () => {
    saveFeedback.mockResolvedValue({
      state: "corrected",
      revision: 5,
      correctionNote: "Use the retry boundary.",
    });
    const { user } = renderWithAppProviders(
      <MessageFeedbackControls
        feedback={{ state: "helpful", revision: 4 }}
        messageId="message-2"
      />,
    );

    await user.click(screen.getByRole("button", { name: "提出纠正" }));
    const correction = screen.getByLabelText("需要纠正什么？");
    expect(correction.getAttribute("maxlength")).toBe("1000");
    await user.type(correction, "Use the retry boundary.");
    await user.click(screen.getByRole("button", { name: "保存" }));

    // Replacing an existing rating asks for confirmation before it overwrites the previous one.
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "确认" }));

    await waitFor(() => expect(saveFeedback).toHaveBeenCalledWith({
      messageId: "message-2",
      expectedRevision: 4,
      state: "corrected",
      correctionNote: "Use the retry boundary.",
    }));
  });

  it("keeps controls retryable after a save conflict", async () => {
    saveFeedback.mockRejectedValue(new Error("feedback-conflict:2"));
    const { user } = renderWithAppProviders(
      <MessageFeedbackControls messageId="message-3" />,
    );
    await user.click(screen.getByRole("button", { name: "没有帮助" }));
    expect((await screen.findByRole("alert")).textContent).toContain("请重新加载消息后重试");
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "没有帮助" }).disabled).toBe(false);
  });

  it("keeps reusable-guidance authorization default-off and supports explicit revocation", async () => {
    saveFeedback.mockResolvedValue({
      state: "corrected",
      revision: 1,
      correctionNote: "Use the retry boundary.",
      reusableGuidanceAuthorization: {
        authorizationId: "authorization-1",
        feedbackRevision: 1,
        disclosureVersion: "reusable-correction-guidance-disclosure-v1",
      },
    });
    revokeAuthorization.mockResolvedValue();
    const { user } = renderWithAppProviders(
      <MessageFeedbackControls messageId="message-4" />,
    );
    await user.click(screen.getByRole("button", { name: "提出纠正" }));
    expect(screen.getByRole("checkbox").getAttribute("checked")).toBeNull();
    await user.type(screen.getByLabelText("需要纠正什么？"), "Use the retry boundary.");
    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(saveFeedback).toHaveBeenCalledWith({
      messageId: "message-4",
      expectedRevision: 0,
      state: "corrected",
      correctionNote: "Use the retry boundary.",
      authorizeReusableGuidance: true,
    }));
    await user.click(screen.getByRole("button", { name: "撤销授权" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "确认" }));
    await waitFor(() => expect(revokeAuthorization).toHaveBeenCalledWith({
      messageId: "message-4",
      expectedFeedbackRevision: 1,
    }));
  });
});
