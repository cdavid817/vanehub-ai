import type { ReactNode } from "react";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import type { ChatMessage } from "../types/chat";
import type { Session } from "../types/agent";
import type { SessionRuntimeSurfaceId, SessionSurfaceId } from "./session-surface-registry";

export interface ConversationVisibilityControls {
  infoPanelExpanded: boolean;
  onToggleInfoPanel: () => void;
  onOpenIm: () => void;
  onToggleSessionList: () => void;
  onToggleWorkspaceTabs: () => void;
  sessionListExpanded: boolean;
  workspaceTabsExpanded: boolean;
}

export interface SessionWorkspaceRegionsProps {
  activeSession: Session | null;
  apiComposer?: ReactNode;
  focusMode?: boolean;
  /** The persisted "preferred Runtime Panel tab" — see `useWorkspaceEvidenceScopeValue`'s doc comment. */
  initialRuntimeSurface?: SessionRuntimeSurfaceId;
  isStreaming?: boolean;
  messages: ChatMessage[];
  messagesPartial: boolean;
  onLoadEarlier?: () => void;
  onOpenSettings: () => void;
  onRuntimeMaximizedChange?: (maximized: boolean) => void;
  recoveryNotice?: ReactNode;
  requestedSurface?: SessionSurfaceId | null;
  requestedSurfaceNonce?: number;
  runtimeMaximized?: boolean;
  sessionActivationKey: number;
  /** Null in a single-seat session, which has no turn to hand off. */
  turnStatus?: TurnStatus | null;
  visibilityControls?: ConversationVisibilityControls;
  workspaceTabsCollapsed?: boolean;
}
