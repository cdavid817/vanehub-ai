import { invoke } from "@tauri-apps/api/core";
import type {
  MessageFeedback,
  RevokeReusableGuidanceAuthorizationInput,
  SaveMessageFeedbackInput,
} from "../types/chat";

export async function saveTauriMessageFeedback(
  input: SaveMessageFeedbackInput,
): Promise<MessageFeedback> {
  const saved = await invoke<{
    messageId: string;
    revision: number;
    state: MessageFeedback["state"] | null;
    correctionNote: string | null;
    reusableGuidanceAuthorization: MessageFeedback["reusableGuidanceAuthorization"] | null;
  }>("save_message_feedback", { input });
  return {
    state: saved.state,
    revision: saved.revision,
    ...(saved.correctionNote ? { correctionNote: saved.correctionNote } : {}),
    ...(saved.reusableGuidanceAuthorization
      ? { reusableGuidanceAuthorization: saved.reusableGuidanceAuthorization }
      : {}),
  };
}

export function revokeTauriReusableGuidanceAuthorization(
  input: RevokeReusableGuidanceAuthorizationInput,
): Promise<void> {
  return invoke<void>("revoke_reusable_guidance_authorization", { input });
}
