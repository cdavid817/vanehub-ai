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

export function Badge({
  className,
  tone = "default",
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & { tone?: BadgeTone }) {
  return (
    <span
      className={cn(
        // A badge is a label, never a paragraph: in a tight flex row it must keep its width and
        // stay on one line rather than folding "会话" into two.
        "inline-flex shrink-0 items-center whitespace-nowrap rounded-sm border px-2 py-0.5 text-xs font-medium",
        tones[tone],
        className,
      )}
      {...props}
    />
  );
}
