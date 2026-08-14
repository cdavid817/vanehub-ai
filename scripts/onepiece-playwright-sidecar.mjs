import readline from "node:readline";
import { Buffer } from "node:buffer";

const PROTOCOL_VERSION = 1;
const MAX_MESSAGE_BYTES = 1024 * 1024;
let shuttingDown = false;
let nextContextId = 1;
const contexts = new Map();
let browser;

async function playwrightBrowser() {
  if (!browser) {
    const { chromium } = await import("playwright");
    browser = await chromium.launch({ headless: process.env.VANEHUB_BROWSER_HEADLESS === "1" });
  }
  return browser;
}

function ownedContext(request) {
  return contexts.get(request.params?.context_id);
}

function ownedPage(context, requestedPageId) {
  if (requestedPageId) return context.pages.get(requestedPageId);
  return context.pages.values().next().value;
}

async function registerOwnedPage(context, page) {
  const existing = context.pageIds.get(page);
  if (existing) return existing;
  if (context.pages.size >= context.policy.max_pages) {
    await page.close().catch(() => {});
    return null;
  }
  const pageId = `page-${context.nextPageId++}`;
  context.pages.set(pageId, page);
  context.pageIds.set(page, pageId);
  page.on("close", () => context.pages.delete(pageId));
  page.on("download", (download) => download.cancel().catch(() => {}));
  return pageId;
}

function boundedString(value, maxChars) {
  const chars = Array.from(String(value ?? ""));
  return { value: chars.slice(0, maxChars).join(""), truncated: chars.length > maxChars };
}

async function pageResult(context, page, payload, truncated = false) {
  return {
    page_id: context.pageIds.get(page),
    frame_id: "main-frame",
    url: page.url(),
    payload,
    truncated,
  };
}

async function dispatchPage(request) {
  const context = ownedContext(request);
  if (!context) return response(request.request_id, false, null, "context_not_found");
  const operation = request.params?.input ?? {};
  if (context.handedOff && request.method !== "page.resume") {
    return response(request.request_id, false, null, "automation_handed_off");
  }
  let page = ownedPage(context, operation.page_id);
  if (!page) {
    page = await context.browserContext.newPage();
    if (!(await registerOwnedPage(context, page))) {
      return response(request.request_id, false, null, "page_limit_reached");
    }
  }
  const input = operation.input ?? {};
  switch (request.method) {
    case "page.handoff":
      context.handedOff = true;
      return response(request.request_id, true, await pageResult(context, page, { handed_off: true }));
    case "page.resume":
      context.handedOff = false;
      return response(request.request_id, true, await pageResult(context, page, { inspection_required: true }));
    case "page.navigate":
      await page.goto(input.url, { waitUntil: "domcontentloaded", timeout: 30000 });
      return response(request.request_id, true, await pageResult(context, page, { title: await page.title() }));
    case "page.go_back":
      await page.goBack({ waitUntil: "domcontentloaded", timeout: 30000 });
      return response(request.request_id, true, await pageResult(context, page, { title: await page.title() }));
    case "page.go_forward":
      await page.goForward({ waitUntil: "domcontentloaded", timeout: 30000 });
      return response(request.request_id, true, await pageResult(context, page, { title: await page.title() }));
    case "page.click":
      await page.locator(input.selector).first().click({ timeout: 10000 });
      return response(request.request_id, true, await pageResult(context, page, { clicked: true }));
    case "page.fill":
      await page.locator(input.selector).first().fill(input.text, { timeout: 10000 });
      return response(request.request_id, true, await pageResult(context, page, { filled: true }));
    case "page.extract": {
      const text = boundedString(await page.locator(input.selector).first().innerText({ timeout: 10000 }), 30000);
      return response(request.request_id, true, await pageResult(context, page, { text: text.value }, text.truncated));
    }
    case "page.inspect": {
      const snapshot = await page.locator("body").evaluate((body) => {
        const visible = (element) => {
          const style = globalThis.getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0 && rect.height > 0;
        };
        return Array.from(body.querySelectorAll("a,button,input,textarea,select,[role]"))
          .filter(visible)
          .slice(0, 200)
          .map((element, index) => ({
            ref: `element-${index + 1}`,
            tag: element.tagName.toLowerCase(),
            role: element.getAttribute("role"),
            text: (element.innerText || element.getAttribute("aria-label") || "").slice(0, 500),
            type: element.getAttribute("type"),
            value: element.matches('input[type="password"]') ? null : (element.value || null),
          }));
      });
      return response(request.request_id, true, await pageResult(context, page, { elements: snapshot }, snapshot.length >= 200));
    }
    case "page.screenshot": {
      const bytes = await page.screenshot({ fullPage: input.full_page === true, type: "png" });
      if (bytes.length > 4 * 1024 * 1024) return response(request.request_id, false, null, "screenshot_too_large");
      return response(request.request_id, true, await pageResult(context, page, { media_type: "image/png", bytes_base64: bytes.toString("base64") }));
    }
    case "page.evaluate": {
      const value = await page.evaluate((expression) => Function(`"use strict"; return (${expression})`)(), input.expression);
      const encoded = JSON.stringify(value);
      if (Buffer.byteLength(encoded ?? "null", "utf8") > 64 * 1024) return response(request.request_id, false, null, "evaluation_too_large");
      return response(request.request_id, true, await pageResult(context, page, { value }));
    }
    default:
      return response(request.request_id, false, null, "unsupported_method");
  }
}

function response(requestId, ok, result = null, errorCode = null) {
  return {
    protocol_version: PROTOCOL_VERSION,
    request_id: requestId,
    ok,
    result,
    error_code: errorCode,
  };
}

function write(message) {
  const encoded = JSON.stringify(message);
  if (Buffer.byteLength(encoded, "utf8") > MAX_MESSAGE_BYTES) {
    process.exitCode = 2;
    return;
  }
  process.stdout.write(`${encoded}\n`);
}

async function dispatch(request) {
  if (request?.protocol_version !== PROTOCOL_VERSION || typeof request.request_id !== "string") {
    return response(request?.request_id ?? "invalid", false, null, "protocol_mismatch");
  }
  switch (request.method) {
    case "handshake":
      return response(request.request_id, true, {
        protocol_version: PROTOCOL_VERSION,
        worker: "onepiece-playwright",
      });
    case "health":
      return response(request.request_id, true, { status: "ready" });
    case "context.create": {
      const policy = request.params?.policy;
      if (
        policy?.incognito !== true ||
        policy?.persistent !== false ||
        policy?.import_cookies !== false ||
        policy?.extensions !== false ||
        policy?.http_credentials !== false ||
        !Number.isInteger(policy?.max_pages) ||
        policy.max_pages < 1 ||
        policy.max_pages > 4
      ) {
        return response(request.request_id, false, null, "unsafe_context_policy");
      }
      const activeBrowser = await playwrightBrowser();
      const browserContext = await activeBrowser.newContext({
        acceptDownloads: false,
        serviceWorkers: "block",
      });
      const contextId = `browser-context-${nextContextId++}`;
      const context = {
        owner: request.params.owner,
        policy,
        browserContext,
        pages: new Map(),
        pageIds: new WeakMap(),
        nextPageId: 1,
        handedOff: false,
      };
      contexts.set(contextId, context);
      browserContext.on("page", (page) => registerOwnedPage(context, page).catch(() => page.close().catch(() => {})));
      return response(request.request_id, true, { context_id: contextId });
    }
    case "context.close": {
      const context = contexts.get(request.params?.context_id);
      if (context) await context.browserContext.close();
      const removed = contexts.delete(request.params?.context_id);
      return response(request.request_id, removed, { closed: removed }, removed ? null : "context_not_found");
    }
    case "shutdown":
      shuttingDown = true;
      return response(request.request_id, true, { status: "stopping" });
    default:
      if (request.method?.startsWith("page.")) return dispatchPage(request);
      return response(request.request_id, false, null, "unsupported_method");
  }
}

const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
let requestQueue = Promise.resolve();
lines.on("line", (line) => {
  requestQueue = requestQueue.then(async () => {
  if (Buffer.byteLength(line, "utf8") > MAX_MESSAGE_BYTES) {
    write(response("oversized", false, null, "message_too_large"));
    lines.close();
    return;
  }
  let request;
  try {
    request = JSON.parse(line);
  } catch {
    write(response("malformed", false, null, "malformed_message"));
    return;
  }
  try {
    write(await dispatch(request));
  } catch {
    write(response(request.request_id, false, null, "operation_failed"));
  }
  if (shuttingDown) {
    lines.close();
  }
  });
});

lines.on("close", async () => {
  for (const context of contexts.values()) await context.browserContext.close().catch(() => {});
  contexts.clear();
  if (browser) await browser.close().catch(() => {});
  process.exit(0);
});
