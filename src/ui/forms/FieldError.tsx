import { AlertCircle } from "lucide-react";
import { cn } from "../../lib/utils";

export interface FieldErrorProps {
  /** Already-localized validation message; renders nothing when absent. */
  message?: string;
  /** Wire to the field's `aria-describedby` so assistive tech announces this alongside the field. */
  id?: string;
  className?: string;
}

/** Sits tight against its field — validation errors must stay next to the field, not in a toast. */
export function FieldError({ message, id, className }: FieldErrorProps) {
  if (!message) return null;
  return (
    <p className={cn("mt-1 flex items-start gap-1 text-xs text-destructive", className)} id={id} role="alert">
      <AlertCircle aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0" />
      {message}
    </p>
  );
}
