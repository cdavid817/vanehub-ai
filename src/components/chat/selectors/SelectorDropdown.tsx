import { useEffect, useRef, type ReactNode } from "react";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "../../../lib/utils";

export interface SelectorOption<T extends string> {
  value: T;
  label: string;
  description?: string;
  icon?: ReactNode;
  disabled?: boolean;
}

export function SelectorButton({
  compact,
  icon,
  label,
  onClick,
  open,
  title,
}: {
  compact?: boolean;
  icon: ReactNode;
  label: string;
  onClick: () => void;
  open: boolean;
  title: string;
}) {
  return (
    <button
      className="inline-flex h-8 max-w-48 items-center gap-1.5 rounded-md border border-border bg-background px-2 text-xs hover:bg-muted"
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
      title={title}
      type="button"
    >
      {icon}
      <span className={cn("truncate", compact && "hidden")}>{label}</span>
      <ChevronDown className={cn("h-3.5 w-3.5 shrink-0 transition-transform", open && "rotate-180", compact && "hidden")} aria-hidden="true" />
    </button>
  );
}

export function SelectorDropdown<T extends string>({
  children,
  onClose,
  onSelect,
  options,
  value,
}: {
  children?: ReactNode;
  onClose: () => void;
  onSelect: (value: T) => void;
  options?: Array<SelectorOption<T>>;
  value?: T;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (!ref.current?.contains(event.target as Node)) onClose();
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    const timer = setTimeout(() => document.addEventListener("mousedown", handlePointerDown), 0);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose]);

  return (
    <div
      className="absolute bottom-full left-0 z-50 mb-1 min-w-52 max-w-[min(28rem,calc(100vw-2rem))] rounded-md border border-border bg-background p-1 shadow-xl"
      ref={ref}
    >
      {options?.map((option) => (
        <button
          className={cn(
            "flex w-full items-start gap-2 rounded px-2 py-2 text-left text-xs hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50",
            option.value === value && "bg-[hsl(var(--nav-active-soft))] text-primary",
          )}
          disabled={option.disabled}
          key={option.value}
          onClick={() => {
            if (option.disabled) return;
            onSelect(option.value);
            onClose();
          }}
          type="button"
        >
          <span className="mt-0.5 h-3.5 w-3.5 shrink-0">{option.icon}</span>
          <span className="min-w-0 flex-1">
            <span className="block truncate font-medium">{option.label}</span>
            {option.description ? <span className="mt-0.5 block truncate text-muted-foreground">{option.description}</span> : null}
          </span>
          {option.value === value ? <Check className="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" /> : null}
        </button>
      ))}
      {children}
    </div>
  );
}
