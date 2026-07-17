import { useEffect } from "react";
import {
  CheckCircle2,
  CircleAlert,
  Info,
  TriangleAlert,
  X,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { NotificationRecord, NotificationType } from "./notification-types";

const toastTone: Record<NotificationType, { Icon: LucideIcon; className: string }> = {
  success: {
    Icon: CheckCircle2,
    className: "border-[hsl(var(--success))]/40 bg-[hsl(var(--success-soft))] text-[hsl(var(--success))]",
  },
  error: {
    Icon: CircleAlert,
    className: "border-[hsl(var(--danger))]/40 bg-[hsl(var(--danger-soft))] text-[hsl(var(--danger))]",
  },
  warning: {
    Icon: TriangleAlert,
    className: "border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning-soft))] text-[hsl(var(--warning))]",
  },
  info: {
    Icon: Info,
    className: "border-primary/40 bg-[hsl(var(--nav-active-soft))] text-primary",
  },
};

export function isNotificationRelevant(notification: NotificationRecord, activeSessionId: string | null) {
  return notification.scope.kind === "global" || notification.scope.sessionId === activeSessionId;
}

function NotificationToast({
  activeSessionId,
  notification,
  onBeginToastExit,
  onHideToast,
}: {
  activeSessionId: string | null;
  notification: NotificationRecord;
  onBeginToastExit: (id: string) => void;
  onHideToast: (id: string) => void;
}) {
  const { t } = useTranslation();
  const { Icon, className } = toastTone[notification.type];

  useEffect(() => {
    if (notification.toastState !== "visible") return;
    const timer = window.setTimeout(
      () => onBeginToastExit(notification.id),
      notification.durationMs,
    );
    return () => window.clearTimeout(timer);
  }, [notification.durationMs, notification.id, notification.toastState, onBeginToastExit]);

  useEffect(() => {
    if (notification.toastState !== "exiting") return;
    const timer = window.setTimeout(() => onHideToast(notification.id), 180);
    return () => window.clearTimeout(timer);
  }, [notification.id, notification.toastState, onHideToast]);

  if (!isNotificationRelevant(notification, activeSessionId)) return null;

  return (
    <article
      className={cn(
        "ucd-panel pointer-events-auto grid w-full grid-cols-[auto_minmax(0,1fr)_auto] gap-3 rounded-lg border p-3 shadow-xl",
        notification.toastState === "exiting"
          ? "animate-out fade-out slide-out-to-right-2 duration-200"
          : "animate-in fade-in slide-in-from-right-2 duration-200",
      )}
      role={notification.type === "error" || notification.type === "warning" ? "alert" : "status"}
    >
      <span className={cn("mt-0.5 flex h-7 w-7 items-center justify-center rounded-md border", className)}>
        <Icon className="h-4 w-4" aria-hidden="true" />
      </span>
      <span className="min-w-0">
        <span className="block break-words text-sm font-semibold text-foreground">{notification.title}</span>
        {notification.message ? (
          <span className="mt-0.5 block break-words text-xs leading-5 text-muted-foreground">
            {notification.message}
          </span>
        ) : null}
      </span>
      <button
        aria-label={t("notifications.dismiss")}
        className="ucd-list-row flex h-7 w-7 items-center justify-center rounded-md"
        onClick={() => onBeginToastExit(notification.id)}
        title={t("notifications.dismiss")}
        type="button"
      >
        <X className="h-4 w-4" aria-hidden="true" />
      </button>
    </article>
  );
}

export function NotificationToastViewport({
  activeSessionId,
  notifications,
  onBeginToastExit,
  onHideToast,
}: {
  activeSessionId: string | null;
  notifications: NotificationRecord[];
  onBeginToastExit: (id: string) => void;
  onHideToast: (id: string) => void;
}) {
  const { t } = useTranslation();
  const activeToasts = notifications
    .filter((notification) => notification.toastState !== "hidden")
    .slice()
    .reverse();

  return (
    <div
      aria-label={t("notifications.toastRegion")}
      aria-live="polite"
      className="pointer-events-none fixed inset-x-2 bottom-10 z-[60] grid justify-items-end gap-2 sm:left-auto sm:right-4 sm:w-[min(24rem,calc(100vw-2rem))]"
    >
      {activeToasts.map((notification) => (
        <NotificationToast
          activeSessionId={activeSessionId}
          key={notification.id}
          notification={notification}
          onBeginToastExit={onBeginToastExit}
          onHideToast={onHideToast}
        />
      ))}
    </div>
  );
}
