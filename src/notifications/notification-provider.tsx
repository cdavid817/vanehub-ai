import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useReducer,
  type ReactNode,
} from "react";
import {
  createNotificationRecord,
  notificationReducer,
} from "./notification-reducer";
import type { NotificationInput, NotificationRecord } from "./notification-types";
import { NotificationToastViewport } from "./notification-toast-viewport";

interface NotificationContextValue {
  notifications: NotificationRecord[];
  unreadCount: number;
  notify: (input: NotificationInput) => string;
  markRead: (id: string) => void;
  markAllRead: () => void;
  remove: (id: string) => void;
  clear: () => void;
}

interface NotificationPresentationContextValue {
  notifications: NotificationRecord[];
  beginToastExit: (id: string) => void;
  hideToast: (id: string) => void;
}

const NotificationContext = createContext<NotificationContextValue | null>(null);
const NotificationPresentationContext = createContext<NotificationPresentationContextValue | null>(null);
let nextNotificationSequence = 0;

function nextNotificationId(createdAt: number) {
  nextNotificationSequence += 1;
  return `notification-${createdAt}-${nextNotificationSequence}`;
}

export function NotificationProvider({ children }: { children: ReactNode }) {
  const [notifications, dispatch] = useReducer(notificationReducer, []);

  const notify = useCallback((input: NotificationInput) => {
    const createdAt = Date.now();
    const id = nextNotificationId(createdAt);
    dispatch({
      type: "published",
      notification: createNotificationRecord(input, id, createdAt),
    });
    return id;
  }, []);

  const beginToastExit = useCallback((id: string) => {
    dispatch({ type: "toast-exit-started", id });
  }, []);
  const hideToast = useCallback((id: string) => {
    dispatch({ type: "toast-hidden", id });
  }, []);
  const markRead = useCallback((id: string) => {
    dispatch({ type: "read", id });
  }, []);
  const markAllRead = useCallback(() => {
    dispatch({ type: "all-read" });
  }, []);
  const remove = useCallback((id: string) => {
    dispatch({ type: "removed", id });
  }, []);
  const clear = useCallback(() => {
    dispatch({ type: "cleared" });
  }, []);

  const value = useMemo<NotificationContextValue>(
    () => ({
      notifications,
      unreadCount: notifications.filter((item) => !item.read).length,
      notify,
      markRead,
      markAllRead,
      remove,
      clear,
    }),
    [
      clear,
      markAllRead,
      markRead,
      notifications,
      notify,
      remove,
    ],
  );
  const presentationValue = useMemo<NotificationPresentationContextValue>(
    () => ({ notifications, beginToastExit, hideToast }),
    [beginToastExit, hideToast, notifications],
  );

  return (
    <NotificationContext.Provider value={value}>
      <NotificationPresentationContext.Provider value={presentationValue}>
        {children}
      </NotificationPresentationContext.Provider>
    </NotificationContext.Provider>
  );
}

export function useNotifications() {
  const context = useContext(NotificationContext);
  if (!context) {
    throw new Error("useNotifications must be used inside NotificationProvider");
  }
  return context;
}

export function NotificationHost({ activeSessionId }: { activeSessionId: string | null }) {
  const context = useContext(NotificationPresentationContext);
  if (!context) {
    throw new Error("NotificationHost must be used inside NotificationProvider");
  }
  return (
    <NotificationToastViewport
      activeSessionId={activeSessionId}
      notifications={context.notifications}
      onBeginToastExit={context.beginToastExit}
      onHideToast={context.hideToast}
    />
  );
}
