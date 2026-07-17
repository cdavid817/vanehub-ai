import type { NotificationInput, NotificationRecord } from "./notification-types";

export const NOTIFICATION_HISTORY_LIMIT = 20;
export const VISIBLE_TOAST_LIMIT = 4;
export const DEFAULT_NOTIFICATION_DURATION_MS = 5_000;

export type NotificationAction =
  | { type: "published"; notification: NotificationRecord }
  | { type: "toast-exit-started"; id: string }
  | { type: "toast-hidden"; id: string }
  | { type: "read"; id: string }
  | { type: "all-read" }
  | { type: "removed"; id: string }
  | { type: "cleared" };

export function createNotificationRecord(
  input: NotificationInput,
  id: string,
  createdAt: number,
): NotificationRecord {
  return {
    id,
    type: input.type,
    title: input.title,
    ...(input.message ? { message: input.message } : {}),
    scope: input.scope ?? { kind: "global" },
    durationMs: Math.max(1_000, input.durationMs ?? DEFAULT_NOTIFICATION_DURATION_MS),
    createdAt,
    read: false,
    toastState: "visible",
  };
}

function publishNotification(
  state: NotificationRecord[],
  notification: NotificationRecord,
): NotificationRecord[] {
  let toastSlotsToRelease = Math.max(
    0,
    state.filter((item) => item.toastState !== "hidden").length - VISIBLE_TOAST_LIMIT + 1,
  );

  const boundedToasts = state.map((item) => {
    if (toastSlotsToRelease > 0 && item.toastState !== "hidden") {
      toastSlotsToRelease -= 1;
      return { ...item, toastState: "hidden" as const };
    }
    return item;
  });

  return [...boundedToasts, notification].slice(-NOTIFICATION_HISTORY_LIMIT);
}

export function notificationReducer(
  state: NotificationRecord[],
  action: NotificationAction,
): NotificationRecord[] {
  switch (action.type) {
    case "published":
      return publishNotification(state, action.notification);
    case "toast-exit-started":
      return state.map((item) =>
        item.id === action.id && item.toastState === "visible"
          ? { ...item, toastState: "exiting" }
          : item,
      );
    case "toast-hidden":
      return state.map((item) =>
        item.id === action.id && item.toastState !== "hidden"
          ? { ...item, toastState: "hidden" }
          : item,
      );
    case "read":
      return state.map((item) => (item.id === action.id ? { ...item, read: true } : item));
    case "all-read":
      return state.map((item) => (item.read ? item : { ...item, read: true }));
    case "removed":
      return state.filter((item) => item.id !== action.id);
    case "cleared":
      return [];
  }
}
