import { useEffect, useId, useState, type ReactNode } from "react";
import { cn } from "../../lib/utils";
import { useFocusTrap } from "./use-focus-trap";

export type SheetPlacement = "left" | "right" | "bottom" | "full";

const CLOSED_TRANSFORM: Record<SheetPlacement, string> = {
  left: "-translate-x-full",
  right: "translate-x-full",
  bottom: "translate-y-full",
  full: "opacity-0",
};

const PLACEMENT_POSITION: Record<SheetPlacement, string> = {
  left: "inset-y-0 left-0 h-full",
  right: "inset-y-0 right-0 h-full",
  bottom: "inset-x-0 bottom-0 max-h-[85vh]",
  full: "inset-0",
};

export interface SheetProps {
  title: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
  onClose: () => void;
  closeDisabled?: boolean;
  placement: SheetPlacement;
  returnFocus?: HTMLElement | null;
  /** Only meaningful for `left`/`right`; `bottom`/`full` size from content and the viewport. */
  widthClassName?: string;
  className?: string;
}

/**
 * Side/full-height equivalent of `ApplicationDialog` for the narrow/compact layouts in design.md
 * Decision 3 ("Compact：...均为互斥 Sheet；Narrow：...辅助区域全屏 Sheet"), sharing its focus-trap
 * and focus-return behavior via `useFocusTrap` instead of reimplementing it.
 */
export function Sheet({
  title,
  description,
  children,
  footer,
  onClose,
  closeDisabled = false,
  placement,
  returnFocus,
  widthClassName = "w-96",
  className,
}: SheetProps) {
  const titleId = useId();
  const descriptionId = useId();
  const sheetRef = useFocusTrap<HTMLElement>({ closeDisabled, onClose, returnFocus });
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const frame = requestAnimationFrame(() => setOpen(true));
    return () => cancelAnimationFrame(frame);
  }, []);

  return (
    <div
      className="fixed inset-0 z-50 bg-black/50"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !closeDisabled) onClose();
      }}
      role="presentation"
    >
      <section
        aria-describedby={description ? descriptionId : undefined}
        aria-labelledby={titleId}
        aria-modal="true"
        className={cn(
          "ucd-pane-transition absolute flex flex-col overflow-hidden border-border bg-background shadow-2xl",
          PLACEMENT_POSITION[placement],
          placement === "left" || placement === "right" ? widthClassName : "w-full",
          placement === "left" && "border-r",
          placement === "right" && "border-l",
          placement === "bottom" && "border-t",
          open ? "translate-x-0 translate-y-0 opacity-100" : CLOSED_TRANSFORM[placement],
          className,
        )}
        ref={sheetRef}
        role="dialog"
        tabIndex={-1}
      >
        <div className="shrink-0 border-b border-border px-5 py-4">
          <h3 className="text-lg font-semibold" id={titleId}>{title}</h3>
          {description ? <p className="mt-1 text-sm leading-6 text-muted-foreground" id={descriptionId}>{description}</p> : null}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-5">{children}</div>
        {footer ? <div className="shrink-0 border-t border-border px-5 py-4">{footer}</div> : null}
      </section>
    </div>
  );
}
