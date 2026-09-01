import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

/**
 * The dialog used to stack six sibling sections in one flat grid, which made "who runs this",
 * "where does it run" and "what is it called" read as three equally weighted questions. Framing
 * each step restores the decision order without hiding any control behind disclosure.
 *
 * This is the one component every one of the wizard's 4 steps wraps itself in, and each step
 * conditionally-renders (unmounts/remounts) rather than hides via CSS -- so focusing this
 * heading on mount, once here, covers every way a reader can land on a step: clicking Next,
 * clicking Back, or a Review-step "Fix" link jumping straight to the step that owns an error
 * (task 11.14's "error focus"). Without it, focus stays on whichever now-removed button was
 * clicked and a keyboard/screen-reader reader loses their place on every step change.
 */
export function CreateSessionSection({
  children,
  hint,
  icon: Icon,
  title,
}: {
  children: ReactNode;
  hint: string;
  icon: LucideIcon;
  title: string;
}) {
  const headingRef = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    headingRef.current?.focus();
  }, []);
  return (
    <section className="grid gap-3 rounded-lg border border-border/70 bg-[hsl(var(--panel-muted))] p-3">
      <header className="flex items-start gap-2.5">
        <span className="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-md border border-primary/30 bg-[hsl(var(--nav-active-soft))] text-primary">
          <Icon aria-hidden="true" className="h-3.5 w-3.5" />
        </span>
        <div className="min-w-0">
          <h4 className="text-sm font-semibold leading-5 outline-hidden" ref={headingRef} tabIndex={-1}>{title}</h4>
          <p className="mt-0.5 text-xs leading-5 text-muted-foreground">{hint}</p>
        </div>
      </header>
      <div className="grid gap-3 rounded-md bg-background p-3">{children}</div>
    </section>
  );
}
