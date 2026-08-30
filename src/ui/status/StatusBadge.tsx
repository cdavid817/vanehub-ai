import { useId, type HTMLAttributes } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "../../lib/utils";

export type StatusTone = "neutral" | "running" | "success" | "warning" | "danger" | "information" | "blocked" | "attention";

const TONE_CLASS: Record<StatusTone, string> = {
  neutral: "ucd-status-neutral",
  running: "ucd-status-running",
  success: "ucd-status-success",
  warning: "ucd-status-warning",
  danger: "ucd-status-danger",
  information: "ucd-status-information",
  blocked: "ucd-status-blocked",
  attention: "ucd-status-attention",
};

export interface StatusBadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone: StatusTone;
  /** Visible status text — status is never conveyed by tone/color alone, so this is required. */
  label: string;
  icon?: LucideIcon;
  /** Extra screen-reader elaboration beyond the visible label, e.g. why a state is blocked. */
  description?: string;
}

export function StatusBadge({ tone, label, icon: Icon, description, className, ...props }: StatusBadgeProps) {
  const descriptionId = useId();
  return (
    <span
      aria-describedby={description ? descriptionId : undefined}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-sm border px-2 py-0.5 text-xs font-medium",
        TONE_CLASS[tone],
        className,
      )}
      {...props}
    >
      {Icon ? <Icon aria-hidden="true" className="h-3.5 w-3.5" /> : <span aria-hidden="true" className="h-2 w-2 shrink-0 rounded-full bg-current" />}
      {label}
      {description ? (
        <span className="sr-only" id={descriptionId}>{description}</span>
      ) : null}
    </span>
  );
}
