import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";
import { navigateTo } from "../helpers/navigation.mjs";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

// Window sizes a user actually works at, from a wide desktop down to the 1100x700 floor that
// `tauri.conf.json` enforces as `minWidth`/`minHeight`; anything smaller is clamped by the window
// itself, so it cannot be a real layout. Each entry is a real WebKitGTK window resize, not a
// viewport emulation, so what is audited is what the desktop client paints.
const SIZES = [
  { name: "wide", width: 1440, height: 900 },
  { name: "laptop", width: 1366, height: 768 },
  { name: "default", width: 1280, height: 820 },
  { name: "shared", width: 1180, height: 740 },
  { name: "floor", width: 1100, height: 700 },
];
// Mirrors `SessionTabId`; the buttons carry `aria-controls`, which does not move with the locale.
const SESSION_TABS = ["chat", "changes", "documents", "files", "terminal", "shell", "logs", "traces", "report"];

const resultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR ?? tmpdir();
const shots = join(resultDir, "ui-ratios");
const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();
const findings = [];
const audited = [];

/**
 * Layout defects a screenshot alone would not flag reliably: the document scrolling sideways, an
 * interactive control that ends up outside the window, an element whose box extends past its
 * panel, and a terminal grid taller than the host that clips it.
 *
 * Text overflow inside a truncating element is deliberately not counted: `truncate` is a
 * legitimate design choice, and every truncated title would otherwise read as a defect.
 */
async function auditLayout(screen) {
  const report = await globalThis.browser.execute(() => {
    const issues = [];
    const width = globalThis.innerWidth;
    const height = globalThis.innerHeight;
    const doc = globalThis.document.documentElement;
    if (doc.scrollWidth > width + 1) issues.push(`document scrolls horizontally: ${doc.scrollWidth} > ${width}`);
    if (doc.scrollHeight > height + 1) {
      // Name the elements that push the document past the window so the finding is actionable.
      const clipped = (element) => {
        for (let node = element.parentElement; node; node = node.parentElement) {
          const overflow = globalThis.getComputedStyle(node).overflowY;
          if (overflow === "hidden" || overflow === "auto" || overflow === "scroll" || overflow === "clip") return true;
        }
        return false;
      };
      const culprits = [...globalThis.document.body.querySelectorAll("*")]
        .filter((element) => element.getBoundingClientRect().bottom > height + 1 && !clipped(element))
        .filter((element) => ![...element.children].some((child) => child.getBoundingClientRect().bottom > height + 1))
        .slice(0, 4)
        .map((element) => `${element.tagName.toLowerCase()}.${element.className.toString().split(" ").slice(0, 4).join(".")}@${Math.round(element.getBoundingClientRect().bottom)}`);
      issues.push(`document scrolls vertically: ${doc.scrollHeight} > ${height} [${culprits.join(" | ")}]`);
    }

    const visible = (element) => {
      const style = globalThis.getComputedStyle(element);
      if (style.display === "none" || style.visibility === "hidden") return false;
      const box = element.getBoundingClientRect();
      return box.width > 0 && box.height > 0;
    };
    const describe = (element) =>
      element.getAttribute("aria-label") ?? element.textContent?.trim().slice(0, 40) ?? element.tagName;

    const controls = [...globalThis.document.querySelectorAll("button, input, select, textarea, [role='tab']")]
      .filter(visible);
    for (const control of controls) {
      const box = control.getBoundingClientRect();
      // A control that is partly hidden by a scrolling ancestor is fine; one that lies outside the
      // window with no scrolling ancestor is unreachable.
      const scrollable = control.closest("[style*='overflow'], .overflow-auto, .overflow-y-auto, .overflow-x-auto, .ucd-scroll-strip");
      if (scrollable) continue;
      if (box.left < -1 || box.top < -1 || box.right > width + 1 || box.bottom > height + 1) {
        issues.push(`control outside window: "${describe(control)}" ${Math.round(box.left)},${Math.round(box.top)} ${Math.round(box.width)}x${Math.round(box.height)}`);
      }
    }

    const panel = globalThis.document.querySelector("[role='tabpanel']:not(.hidden)");
    if (panel) {
      const panelBox = panel.getBoundingClientRect();
      for (const element of panel.querySelectorAll("section, aside, form, [role='toolbar']")) {
        if (!visible(element)) continue;
        const box = element.getBoundingClientRect();
        if (box.right > panelBox.right + 2 || box.left < panelBox.left - 2) {
          issues.push(`panel child overflows horizontally: ${element.tagName.toLowerCase()}.${element.className.toString().split(" ").slice(0, 3).join(".")} ${Math.round(box.left)}..${Math.round(box.right)} vs ${Math.round(panelBox.left)}..${Math.round(panelBox.right)}`);
        }
      }
    }

    // The session list and the conversation are neighbours in a grid; a row that reaches past the
    // sidebar's edge, or a conversation shell that starts before it, is one covering the other.
    const sidebar = globalThis.document.querySelector(".ucd-session-sidebar-shell");
    const conversation = globalThis.document.querySelector(".ucd-conversation-shell");
    if (sidebar && conversation && visible(sidebar)) {
      const sidebarBox = sidebar.getBoundingClientRect();
      const conversationBox = conversation.getBoundingClientRect();
      if (conversationBox.left < sidebarBox.right - 1) {
        issues.push(`conversation shell starts at ${Math.round(conversationBox.left)} before the session sidebar ends at ${Math.round(sidebarBox.right)}`);
      }
      for (const row of sidebar.querySelectorAll("[data-session-id]")) {
        const box = row.getBoundingClientRect();
        if (box.right > sidebarBox.right + 1) {
          issues.push(`session row reaches ${Math.round(box.right)} past the sidebar edge ${Math.round(sidebarBox.right)}`);
          break;
        }
      }
    }

    // A virtual list with items but no rendered rows has measured a zero-height viewport; it looks
    // like an empty panel and no other check would see it.
    for (const list of globalThis.document.querySelectorAll("[data-virtual-count]")) {
      if (!visible(list.parentElement ?? list)) continue;
      if (Number(list.dataset.virtualCount) > 0 && Number(list.dataset.renderedCount) === 0) {
        issues.push(`virtual list renders none of its ${list.dataset.virtualCount} rows: "${list.getAttribute("aria-label")}"`);
      }
    }

    for (const host of globalThis.document.querySelectorAll(".ucd-agent-terminal, .ucd-shell-terminal")) {
      if (!visible(host)) continue;
      const screenElement = host.querySelector(".xterm-screen");
      if (!screenElement) continue;
      const hostBox = host.getBoundingClientRect();
      const screenBox = screenElement.getBoundingClientRect();
      if (screenBox.bottom > hostBox.bottom + 1 || screenBox.right > hostBox.right + 1) {
        issues.push(`terminal grid exceeds its host: screen ${Math.round(screenBox.width)}x${Math.round(screenBox.height)} host ${Math.round(hostBox.width)}x${Math.round(hostBox.height)}`);
      }
    }
    return { size: `${width}x${height}`, issues };
  });
  audited.push(`${screen} @${report.size}`);
  for (const issue of report.issues) findings.push(`${screen} @${report.size}: ${issue}`);
}

/**
 * WebKitWebDriver's window rect is in physical pixels, so on a HiDPI display a request in CSS
 * pixels lands at half the intended size and gets clamped to the window's minimum. Scaling by
 * the page's device pixel ratio and verifying the viewport makes every audited size the one that
 * was asked for; the audit records the size it actually saw either way.
 */
async function resizeWindow(size) {
  const ratio = await globalThis.browser.execute(() => globalThis.devicePixelRatio || 1);
  for (let attempt = 0; attempt < 4; attempt += 1) {
    await globalThis.browser.setWindowSize(Math.round(size.width * ratio), Math.round(size.height * ratio));
    await globalThis.browser.pause(500);
    const inner = await globalThis.browser.execute(() => [globalThis.innerWidth, globalThis.innerHeight]);
    // The requested height is the outer window, so the viewport comes back shorter by the
    // window manager's title bar; the width lands exactly.
    if (Math.abs(inner[0] - size.width) <= 8 && size.height - inner[1] >= 0 && size.height - inner[1] <= 48) return inner;
  }
  const inner = await globalThis.browser.execute(() => [globalThis.innerWidth, globalThis.innerHeight]);
  findings.push(`${size.name}: window settled at ${inner[0]}x${inner[1]} instead of ${size.width}x${size.height}`);
  return inner;
}

async function capture(name) {
  await globalThis.browser.saveScreenshot(join(shots, `${name}.png`));
}

async function waitForBootstrap() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
  );
}

async function createSweepSession() {
  await mkdir(fixtureRoot, { recursive: true });
  const repository = await mkdtemp(join(fixtureRoot, "ratios-"));
  await run("git", ["init"], { cwd: repository });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: repository });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: repository });
  await writeFile(join(repository, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: repository });
  await run("git", ["commit", "-m", "fixture"], { cwd: repository });

  const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
  const cli = agents.find((agent) => agent.availabilityState === "available"
    && agent.supportedInteractionModes.includes("cli"));
  assert.ok(cli, "the ratio sweep needs one installed CLI Agent");

  const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
    agentId: cli.id,
    interactionMode: "cli",
    title: "UI ratio sweep session with a deliberately long title to size the sidebar rows",
    folder: repository,
    projectPath: repository,
    remoteWorkspace: null,
    worktree: null,
  });
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 30_000, timeoutMsg: "Session creation never settled." });
  assert.equal(settled.status, "succeeded", settled.error ?? "session creation failed");

  const session = await globalThis.browser.waitUntil(async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    return sessions.find((item) => item.title === "UI ratio sweep session with a deliberately long title to size the sidebar rows") ?? false;
  }, { timeout: 30_000, timeoutMsg: "The sweep session was not created." });
  // The list was loaded before the session existed; see screen-sweep for why a reload is needed.
  await globalThis.browser.refresh();
  await waitForBootstrap();
  return session;
}

globalThis.describe("VaneHub AI desktop UI ratio sweep", () => {
  globalThis.before(async () => {
    await mkdir(shots, { recursive: true });
    await globalThis.browser.refresh();
    await waitForBootstrap();
  });

  globalThis.it("keeps every session workspace tab laid out at every window size", async function () {
    this.timeout(600_000);
    const session = await createSweepSession();
    await navigateTo(`/workspace/sessions/${encodeURIComponent(session.id)}`);
    const firstTab = await globalThis.$('[aria-controls="session-tab-panel-chat"]');
    await firstTab.waitForExist({ timeout: 30_000 });

    for (const size of SIZES) {
      await resizeWindow(size);
      for (const tab of SESSION_TABS) {
        const button = await globalThis.$(`[aria-controls="session-tab-panel-${tab}"]`);
        await button.waitForExist({ timeout: 20_000 });
        // The tab strip scrolls in a narrow window; bring the button into view before clicking.
        await button.scrollIntoView({ block: "nearest", inline: "nearest" });
        await button.click();
        const panel = await globalThis.$(`#session-tab-panel-${tab}`);
        await panel.waitForExist({ timeout: 30_000 });
        await globalThis.browser.waitUntil(
          async () => (await button.getAttribute("aria-selected")) === "true",
          { timeout: 20_000, timeoutMsg: `The ${tab} tab never became selected at ${size.name}.` },
        );
        // Lazy panels and xterm both settle after the click; a fixed pause is the honest wait
        // here because neither reports completion.
        await globalThis.browser.pause(900);
        await auditLayout(`session-tab/${tab}`);
        await capture(`${size.name}-${size.width}x${size.height}-${tab}`);
      }
    }
  });

  globalThis.it("keeps Basic Configuration laid out at every window size", async function () {
    this.timeout(300_000);
    await navigateTo("/settings");
    const sidebar = await globalThis.$("nav");
    await sidebar.waitForExist({ timeout: 20_000 });
    for (const size of SIZES) {
      await resizeWindow(size);
      const basic = (await globalThis.$$("nav button"))[0];
      await basic.click();
      await globalThis.browser.pause(600);
      await auditLayout("settings/basic");
      await capture(`${size.name}-${size.width}x${size.height}-settings-basic`);
    }
    await resizeWindow({ name: "restore", width: 1280, height: 820 }).catch(() => {});
  });

  globalThis.after(async () => {
    await writeFile(join(shots, "audit.json"), JSON.stringify({ audited, findings }, null, 2), "utf8");
    globalThis.console.log(`Audited ${audited.length} screens; findings: ${findings.length}`);
    for (const finding of findings) globalThis.console.log(`  - ${finding}`);
    await navigateTo("/workspace/sessions");
    await invoke(({ core }) => core.invoke("exit_application"));
    assert.equal(findings.length, 0, `layout defects found:\n${findings.join("\n")}`);
  });
});
