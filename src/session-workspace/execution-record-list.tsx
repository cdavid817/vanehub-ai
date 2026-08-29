import { useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  MeasuredVirtualList,
  type MeasuredVirtualListHandle,
} from "../components/measured-virtual-list";
import type { ExecutionRecord, EvidenceRecordId } from "../types/session-workspace-evidence";
import { ExecutionRecordRow } from "./execution-record-row";

type ListItem = { kind: "record"; record: ExecutionRecord } | { kind: "load-more" };

/**
 * The loaded records, virtualized.
 *
 * Reuses the workspace's existing measured list rather than introducing a second virtualizer: two
 * of them in one product means two sets of scroll-restoration and measurement bugs, and the
 * difference is invisible until one of them is the only one that was fixed.
 *
 * The key is the record id, which is what keeps a row's identity stable while the window recycles
 * DOM nodes underneath it — an index key would move a selection to whichever row happened to take
 * that slot after an append.
 */
export function ExecutionRecordList({
  ariaLabel,
  hasMore,
  loading,
  onLoadMore,
  onSelect,
  records,
  selectedId,
}: {
  ariaLabel: string;
  hasMore: boolean;
  loading: boolean;
  onLoadMore: () => void;
  onSelect: (record: ExecutionRecord) => void;
  records: readonly ExecutionRecord[];
  selectedId: EvidenceRecordId | null;
}) {
  const { t } = useTranslation();
  const listRef = useRef<MeasuredVirtualListHandle>(null);
  const items: ListItem[] = [
    ...records.map((record) => ({ kind: "record" as const, record })),
    ...(hasMore ? [{ kind: "load-more" as const }] : []),
  ];

  return (
    <MeasuredVirtualList
      ariaLabel={ariaLabel}
      className="h-full"
      estimateSize={() => 56}
      getItemKey={(item, index) => (item.kind === "record" ? item.record.id : `load-more-${index}`)}
      items={items}
      itemClassName="pb-1.5"
      overscan={6}
      ref={listRef}
      renderItem={(item) =>
        item.kind === "record" ? (
          <ExecutionRecordRow
            isSelected={item.record.id === selectedId}
            onSelect={onSelect}
            record={item.record}
          />
        ) : (
          <button
            className="h-8 w-full rounded border border-border text-xs hover:bg-muted"
            data-testid="execution-records-load-more"
            disabled={loading}
            onClick={onLoadMore}
            type="button"
          >
            {t(loading ? "executionRecords.loading" : "executionRecords.loadMore")}
          </button>
        )
      }
      testId="execution-record-list"
    />
  );
}
