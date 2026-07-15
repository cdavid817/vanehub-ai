import { Loader2 } from "lucide-react";

export function WaitingIndicator() {
  return (
    <span className="inline-flex items-center gap-2 text-xs text-muted-foreground">
      <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" />
      等待响应
    </span>
  );
}
