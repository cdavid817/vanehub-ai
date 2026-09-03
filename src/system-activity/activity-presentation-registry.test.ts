import { describe, expect, it } from "vitest";
import {
  activityEventCodes, activityPayloadSchemas, activityReasonCodes, activitySeverities,
  activityStatuses,
} from "./activity-contracts";
import {
  activityEventPresentation, activityPayloadIcons, activityPayloadPresentation,
  activityReasonPresentation, activitySeverityPresentation, activityStatusPresentation,
} from "./activity-presentation-registry";

describe("activity presentation registry", () => {
  it("covers every closed event, status, severity, reason, and payload registry", () => {
    expect(Object.keys(activityEventPresentation)).toEqual(activityEventCodes);
    expect(Object.keys(activityStatusPresentation)).toEqual(activityStatuses);
    expect(Object.keys(activitySeverityPresentation)).toEqual(activitySeverities);
    expect(Object.keys(activityReasonPresentation)).toEqual(activityReasonCodes);
    expect(Object.keys(activityPayloadPresentation)).toEqual(activityPayloadSchemas);
    expect(Object.keys(activityPayloadIcons)).toEqual(activityPayloadSchemas);
  });

  it("provides locale keys, accessible labels, icons, and read-only renderer ids", () => {
    for (const presentation of Object.values(activityEventPresentation)) {
      expect(presentation.titleKey).toMatch(/^systemActivity\./);
      expect(presentation.accessibleLabelKey).toMatch(/^systemActivity\./);
      expect(presentation.icon).toBeTypeOf("object");
    }
    for (const presentation of Object.values(activityPayloadPresentation)) {
      expect(presentation.renderer).not.toMatch(/html|markdown|diff|action|media|file/);
      expect(presentation.accessibleLabelKey).toMatch(/\.label$/);
    }
  });
});
