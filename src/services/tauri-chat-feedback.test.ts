import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import {
  revokeTauriReusableGuidanceAuthorization,
  saveTauriMessageFeedback,
} from "./tauri-chat-feedback";

describe("Tauri chat feedback adapter", () => {
  beforeEach(() => invokeMock.mockReset());

  it("preserves the explicit authorization witness and maps revocation", async () => {
    const input = {
      messageId: "message-1",
      expectedRevision: 2,
      state: "corrected" as const,
      correctionNote: "Use the retry boundary.",
      authorizeReusableGuidance: true,
    };
    invokeMock.mockResolvedValueOnce({
      messageId: "message-1",
      revision: 3,
      state: "corrected",
      correctionNote: "Use the retry boundary.",
      reusableGuidanceAuthorization: {
        authorizationId: "authorization-1",
        feedbackRevision: 3,
        disclosureVersion: "reusable-correction-guidance-disclosure-v1",
      },
    });
    const saved = await saveTauriMessageFeedback(input);
    expect(saved.reusableGuidanceAuthorization?.feedbackRevision).toBe(3);
    await revokeTauriReusableGuidanceAuthorization({
      messageId: "message-1",
      expectedFeedbackRevision: 3,
    });
    expect(invokeMock.mock.calls).toEqual([
      ["save_message_feedback", { input }],
      ["revoke_reusable_guidance_authorization", {
        input: { messageId: "message-1", expectedFeedbackRevision: 3 },
      }],
    ]);
  });
});
