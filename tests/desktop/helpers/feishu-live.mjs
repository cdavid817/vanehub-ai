import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const evidencePath = () => path.join(
  globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR,
  "feishu-live-scenarios.json",
);
const nativeLogPath = () => path.join(
  globalThis.process.env.VANEHUB_APP_DATA_DIR,
  "logs",
  "vanehub.log",
);

async function existingResults() {
  try {
    const evidence = JSON.parse(await readFile(evidencePath(), "utf8"));
    return Array.isArray(evidence.results) ? evidence.results : [];
  } catch {
    return [];
  }
}

export async function recordLiveScenario(scenario, status, safeErrorCode) {
  const results = (await existingResults()).filter((entry) => entry.scenario !== scenario);
  results.push({ scenario, status, ...(safeErrorCode ? { safeErrorCode } : {}) });
  await writeFile(
    evidencePath(),
    `${JSON.stringify({ livePlatform: true, fixture: false, results }, null, 2)}\n`,
    "utf8",
  );
}

export function safeLiveFailureCode(reason) {
  const message = reason instanceof Error ? reason.message : String(reason);
  const known = message.match(/\b(?:communications|connector|credential|feishu|im)-[a-z0-9-]+\b/iu);
  if (known) return known[0].toLowerCase();
  if (/timed out/iu.test(message)) return "webdriver-command-timeout";
  return "live-scenario-failed";
}

export async function waitForLiveNativeBridge() {
  await globalThis.browser.waitUntil(async () => {
    try {
      const connectors = await globalThis.browser.tauri.execute(
        ({ core }) => core.invoke("list_im_connectors"),
      );
      return Array.isArray(connectors);
    } catch {
      return false;
    }
  }, {
    timeout: 120_000,
    interval: 500,
    timeoutMsg: "The native IM service bridge did not become ready.",
  });
}

async function finalDeliveryRecorded(sessionId, messageId) {
  let contents;
  try {
    contents = await readFile(nativeLogPath(), "utf8");
  } catch {
    return false;
  }
  return contents.split(/\r?\n/u).some((line) => {
    if (!line) return false;
    try {
      const event = JSON.parse(line);
      return event.category === "im.connector"
        && event.context?.operation === "deliver-final"
        && event.context?.safeCode === "delivered"
        && event.context?.internalSessionId === sessionId
        && event.context?.internalMessageId === messageId;
    } catch {
      return false;
    }
  });
}

export async function waitForLiveFinalDelivery(sessionId, messageId) {
  await globalThis.browser.waitUntil(
    () => finalDeliveryRecorded(sessionId, messageId),
    {
      timeout: 60_000,
      interval: 500,
      timeoutMsg: "The completed Agent response was not delivered to Feishu.",
    },
  );
}

export async function visibleUiSafeErrorCode() {
  try {
    const error = await globalThis.$('div[aria-live="assertive"]');
    if (!await error.isExisting() || !await error.isDisplayed()) return null;
    const safeErrorCode = safeLiveFailureCode(await error.getText());
    return safeErrorCode === "live-scenario-failed" ? null : safeErrorCode;
  } catch {
    return null;
  }
}

export async function qualifyLiveScenario(scenario, action) {
  try {
    const value = await action();
    await recordLiveScenario(scenario, "PASSED");
    return value;
  } catch (reason) {
    const safeErrorCode = await visibleUiSafeErrorCode() ?? safeLiveFailureCode(reason);
    await recordLiveScenario(scenario, "FAILED", safeErrorCode);
    throw new Error(`${scenario} failed (${safeErrorCode})`, { cause: reason });
  }
}

export function operatorInstruction(message) {
  globalThis.process.stdout.write(`\n[Feishu live operator] ${message}\n`);
}
