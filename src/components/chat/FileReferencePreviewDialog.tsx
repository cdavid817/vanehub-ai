import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { MeasuredVirtualList } from "../measured-virtual-list";
import { ApplicationDialog } from "../ui/application-dialog";
import { Button } from "../ui/button";
import { highlightFileLines } from "../../services/code-highlighting";
import {
  advanceLineSelection,
  isLineSelected,
  selectionToRange,
  type PendingLineSelection,
} from "../../services/line-selection";
import { describeLineRange, type MentionLineRange } from "../../services/composer-mention";
import { agentService } from "../../services/runtime-agent-client";
import type { FileContent } from "../../types/session-workspace";
import { PreviewLineRow } from "./PreviewLineRow";

type LoadState = { kind: "loading" } | { kind: "loaded"; file: FileContent } | { kind: "failed" };

export function FileReferencePreviewDialog({
  name,
  onAttach,
  onCancel,
  path,
  sessionId,
}: {
  name: string;
  onAttach: (range: MentionLineRange) => void;
  onCancel: () => void;
  path: string;
  sessionId: string;
}) {
  const { t } = useTranslation();
  const [state, setState] = useState<LoadState>({ kind: "loading" });
  const [selection, setSelection] = useState<PendingLineSelection | null>(null);

  useEffect(() => {
    let active = true;
    setState({ kind: "loading" });
    agentService
      .readSessionFile(sessionId, path)
      .then((file) => { if (active) setState({ kind: "loaded", file }); })
      .catch(() => { if (active) setState({ kind: "failed" }); });
    return () => { active = false; };
  }, [path, sessionId]);

  const file = state.kind === "loaded" ? state.file : null;
  const lines = useMemo(
    () => (file?.status === "text" && file.content ? highlightFileLines(path, file.content) : []),
    [file, path],
  );
  // Measured on the raw text, not the highlighted markup: the latter is mostly span tags
  // and would overstate the width several times over.
  const widestLine = useMemo(
    () => (file?.content ?? "").split("\n").reduce((widest, line) => Math.max(widest, line.length), 0),
    [file],
  );
  const range = selectionToRange(selection);
  const rangeLabel = describeLineRange(range);
  // Only a readable file can be attached. An oversized or binary one contributes no
  // content to the prompt, so attaching it would put a chip on screen that the Agent
  // never receives anything for.
  const canAttachWhole = file?.status === "text";

  return (
    <ApplicationDialog description={path} maxWidth="max-w-4xl" onClose={onCancel} title={name}>
      {state.kind === "loading" ? (
        <p className="py-8 text-center text-sm text-muted-foreground">{t("chat.filePreview.loading")}</p>
      ) : state.kind === "failed" || file === null || file.status === "missing" ? (
        <p className="py-8 text-center text-sm text-destructive">{t("chat.filePreview.unavailable")}</p>
      ) : file.status !== "text" ? (
        <p className="py-8 text-center text-sm text-muted-foreground">
          {t(file.status === "binary" ? "chat.filePreview.binary" : "chat.filePreview.oversized")}
        </p>
      ) : (
        <MeasuredVirtualList
          ariaLabel={t("chat.filePreview.lines")}
          className="h-[55vh] overflow-x-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] font-mono text-xs"
          // The widest line decides how far the list scrolls. Rows are absolutely
          // positioned, so they cannot establish this width themselves; `ch` is exact
          // here because the list is monospaced.
          contentStyle={{ minWidth: `calc(${widestLine}ch + 4.5rem)` }}
          estimateSize={() => 20}
          getItemKey={(line) => String(line.number)}
          // Rows size to their longest line rather than the viewport, which is what lets
          // the list scroll sideways instead of wrapping code.
          itemClassName="w-max min-w-full"
          items={lines}
          overscan={20}
          renderItem={(line) => (
            <PreviewLineRow
              html={line.html}
              number={line.number}
              onSelect={() => setSelection((current) => advanceLineSelection(current, line.number))}
              selected={isLineSelected(selection, line.number)}
            />
          )}
          testId="file-preview-lines"
        />
      )}
      <div className="mt-4 flex flex-wrap items-center justify-between gap-2 border-t border-border pt-4">
        <p className="text-xs text-muted-foreground">
          {rangeLabel
            ? t("chat.filePreview.pendingRange", { range: rangeLabel })
            : t("chat.filePreview.selectHint")}
        </p>
        <div className="flex gap-2">
          <Button onClick={onCancel} type="button" variant="ghost">
            {t("chat.filePreview.cancel")}
          </Button>
          {/* No autofocus target: this button is disabled while the file loads, and a
              disabled element cannot take focus, which would leave focus outside the
              dialog. The dialog element itself takes it instead. */}
          <Button disabled={!canAttachWhole} onClick={() => onAttach({})} type="button" variant="outline">
            {t("chat.filePreview.attachWholeFile")}
          </Button>
          <Button disabled={rangeLabel === null} onClick={() => onAttach(range)} type="button">
            {t("chat.filePreview.attachSelection")}
          </Button>
        </div>
      </div>
    </ApplicationDialog>
  );
}
