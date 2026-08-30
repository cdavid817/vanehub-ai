export type NotificationType = "success" | "error" | "warning" | "info";

export type NotificationScope =
  | { kind: "global" }
  | { kind: "session"; sessionId: string };

export type NotificationToastState = "visible" | "exiting" | "hidden";

export interface NotificationNavigationAction {
  label: string;
  path: string;
}

export interface NotificationInput {
  type: NotificationType;
  title: string;
  message?: string;
  scope?: NotificationScope;
  durationMs?: number;
  navigation?: NotificationNavigationAction;
}

export interface NotificationRecord {
  id: string;
  type: NotificationType;
  title: string;
  message?: string;
  scope: NotificationScope;
  durationMs: number;
  createdAt: number;
  read: boolean;
  toastState: NotificationToastState;
  navigation?: NotificationNavigationAction;
}
