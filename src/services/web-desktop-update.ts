import type { DesktopUpdateSnapshot, UpdateOperationReceipt, UpdatePreferences } from "../types/desktop-update";
import { defaultUpdateChannel } from "./desktop-update-policy";

const currentVersion = __APP_VERSION__;
let preferences: UpdatePreferences = { automaticCheck: false, channel: defaultUpdateChannel(currentVersion) };
let snapshot: DesktopUpdateSnapshot = { phase: "idle", currentVersion, channel: preferences.channel };
let sequence = 0;

function receipt(next: DesktopUpdateSnapshot): UpdateOperationReceipt {
  snapshot = next;
  return { operationId: next.operationId ?? `web-update-${++sequence}`, snapshot: structuredClone(next) };
}

export const webDesktopUpdateClient = {
  async getSnapshot() { return structuredClone(snapshot); },
  async getPreferences() { return { ...preferences }; },
  async savePreferences(input: UpdatePreferences) {
    preferences = { ...input }; snapshot = { ...snapshot, channel: input.channel }; return { ...preferences };
  },
  async check() {
    const operationId = `web-update-${++sequence}`;
    return receipt({ ...snapshot, phase: "available", operationId, checkedAt: new Date().toISOString(), latestVersion: "0.2.0", releaseNotes: "Signed desktop update preview" });
  },
  async install() {
    const operationId = `web-update-${++sequence}`;
    return receipt({ ...snapshot, phase: "ready-to-restart", operationId, downloadedBytes: 24_000_000, totalBytes: 24_000_000 });
  },
  async restart() { snapshot = { ...snapshot, phase: "up-to-date" }; },
};
