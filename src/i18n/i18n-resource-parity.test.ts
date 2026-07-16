import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

describe("i18n resources", () => {
  it("keeps zh-CN and en key sets aligned", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(zhCN).sort());
  });
});
