import { useId, type ReactNode } from "react";
import { useFocusTrap } from "../../ui/sheet/use-focus-trap";

export function ApplicationDialog({
  title,
  description,
  children,
  footer,
  onClose,
  closeDisabled = false,
  maxWidth = "max-w-2xl",
  returnFocus,
}: {
  title: string;
  description?: string;
  children: ReactNode;
  /**
   * Pinned below a scrolling body. Callers that omit it keep the original single-scroll
   * rendering, so adding this does not reflow the dialogs that already use this primitive.
   */
  footer?: ReactNode;
  onClose: () => void;
  closeDisabled?: boolean;
  maxWidth?: string;
  returnFocus?: HTMLElement | null;
}) {
  const titleId = useId();
  const descriptionId = useId();
  const dialogRef = useFocusTrap<HTMLElement>({ closeDisabled, onClose, returnFocus });

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-3 sm:p-5"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !closeDisabled) onClose();
      }}
      role="presentation"
    >
      <section
        aria-describedby={description ? descriptionId : undefined}
        aria-labelledby={titleId}
        aria-modal="true"
        className={footer
          ? `flex max-h-[calc(100vh-1.5rem)] w-full flex-col overflow-hidden rounded-xl border border-border bg-background shadow-2xl sm:max-h-[90vh] ${maxWidth}`
          : `max-h-[calc(100vh-1.5rem)] w-full overflow-y-auto rounded-xl border border-border bg-background p-5 shadow-2xl sm:max-h-[90vh] sm:p-6 ${maxWidth}`}
        ref={dialogRef}
        role="dialog"
        tabIndex={-1}
      >
        <div className={footer ? "shrink-0 border-b border-border px-5 py-4 sm:px-6" : "mb-5 border-b border-border pb-4"}>
          <h3 className="text-lg font-semibold" id={titleId}>{title}</h3>
          {description ? <p className="mt-1 text-sm leading-6 text-muted-foreground" id={descriptionId}>{description}</p> : null}
        </div>
        {footer ? (
          <>
            <div className="min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6">{children}</div>
            <div className="shrink-0 border-t border-border px-5 py-4 sm:px-6">{footer}</div>
          </>
        ) : children}
      </section>
    </div>
  );
}
