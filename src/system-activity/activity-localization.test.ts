import { describe, expect, it } from "vitest";
import { createInstance } from "i18next";
import { loadLocaleResource, supportedLocales } from "../i18n/supported-locales";
import { activityEventPresentation, activityPayloadPresentation } from "./activity-presentation-registry";

describe("system activity localization", () => {
  it("ships every registry key in every supported locale", async () => {
    const keys = [
      ...Object.values(activityEventPresentation).flatMap((entry) => [entry.titleKey, entry.accessibleLabelKey]),
      ...Object.values(activityPayloadPresentation).map((entry) => entry.accessibleLabelKey),
      "systemActivity.payload.unavailable",
      "systemActivity.supersession.notice",
    ];
    for (const locale of supportedLocales) {
      const resource = await loadLocaleResource(locale.id);
      for (const key of new Set(keys)) expect(resource[key], `${locale.id}:${key}`).toBeTruthy();
    }
  });

  it("keeps persisted codes stable while locale presentation changes", async () => {
    const code = "run_completed";
    const key = activityEventPresentation[code].titleKey;
    const simplified = (await loadLocaleResource("zh-CN"))[key];
    const english = (await loadLocaleResource("en"))[key];
    expect(simplified).not.toBe(english);
    expect(code).toBe("run_completed");
  });

  it("uses the shared default locale when an active-locale key is unavailable", async () => {
    const key = activityEventPresentation.run_failed.titleKey;
    const simplified = await loadLocaleResource("zh-CN");
    const incompleteJapanese = { ...await loadLocaleResource("ja") };
    delete incompleteJapanese[key];
    const instance = createInstance();
    await instance.init({
      fallbackLng: "zh-CN",
      lng: "ja",
      resources: {
        "zh-CN": { translation: simplified },
        ja: { translation: incompleteJapanese },
      },
    });
    expect(instance.t(key)).toBe(simplified[key]);
  });
});
