import type {
  MessageFeedback,
  RevokeReusableGuidanceAuthorizationInput,
  SaveMessageFeedbackInput,
} from "../types/chat";
import { listWebSessionMessageBuckets } from "./web-chat-state";

export async function saveWebMessageFeedback(
  input: SaveMessageFeedbackInput,
): Promise<MessageFeedback> {
  const message = findFeedbackMessage(input.messageId);
  const currentRevision = message.feedback?.revision ?? 0;
  if (currentRevision !== input.expectedRevision) {
    throw new Error(`feedback-conflict:${currentRevision}`);
  }
  if (input.state === "corrected" && !input.correctionNote?.trim()) {
    throw new Error("invalid-feedback");
  }
  if (input.authorizeReusableGuidance && input.state !== "corrected") {
    throw new Error("invalid-feedback");
  }
  if (input.state === null) {
    message.feedback = { state: null, revision: currentRevision + 1 };
    return message.feedback;
  }
  message.feedback = {
    state: input.state,
    revision: currentRevision + 1,
    ...(input.correctionNote?.trim()
      ? { correctionNote: input.correctionNote.trim().slice(0, 1_000) }
      : {}),
    ...(input.authorizeReusableGuidance
      ? {
          reusableGuidanceAuthorization: {
            authorizationId: `web-authorization-${input.messageId}-${currentRevision + 1}`,
            feedbackRevision: currentRevision + 1,
            disclosureVersion: "reusable-correction-guidance-disclosure-v1",
          },
        }
      : {}),
  };
  return message.feedback;
}

export async function revokeWebReusableGuidanceAuthorization(
  input: RevokeReusableGuidanceAuthorizationInput,
): Promise<void> {
  const message = findFeedbackMessage(input.messageId);
  const feedback = message.feedback;
  if (!feedback || feedback.revision !== input.expectedFeedbackRevision) {
    throw new Error(`feedback-conflict:${feedback?.revision ?? 0}`);
  }
  if (!feedback.reusableGuidanceAuthorization) throw new Error("invalid-feedback");
  const revoked = { ...feedback };
  delete revoked.reusableGuidanceAuthorization;
  message.feedback = revoked;
}

function findFeedbackMessage(messageId: string) {
  const message = listWebSessionMessageBuckets()
    .flat()
    .find((candidate) => candidate.id === messageId);
  if (!message || message.role !== "assistant" || message.status !== "completed") {
    throw new Error("message-not-eligible");
  }
  return message;
}
