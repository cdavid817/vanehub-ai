import { useEffect, useRef, useState } from "react";
import {
  Bell,
  Check,
  CheckCheck,
  CircleAlert,
  Info,
  Trash2,
  TriangleAlert,
  CheckCircle2,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { useNotifications } from "./notification-provider";
import type { NotificationType } from "./notification-types";

const centerIcon: Record<NotificationType, { Icon: LucideIcon; className: string }> = {
  success: { Icon: CheckCircle2, className: "text-[hsl(var(--success))]" },
  error: { Icon: CircleAlert, className: "text-[hsl(var(--danger))]" },
  warning: { Icon: TriangleAlert, className: "text-[hsl(var(--warning))]" },
  info: { Icon: Info, className: "text-primary" },
};

export function formatNotificationTimestamp(createdAt: number, language: string) {
  return new Intl.DateTimeFormat(language, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(createdAt);
}

export function NotificationCenter() {
  const { i18n, t } = useTranslation();
  const { notifications, unreadCount, markRead, markAllRead, remove, clear } = useNotifications();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const newestFirst = notifications.slice().reverse();

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: PointerEvent) {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) {
        setOpen(false);
      }
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  return (
    <div className="relative" ref={rootRef}>
      <button
        aria-controls="notification-center"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-label={
          unreadCount > 0
            ? t("notifications.unreadCount", { count: unreadCount })
            : t("layout.notifications")
        }
        className="ucd-list-row relative flex h-8 w-9 items-center justify-center rounded-md"
        onClick={() => setOpen((current) => !current)}
        title={t("layout.notifications")}
        type="button"
      >
        <Bell className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
        {unreadCount > 0 ? (
          <span className="absolute right-0.5 top-0 flex min-h-4 min-w-4 items-center justify-center rounded-full bg-[hsl(var(--danger))] px-1 text-[9px] font-semibold leading-none text-white">
            {unreadCount > 99 ? "99+" : unreadCount}
          </span>
        ) : null}
      </button>

      {open ? (
        <section
          aria-label={t("layout.notifications")}
          className="ucd-panel absolute right-0 top-[calc(100%+0.5rem)] z-50 grid w-[min(22rem,calc(100vw-1rem))] grid-rows-[auto_minmax(0,1fr)] overflow-hidden rounded-lg border border-border !bg-[hsl(var(--panel))] shadow-xl"
          id="notification-center"
          role="dialog"
        >
          <header className="flex min-h-11 items-center justify-between gap-2 border-b border-border px-3 py-2">
            <div className="min-w-0">
              <h2 className="truncate text-sm font-semibold">{t("layout.notifications")}</h2>
              {unreadCount > 0 ? (
                <p className="text-[11px] text-muted-foreground">
                  {t("notifications.unreadCount", { count: unreadCount })}
                </p>
              ) : null}
            </div>
            {notifications.length > 0 ? (
              <div className="flex shrink-0 items-center gap-1">
                {unreadCount > 0 ? (
                  <button
                    aria-label={t("notifications.markAllRead")}
                    className="ucd-list-row flex h-7 w-7 items-center justify-center rounded-md"
                    onClick={markAllRead}
                    title={t("notifications.markAllRead")}
                    type="button"
                  >
                    <CheckCheck className="h-4 w-4" aria-hidden="true" />
                  </button>
                ) : null}
                <button
                  aria-label={t("notifications.clear")}
                  className="ucd-list-row flex h-7 w-7 items-center justify-center rounded-md"
                  onClick={clear}
                  title={t("notifications.clear")}
                  type="button"
                >
                  <Trash2 className="h-4 w-4" aria-hidden="true" />
                </button>
              </div>
            ) : null}
          </header>

          <div className="max-h-[min(26rem,calc(100vh-6rem))] overflow-y-auto">
            {newestFirst.length === 0 ? (
              <div className="grid min-h-40 place-items-center px-6 py-8 text-center">
                <span>
                  <Bell className="mx-auto h-6 w-6 text-muted-foreground" aria-hidden="true" />
                  <span className="mt-2 block text-sm font-medium">{t("notifications.empty")}</span>
                  <span className="mt-1 block text-xs text-muted-foreground">
                    {t("notifications.emptyDescription")}
                  </span>
                </span>
              </div>
            ) : (
              newestFirst.map((notification) => {
                const { Icon, className } = centerIcon[notification.type];
                return (
                  <article
                    className={cn(
                      "grid grid-cols-[auto_minmax(0,1fr)_auto] gap-2 border-b border-border px-3 py-2.5 last:border-b-0",
                      !notification.read && "bg-[hsl(var(--nav-active-soft))]",
                    )}
                    key={notification.id}
                  >
                    <Icon className={cn("mt-0.5 h-4 w-4", className)} aria-hidden="true" />
                    <button
                      aria-label={
                        notification.read
                          ? notification.title
                          : t("notifications.markReadItem", { title: notification.title })
                      }
                      className="min-w-0 text-left"
                      onClick={() => markRead(notification.id)}
                      type="button"
                    >
                      <span className="flex items-start gap-2">
                        <span className="min-w-0 flex-1 break-words text-xs font-semibold">
                          {notification.title}
                        </span>
                        {!notification.read ? (
                          <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-primary" />
                        ) : null}
                      </span>
                      {notification.message ? (
                        <span className="mt-0.5 block break-words text-xs leading-5 text-muted-foreground">
                          {notification.message}
                        </span>
                      ) : null}
                      <span className="mt-1 flex flex-wrap items-center gap-2 text-[10px] text-muted-foreground">
                        <time dateTime={new Date(notification.createdAt).toISOString()}>
                          {formatNotificationTimestamp(notification.createdAt, i18n.language)}
                        </time>
                        {notification.scope.kind === "session" ? (
                          <span>{t("notifications.sessionScope")}</span>
                        ) : null}
                      </span>
                    </button>
                    <span className="flex flex-col gap-1">
                      {!notification.read ? (
                        <button
                          aria-label={t("notifications.markRead")}
                          className="ucd-list-row flex h-6 w-6 items-center justify-center rounded"
                          onClick={() => markRead(notification.id)}
                          title={t("notifications.markRead")}
                          type="button"
                        >
                          <Check className="h-3.5 w-3.5" aria-hidden="true" />
                        </button>
                      ) : null}
                      <button
                        aria-label={t("notifications.remove")}
                        className="ucd-list-row flex h-6 w-6 items-center justify-center rounded"
                        onClick={() => remove(notification.id)}
                        title={t("notifications.remove")}
                        type="button"
                      >
                        <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                      </button>
                    </span>
                  </article>
                );
              })
            )}
          </div>
        </section>
      ) : null}
    </div>
  );
}
