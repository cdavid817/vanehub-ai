import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { EvidenceRecordId, EvidenceSessionId, ExecutionRecord } from "../types/session-workspace-evidence";
import type { ChatMessage } from "../types/chat";
import { ExecutionRecordList } from "./execution-record-list";
import { legacyActivityRecords } from "./legacy-activity-adapter";
import { LegacySourceNotice, TerminalHistoryEmpty } from "./terminal-history-states";

/**
 * Legacy activity, kept as its own list.
 *
 * Not merged into the native records, and not merely tagged inside them. The two corpora are
 * different claims — one is what the runtime observed, the other is what an assistant said it was
 * doing — and interleaving them would leave a reader sorting the two apart by badge on every row.
 * A separate list makes the boundary the first thing they see.
 */
export function LegacyActivityList({
  messages,
  messagesPartial,
  onSelect,
  seatId,
  selectedId,
  sessionId,
}: {
  messages: ChatMessage[];
  messagesPartial: boolean;
  onSelect: (record: ExecutionRecord) => void;
  seatId: string | null;
  selectedId: EvidenceRecordId | null;
  sessionId: EvidenceSessionId;
}) {
  const { t } = useTranslation();
  const { coverage, records } = useMemo(
    () => legacyActivityRecords({ messages, messagesPartial, seatId, sessionId }),
    [messages, messagesPartial, seatId, sessionId],
  );

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <LegacySourceNotice coverage={coverage} />
      {records.length === 0 ? (
        // Never "complete-empty": this projection cannot see past compaction, so "there was no
        // activity" is a claim it is not in a position to make.
        <TerminalHistoryEmpty state="partial" />
      ) : (
        <div className="min-h-0 flex-1">
          <ExecutionRecordList
            ariaLabel={t("executionRecords.legacy.listLabel")}
            hasMore={false}
            loading={false}
            onLoadMore={() => undefined}
            onSelect={onSelect}
            records={records}
            selectedId={selectedId}
          />
        </div>
      )}
    </div>
  );
}
