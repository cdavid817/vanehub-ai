// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import {
  evidenceRecordIdSchema,
  evidenceSessionIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import { EVIDENCE_PAGE_LIMITS } from "../types/session-workspace-evidence";
import type { ExecutionRecord, QueryCoverage } from "../types/session-workspace-evidence";
import { ExecutionRecordList } from "./execution-record-list";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const coverage: QueryCoverage = { state: "complete", reasonCodes: [], truncated: false };

function record(index: number): ExecutionRecord {
  return {
    id: evidenceRecordIdSchema.parse(`tool:call-${index}`),
    kind: "tool",
    sessionId,
    status: "succeeded",
    fidelity: "native",
    coverage,
    toolName: `tool_${index}`,
    source: "native",
  };
}

/**
 * jsdom gives every element a zero size, and the virtualizer measures its scroll element through
 * `offsetWidth` and `offsetHeight`. Without a viewport it renders nothing at all, and a bound
 * asserted against nothing is not an assertion.
 */
function shimLayout({ rowHeight, viewport }: { rowHeight: number; viewport: number }) {
  const isViewport = (element: HTMLElement) =>
    element.getAttribute("data-testid") === "execution-record-list";
  Object.defineProperty(HTMLElement.prototype, "offsetHeight", {
    configurable: true,
    get(this: HTMLElement) {
      return isViewport(this) ? viewport : rowHeight;
    },
  });
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", {
    configurable: true,
    get: () => 800,
  });
  return () => {
    Reflect.deleteProperty(HTMLElement.prototype, "offsetHeight");
    Reflect.deleteProperty(HTMLElement.prototype, "offsetWidth");
  };
}

describe("ExecutionRecordList", () => {
  let restore: (() => void) | null = null;

  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  afterEach(() => {
    restore?.();
    restore = null;
    vi.restoreAllMocks();
  });

  it("mounts a bounded number of rows for a maximum-size page", () => {
    restore = shimLayout({ rowHeight: 56, viewport: 480 });
    const records = Array.from({ length: EVIDENCE_PAGE_LIMITS.maximum }, (_, index) =>
      record(index),
    );

    render(
      <I18nextProvider i18n={i18n}>
        <ExecutionRecordList
          ariaLabel="Execution records"
          hasMore={false}
          loading={false}
          onLoadMore={() => undefined}
          onSelect={() => undefined}
          records={records}
          selectedId={null}
        />
      </I18nextProvider>,
    );

    const list = screen.getByTestId("execution-record-list");
    expect(list.getAttribute("data-virtual-count")).toBe(String(EVIDENCE_PAGE_LIMITS.maximum));
    // The whole point of the virtualizer: five hundred rows in the page, a viewport's worth in the
    // DOM. A list that mounted them all would stall the tab it lives in.
    const rendered = Number(list.getAttribute("data-rendered-count"));
    expect(rendered).toBeGreaterThan(0);
    expect(rendered).toBeLessThan(40);
    expect(screen.getAllByTestId("execution-record-row").length).toBe(rendered);
  });

  it("keeps a row's identity across recycling by keying on the record id", () => {
    restore = shimLayout({ rowHeight: 56, viewport: 480 });
    const view = render(
      <I18nextProvider i18n={i18n}>
        <ExecutionRecordList
          ariaLabel="Execution records"
          hasMore={false}
          loading={false}
          onLoadMore={() => undefined}
          onSelect={() => undefined}
          records={[record(0), record(1)]}
          selectedId={evidenceRecordIdSchema.parse("tool:call-1")}
        />
      </I18nextProvider>,
    );

    // An index key would move the selection to whichever row took that slot after an append.
    const selected = view.container.querySelector('[aria-current="true"]');
    expect(selected?.getAttribute("data-record-id")).toBe("tool:call-1");
  });

  it("offers a load-more control only while the server said there is more", async () => {
    restore = shimLayout({ rowHeight: 56, viewport: 480 });
    const onLoadMore = vi.fn();
    const user = userEvent.setup();
    const view = render(
      <I18nextProvider i18n={i18n}>
        <ExecutionRecordList
          ariaLabel="Execution records"
          hasMore
          loading={false}
          onLoadMore={onLoadMore}
          onSelect={() => undefined}
          records={[record(0)]}
          selectedId={null}
        />
      </I18nextProvider>,
    );

    await user.click(screen.getByTestId("execution-records-load-more"));
    expect(onLoadMore).toHaveBeenCalledTimes(1);

    view.rerender(
      <I18nextProvider i18n={i18n}>
        <ExecutionRecordList
          ariaLabel="Execution records"
          hasMore={false}
          loading={false}
          onLoadMore={onLoadMore}
          onSelect={() => undefined}
          records={[record(0)]}
          selectedId={null}
        />
      </I18nextProvider>,
    );
    expect(screen.queryByTestId("execution-records-load-more")).toBeNull();
  });
});
