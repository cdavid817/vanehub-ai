import { X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { GoalLink, GoalLinkTarget } from "../contracts/goal";
import { groupLinks } from "./goal-presentation";

/**
 * Below this, a kind's group renders in full; at or above it, only this many rows render until the
 * reader asks for the rest. 15.10's "bounded relationship sections": nothing here reads a hard
 * limit from the server, so a goal with hundreds of links (e.g. a scripted or duplicate-import
 * bug) must not silently turn this section into hundreds of unbounded DOM rows by default.
 */
const VISIBLE_LINKS_PER_GROUP = 20;

export interface GoalRelationshipSectionsProps {
  links: GoalLink[];
  pending: boolean;
  onUnlink: (targetKind: GoalLinkTarget, targetId: string) => void;
}

/** Same row markup goal-detail.tsx rendered inline before this was extracted -- moving it here
 *  adds the per-group count and the bounded/"show more" behavior, not a visual redesign. */
export function GoalRelationshipSections({ links, onUnlink, pending }: GoalRelationshipSectionsProps) {
  const { t } = useTranslation();
  const [expandedGroups, setExpandedGroups] = useState<ReadonlySet<GoalLinkTarget>>(new Set());

  return (
    <>
      {groupLinks(links).map((group) => {
        const expanded = expandedGroups.has(group.kind);
        const visible = expanded ? group.links : group.links.slice(0, VISIBLE_LINKS_PER_GROUP);
        const hiddenCount = group.links.length - visible.length;
        return (
          <div className="grid gap-1" key={group.kind}>
            <h4 className="text-xs font-medium text-muted-foreground">
              {/* The label is its own element (not concatenated inline with the separator/count)
                  so it remains a clean, single-node match for both assistive tech and
                  `getByText` -- a mixed text node otherwise reads as "broken up by multiple
                  elements" to both. */}
              <span>{t(`goals.target.${group.kind}`)}</span> · <span className="tabular-nums">{group.links.length}</span>
            </h4>
            {visible.map((link) => (
              <div className="flex items-center justify-between gap-2 rounded border border-border px-2 py-1" key={`${link.targetKind}:${link.targetId}`}>
                <span className="truncate text-xs">{link.targetId}</span>
                <span className="flex shrink-0 items-center gap-2">
                  <span className={`text-xs ${link.progress === "unresolvable" ? "text-amber-600 dark:text-amber-400" : "text-muted-foreground"}`}>
                    {t(group.kind === "session" ? "goals.linkProgress.notCounted" : `goals.linkProgress.${link.progress}`)}
                  </span>
                  <Button aria-label={t("goals.actions.unlink")} disabled={pending} onClick={() => onUnlink(link.targetKind, link.targetId)} size="icon" type="button" variant="ghost">
                    <X aria-hidden="true" />
                  </Button>
                </span>
              </div>
            ))}
            {hiddenCount > 0 ? (
              <Button
                className="justify-self-start"
                onClick={() => setExpandedGroups((current) => new Set(current).add(group.kind))}
                size="sm"
                type="button"
                variant="ghost"
              >
                {t("goals.links.showMore", { count: hiddenCount })}
              </Button>
            ) : null}
          </div>
        );
      })}
    </>
  );
}
