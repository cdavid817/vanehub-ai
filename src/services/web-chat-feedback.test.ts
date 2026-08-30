import { afterEach, describe, expect, it } from "vitest";
import type { ChatMessage } from "../types/chat";
import { getWebSessionMessages, setWebSessionMessages } from "./web-chat-state";
import {
  revokeWebReusableGuidanceAuthorization,
  saveWebMessageFeedback,
} from "./web-chat-feedback";

const message: ChatMessage = {
  id: "message-1",
  sessionId: "session-1",
  role: "assistant",
  content: "Answer",
  status: "completed",
  createdAt: "2026-08-27T00:00:00Z",
  updatedAt: "2026-08-27T00:00:00Z",
  sessionSequence: 1,
  executionRunId: null,
};

describe("Web chat feedback authorization", () => {
  afterEach(() => setWebSessionMessages("session-1", []));

  it("simulates explicit authorization, CAS, replacement, and revocation", async () => {
    setWebSessionMessages("session-1", [{ ...message }]);
    const saved = await saveWebMessageFeedback({
      messageId: "message-1",
      expectedRevision: 0,
      state: "corrected",
      correctionNote: "Use the retry boundary.",
      authorizeReusableGuidance: true,
    });
    expect(saved.reusableGuidanceAuthorization).toMatchObject({
      feedbackRevision: 1,
      disclosureVersion: "reusable-correction-guidance-disclosure-v1",
    });
    await expect(revokeWebReusableGuidanceAuthorization({
      messageId: "message-1",
      expectedFeedbackRevision: 0,
    })).rejects.toThrow("feedback-conflict:1");
    await revokeWebReusableGuidanceAuthorization({
      messageId: "message-1",
      expectedFeedbackRevision: 1,
    });
    expect(getWebSessionMessages("session-1")[0]?.feedback)
      .not.toHaveProperty("reusableGuidanceAuthorization");
  });

  it("keeps authorization default-off and rejects non-correction authorization", async () => {
    setWebSessionMessages("session-1", [{ ...message }]);
    await expect(saveWebMessageFeedback({
      messageId: "message-1",
      expectedRevision: 0,
      state: "helpful",
      authorizeReusableGuidance: true,
    })).rejects.toThrow("invalid-feedback");
    const saved = await saveWebMessageFeedback({
      messageId: "message-1",
      expectedRevision: 0,
      state: "corrected",
      correctionNote: "Use the retry boundary.",
    });
    expect(saved.reusableGuidanceAuthorization).toBeUndefined();
  });
});
