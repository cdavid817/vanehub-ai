import { useEffect, useId, useRef, type ReactNode } from "react";

export function ApplicationDialog({
  title,
  description,
  children,
  onClose,
  closeDisabled = false,
  maxWidth = "max-w-2xl",
}: {
  title: string;
  description?: string;
  children: ReactNode;
  onClose: () => void;
  closeDisabled?: boolean;
  maxWidth?: string;
}) {
  const titleId = useId();
  const descriptionId = useId();
  const dialogRef = useRef<HTMLElement>(null);
  const closeRef = useRef(onClose);
  const closeDisabledRef = useRef(closeDisabled);
  closeRef.current = onClose;
  closeDisabledRef.current = closeDisabled;

  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const dialog = dialogRef.current;
    const focusTarget = dialog?.querySelector<HTMLElement>("[data-dialog-autofocus]") ?? dialog;
    focusTarget?.focus();

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !closeDisabledRef.current) closeRef.current();
      if (event.key !== "Tab" || !dialog) return;
      const focusable = Array.from(dialog.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])',
      )).filter((element) => !element.hidden && element.getAttribute("aria-hidden") !== "true");
      if (focusable.length === 0) {
        event.preventDefault();
        dialog.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      previousFocus?.focus();
    };
  }, []);

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
        className={`max-h-[calc(100vh-1.5rem)] w-full overflow-y-auto rounded-xl border border-border bg-background p-5 shadow-2xl sm:max-h-[90vh] sm:p-6 ${maxWidth}`}
        ref={dialogRef}
        role="dialog"
        tabIndex={-1}
      >
        <div className="mb-5 border-b border-border pb-4">
          <h3 className="text-lg font-semibold" id={titleId}>{title}</h3>
          {description ? <p className="mt-1 text-sm leading-6 text-muted-foreground" id={descriptionId}>{description}</p> : null}
        </div>
        {children}
      </section>
    </div>
  );
}
