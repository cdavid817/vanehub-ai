import { Plus } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { linkableGoalTargets, type GoalLinkTarget } from "../contracts/goal";
import { ExecutionTargetOptionSummary } from "./execution-target-option-summary";
import type { ExecutionTargetKind, ExecutionTargetOption } from "./execution-target-providers";
import { ExecutionTargetRawIdField } from "./execution-target-raw-id-field";
import { ExecutionTargetResults } from "./execution-target-result-list";
import { useExecutionTargetSearch } from "./use-execution-target-search";

const fieldClass = "ucd-input rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export interface ExecutionTargetPickerProps {
  pending: boolean;
  onLink: (targetKind: GoalLinkTarget, targetId: string) => void;
}

/**
 * design.md Decision 12's `ExecutionTargetPicker` (tasks 15.4-15.6): search-then-confirm, not
 * search-then-instant-link. Clicking a result only stages it in `selected` -- its type/title/
 * project/status render once more in the confirm panel below, via the same
 * `ExecutionTargetOptionSummary` the result row itself used, before `onLink` (and therefore the
 * actual goal mutation) ever fires. This satisfies 15.6 literally ("...before linking"), not just
 * "visible in the list the click came from."
 *
 * Changing `kind` clears both `query` and `selected`: a Work Item selected while searching Loops
 * would otherwise survive a kind switch as a stale, mismatched confirm panel.
 */
export function ExecutionTargetPicker({ onLink, pending }: ExecutionTargetPickerProps) {
  const { t } = useTranslation();
  const [kind, setKind] = useState<ExecutionTargetKind>("loop");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<ExecutionTargetOption | null>(null);
  const { error, loading, options } = useExecutionTargetSearch(kind, query);

  function changeKind(next: string) {
    setKind(next as ExecutionTargetKind);
    setQuery("");
    setSelected(null);
  }

  function confirmLink() {
    if (!selected) return;
    onLink(kind, selected.id);
    setSelected(null);
    setQuery("");
  }

  return (
    <div className="grid gap-2 rounded-md border border-border bg-muted/10 p-3">
      <div className="flex flex-wrap gap-2">
        <select
          aria-label={t("goals.fields.targetKind")}
          className={fieldClass}
          disabled={pending}
          onChange={(event) => changeKind(event.target.value)}
          value={kind}
        >
          {linkableGoalTargets.map((candidate) => <option key={candidate} value={candidate}>{t(`goals.target.${candidate}`)}</option>)}
        </select>
        <input
          aria-label={t("goals.picker.searchLabel")}
          className={`${fieldClass} min-w-0 flex-1`}
          disabled={pending}
          onChange={(event) => { setQuery(event.target.value); setSelected(null); }}
          placeholder={t("goals.picker.searchPlaceholder")}
          type="search"
          value={query}
        />
      </div>

      {selected ? (
        <div className="grid gap-2 rounded border border-primary/40 bg-primary/5 p-2">
          <ExecutionTargetOptionSummary kind={kind} option={selected} />
          <div className="flex items-center justify-end gap-2">
            <Button onClick={() => setSelected(null)} size="sm" type="button" variant="outline">
              {t("goals.picker.changeSelection")}
            </Button>
            <Button disabled={pending} onClick={confirmLink} size="sm" type="button">
              <Plus aria-hidden="true" />{t("goals.actions.link")}
            </Button>
          </div>
        </div>
      ) : (
        <ExecutionTargetResults error={error} kind={kind} loading={loading} onSelect={setSelected} options={options} query={query} />
      )}

      <ExecutionTargetRawIdField onLink={onLink} pending={pending} />
    </div>
  );
}
