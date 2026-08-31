import { useId, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "../../lib/utils";

export interface AccordionItem {
  id: string;
  /** The clickable header's own content — plain text or a richer node (icon + label + badge). */
  header: ReactNode;
  content: ReactNode;
}

export interface AccordionProps {
  items: AccordionItem[];
  /** Controlled: which section ids are currently expanded. More than one may be open at once. */
  openIds: string[];
  onOpenIdsChange: (ids: string[]) => void;
  className?: string;
}

/**
 * WAI-ARIA Accordion Pattern (https://www.w3.org/WAI/ARIA/apg/patterns/accordion/), generic and
 * reusable across the workbench — not Session Overview specific. Deliberately not built from
 * `role="tablist"`/`role="tabpanel"` (design.md task 9.15's named anti-pattern, "four to six
 * equal-width text tabs in a 300px panel"): each section's own header is a real `<button>` with
 * `aria-expanded`/`aria-controls`, and its content is a region named by `aria-labelledby`, not a
 * panel switched by a shared strip of tab buttons.
 *
 * Every header is wrapped in an `<h3>`. Fixed rather than a configurable heading-level prop: this
 * primitive's first and only consumer today (Session Overview) nests directly under the
 * Inspector's own `<h2>` title, and a speculative level prop would be untested surface area with
 * no second caller yet to validate it against.
 *
 * Multiple sections may be open at once — unlike a single-open accordion, opening one section
 * never closes another. Session Overview's sections (Participants, Runtime, Workspace, Usage,
 * Skills, IM, Code Index) are independent content, not mutually exclusive views of the same thing.
 *
 * Every section's content stays mounted at all times, hidden with the HTML `hidden` attribute
 * while its section is collapsed, rather than being mounted only after its first expand. This
 * matches how every pane Session Overview migrates into this accordion already tracks its own
 * `active` prop internally (session-token-usage-pane.tsx, session-skills-pane.tsx, and friends):
 * "mounted either way ... hidden pane polling its own service costs a request for an answer
 * nobody is looking at." A generic accordion cannot assume every caller wants a lazy first-open
 * mount instead — always-mounted is the simpler, state-preserving default.
 */
export function Accordion({ items, openIds, onOpenIdsChange, className }: AccordionProps) {
  const baseId = useId();
  const openSet = new Set(openIds);

  function toggle(id: string) {
    onOpenIdsChange(openSet.has(id) ? openIds.filter((openId) => openId !== id) : [...openIds, id]);
  }

  return (
    <div className={cn("grid gap-2", className)}>
      {items.map((item) => {
        const open = openSet.has(item.id);
        const headerId = `${baseId}-${item.id}-header`;
        const contentId = `${baseId}-${item.id}-content`;
        return (
          <div className="ucd-muted-panel overflow-hidden rounded-lg" key={item.id}>
            <h3 className="m-0">
              <button
                aria-controls={contentId}
                aria-expanded={open}
                className="ucd-focus-ring flex w-full items-center justify-between gap-2 px-3 py-2.5 text-left text-sm font-semibold hover:bg-muted"
                data-testid={`accordion-header-${item.id}`}
                id={headerId}
                onClick={() => toggle(item.id)}
                type="button"
              >
                <span className="min-w-0 flex-1 truncate">{item.header}</span>
                <ChevronDown
                  aria-hidden="true"
                  className={cn("h-4 w-4 shrink-0 text-muted-foreground transition-transform", open && "rotate-180")}
                />
              </button>
            </h3>
            <div aria-labelledby={headerId} className="border-t border-border/60 p-3" data-testid={`accordion-content-${item.id}`} hidden={!open} id={contentId}>
              {item.content}
            </div>
          </div>
        );
      })}
    </div>
  );
}
