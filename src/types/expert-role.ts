/// A role is reusable and describes a job; a seat binds one role to one Agent for one session.
/// Roles are deliberately not an Agent attribute, because the same installed CLI must be able to
/// review in one session and architect in another.

export type ExpertRoleOrigin = "builtin" | "user";

export interface ExpertRoleReviewPolicy {
  /** Whether a seat holding this role may be recommended as a peer reviewer. */
  peerReviewer: boolean;
  /**
   * Whether such a recommendation should prefer a different model family. Same-family models make
   * correlated errors, so a cross-family reviewer catches more.
   */
  requireDifferentFamily: boolean;
}

export interface ExpertRole {
  id: string;
  displayName: string;
  /** Emoji or short glyph shown on the seat and on every message it speaks. */
  avatar: string;
  /** Hex colour used for the speaker band, so a reader can tell seats apart at a glance. */
  color: string;
  /**
   * One line naming what this role is for. Required, and not decorative: it is published to the
   * other seats as the basis for choosing whom to hand off to.
   */
  responsibility: string;
  /** The role text injected through the Agent CLI's native system-prompt channel. */
  instruction: string;
  /** Ids of existing Skills this role relies on. Roles reference Skills rather than replacing them. */
  skillIds: string[];
  reviewPolicy: ExpertRoleReviewPolicy;
  /** Soft preference only; a role never binds itself to a specific Agent. */
  preferredProviders: string[];
  origin: ExpertRoleOrigin;
  createdAt: string;
  updatedAt: string;
}

export interface SaveExpertRoleInput {
  /** Omitted when creating; present when updating an existing role. */
  id?: string | null;
  displayName: string;
  avatar: string;
  color: string;
  responsibility: string;
  instruction: string;
  skillIds: string[];
  reviewPolicy: ExpertRoleReviewPolicy;
  preferredProviders: string[];
}
