export type MessageFeedbackState = "helpful" | "unhelpful" | "corrected";

export interface ReusableGuidanceAuthorization {
  authorizationId: string;
  feedbackRevision: number;
  disclosureVersion: string;
}

export interface MessageFeedback {
  state: MessageFeedbackState | null;
  revision: number;
  correctionNote?: string;
  reusableGuidanceAuthorization?: ReusableGuidanceAuthorization;
}

export interface SaveMessageFeedbackInput {
  messageId: string;
  expectedRevision: number;
  state: MessageFeedbackState | null;
  correctionNote?: string;
  authorizeReusableGuidance?: boolean;
}

export interface RevokeReusableGuidanceAuthorizationInput {
  messageId: string;
  expectedFeedbackRevision: number;
}
