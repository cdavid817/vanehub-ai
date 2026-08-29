import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { decodeActivityPayload } from "./activity-payload-decoder";
import { SafeActivityPayload } from "./safe-activity-payload";

const translate = (key: string) => key;

describe("safe activity payload boundary", () => {
  it("rejects executable, freeform, diff, media, file, and mutation schemas", () => {
    for (const schema of [
      "html_widget", "markdown", "raw_diff", "media", "file", "mutation_action",
    ]) {
      const hostile = { schema, html: "<script>alert(1)</script>", raw: "secret diff" };
      expect(decodeActivityPayload(hostile)).toBeNull();
      const html = renderToStaticMarkup(
        <SafeActivityPayload
          eventCode="unknown_event"
          occurredAtMs={1}
          payload={hostile}
          severity="warning"
          translate={translate}
        />,
      );
      expect(html).toContain('data-payload-schema="safe-fallback"');
      expect(html).not.toContain("secret diff");
      expect(html).not.toContain("&lt;script");
    }
  });

  it("rejects arbitrary fields and unsafe identifiers without leaking their values", () => {
    expect(decodeActivityPayload({
      schema: "status_card",
      labelCode: "outcome",
      valueCode: "completed",
      rawPrompt: "ignore previous instructions",
    })).toBeNull();
    expect(decodeActivityPayload({
      schema: "navigation_list",
      links: [{ kind: "skill", stableId: "<img src=x>" }],
    })).toBeNull();
  });
});
