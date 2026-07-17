import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { NotificationProvider, useNotifications } from "./notification-provider";
import {
  createNotificationRecord,
  notificationReducer,
  NOTIFICATION_HISTORY_LIMIT,
  VISIBLE_TOAST_LIMIT,
} from "./notification-reducer";
import type { NotificationRecord } from "./notification-types";
import { isNotificationRelevant } from "./notification-toast-viewport";

function publish(state: NotificationRecord[], index: number, sessionId?: string) {
  const notification = createNotificationRecord(
    {
      type: index % 2 === 0 ? "success" : "info",
      title: `Notification ${index}`,
      scope: sessionId ? { kind: "session", sessionId } : { kind: "global" },
    },
    `notification-${index}`,
    index,
  );
  return notificationReducer(state, { type: "published", notification });
}

describe("notificationReducer", () => {
  it("publishes unread notifications and preserves scope", () => {
    const state = publish([], 1, "session-a");

    expect(state).toHaveLength(1);
    expect(state[0]).toMatchObject({
      id: "notification-1",
      read: false,
      toastState: "visible",
      scope: { kind: "session", sessionId: "session-a" },
    });
  });

  it("shows global and active-session toasts without losing other session records", () => {
    const globalNotification = publish([], 1)[0];
    const scopedNotification = publish([], 2, "session-a")[0];

    expect(isNotificationRelevant(globalNotification, null)).toBe(true);
    expect(isNotificationRelevant(scopedNotification, "session-a")).toBe(true);
    expect(isNotificationRelevant(scopedNotification, "session-b")).toBe(false);
  });

  it("hides a toast without removing its history entry", () => {
    const published = publish([], 1);
    const exiting = notificationReducer(published, {
      type: "toast-exit-started",
      id: "notification-1",
    });
    const hidden = notificationReducer(exiting, { type: "toast-hidden", id: "notification-1" });

    expect(exiting[0].toastState).toBe("exiting");
    expect(hidden[0].toastState).toBe("hidden");
    expect(hidden[0].title).toBe("Notification 1");
  });

  it("updates read state and history management consistently", () => {
    let state = publish(publish([], 1), 2);
    state = notificationReducer(state, { type: "read", id: "notification-1" });
    expect(state.map((item) => item.read)).toEqual([true, false]);

    state = notificationReducer(state, { type: "all-read" });
    expect(state.every((item) => item.read)).toBe(true);

    state = notificationReducer(state, { type: "removed", id: "notification-1" });
    expect(state.map((item) => item.id)).toEqual(["notification-2"]);
    expect(notificationReducer(state, { type: "cleared" })).toEqual([]);
  });

  it("bounds history and visible toast counts while keeping newest entries", () => {
    let state: NotificationRecord[] = [];
    for (let index = 0; index < NOTIFICATION_HISTORY_LIMIT + 3; index += 1) {
      state = publish(state, index);
    }

    expect(state).toHaveLength(NOTIFICATION_HISTORY_LIMIT);
    expect(state[0].id).toBe("notification-3");
    expect(state.filter((item) => item.toastState !== "hidden")).toHaveLength(VISIBLE_TOAST_LIMIT);
    expect(state.at(-1)?.id).toBe(`notification-${NOTIFICATION_HISTORY_LIMIT + 2}`);
  });
});

describe("NotificationProvider", () => {
  it("provides an empty notification state to descendants", () => {
    function Consumer() {
      const context = useNotifications();
      return (
        <span data-has-lifecycle={String("beginToastExit" in context)}>
          {`${context.notifications.length}:${context.unreadCount}`}
        </span>
      );
    }

    expect(
      renderToStaticMarkup(
        <NotificationProvider>
          <Consumer />
        </NotificationProvider>,
      ),
    ).toContain("data-has-lifecycle=\"false\"");
  });

  it("rejects use outside the provider", () => {
    function Consumer() {
      useNotifications();
      return null;
    }

    expect(() => renderToStaticMarkup(<Consumer />)).toThrow(
      "useNotifications must be used inside NotificationProvider",
    );
  });
});
