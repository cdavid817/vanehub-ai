import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ActivityPayloadRenderer } from "./activity-payload-renderer";
import type { ActivityPayload } from "./activity-contracts";

const translate = (key: string, values?: Record<string, string | number>) =>
  `${key}${values?.id ? `:${values.id}` : ""}`;

const payloads: ActivityPayload[] = [
  { schema: "status_card", labelCode: "outcome", valueCode: "completed" },
  { schema: "stage_timeline", stages: [{ code: "assess", status: "succeeded" }] },
  { schema: "check_summary", passed: 4, failed: 1, review: 2 },
  { schema: "metric_summary", metrics: { evidence_count: 7, duration_ms: 120 } },
  { schema: "navigation_list", links: [{ kind: "run", stableId: "run-safe" }] },
  { schema: "supersession_notice", priorEventId: "event-prior" },
];

describe("ActivityPayloadRenderer", () => {
  it("renders every supported schema as an accessible read-only block", () => {
    for (const payload of payloads) {
      const html = renderToStaticMarkup(
        <ActivityPayloadRenderer payload={payload} translate={translate} />,
      );
      expect(html).toContain(`data-payload-schema="${payload.schema}"`);
      expect(html).toContain("aria-label=");
      expect(html).toContain("dark:");
      expect(html).not.toMatch(/<script|contenteditable|type="submit"/i);
    }
  });

  it("renders safe identities as escaped text and navigation as a non-submit button", () => {
    const html = renderToStaticMarkup(
      <ActivityPayloadRenderer
        onNavigate={() => undefined}
        payload={{
          schema: "navigation_list",
          links: [{ kind: "skill", stableId: "<img src=x onerror=alert(1)>" }],
        }}
        translate={translate}
      />,
    );
    expect(html).toContain("&lt;img src=x onerror=alert(1)&gt;");
    expect(html).toContain('type="button"');
    expect(html).not.toContain("<img");
  });
});
