import * as React from "react";
import { cn } from "../../lib/utils";

type BadgeTone = "default" | "success" | "warning" | "danger" | "muted";

const tones: Record<BadgeTone, string> = {
  default: "border-transparent bg-primary text-primary-foreground",
  success: "ucd-status-success",
  warning: "ucd-status-warning",
  danger: "ucd-status-danger",
  muted: "border-border bg-muted text-muted-foreground",
};

/** `children` is required, not just conventionally always passed: 20.11 wants a color-only status
 *  badge to be structurally impossible, and `StatusBadge`'s own `label: string` (`src/ui/status/
 *  StatusBadge.tsx`) already makes that guarantee for tone/status the same way — this makes it a
 *  type error to render a `Badge` carrying only color, mirroring that primitive. */
export interface BadgeProps extends Omit<React.HTMLAttributes<HTMLSpanElement>, "children"> {
  tone?: BadgeTone;
  children: React.ReactNode;
}

export function Badge({
  className,
  tone = "default",
  ...props
}: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-sm border px-2 py-0.5 text-xs font-medium",
        tones[tone],
        className,
      )}
      {...props}
    />
  );
}
