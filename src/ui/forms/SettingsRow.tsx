import { useId, type ReactNode } from "react";
import { cn } from "../../lib/utils";
import type { MutationState } from "../async/mutation-state";
import { MutationStatus } from "../async/MutationStatus";
import { FieldError } from "./FieldError";

export interface SettingsRowProps {
  title: string;
  description?: string;
  children: ReactNode;
  /** Only meaningful in "immediate" save mode (design.md Decision 17) — omit for draft-mode rows. */
  mutation?: MutationState;
  onRetryMutation?: () => void;
  onDismissMutation?: () => void;
  /** Already-localized validation message for this field. */
  errorMessage?: string;
  className?: string;
}

export function SettingsRow({
  title,
  description,
  children,
  mutation,
  onRetryMutation,
  onDismissMutation,
  errorMessage,
  className,
}: SettingsRowProps) {
  const errorId = useId();
  return (
    <div className={cn("border-b border-border-subtle px-5 py-4 last:border-b-0 sm:px-6", className)}>
      <div className="grid min-h-18 gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(180px,auto)] sm:items-center">
        <div className="min-w-0">
          <div className="text-sm font-medium leading-5 text-foreground">{title}</div>
          {description ? <div className="mt-0.5 text-xs leading-5 text-muted-foreground">{description}</div> : null}
        </div>
        <div className="flex min-w-0 items-center gap-2 sm:justify-self-end">
          {children}
          <MutationStatus onDismiss={onDismissMutation} onRetry={onRetryMutation} state={mutation} />
        </div>
      </div>
      <FieldError id={errorId} message={errorMessage} />
    </div>
  );
}
