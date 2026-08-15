import assert from "node:assert/strict";

export async function readNativeSettings() {
  const settings = await globalThis.browser.tauri.execute(({ core }) => core.invoke("get_settings"));
  assert.equal(typeof settings, "object");
  assert.notEqual(settings, null);
  assert.equal(typeof settings.applicationLanguage, "string");
  assert.equal(typeof settings.loggingPolicy?.redactionEnabled, "boolean");
  return settings;
}
