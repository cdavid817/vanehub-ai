import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);

// Only the fixture emits these, so matching them is what proves the layer drove the fixture and
// not a real CLI Agent that happened to be installed on the host.
const READY = "VANEHUB-FIXTURE-CLI READY";
const ECHO = "VANEHUB-FIXTURE-ECHO";
const PROBE = "round-trip-probe";
const ENTER = String.fromCharCode(13);

// Counted rather than matched exactly, and compared against a baseline taken before launch: a
// leftover fixture from an earlier aborted run would otherwise make this assertion unsatisfiable
// forever, failing the layer for a reason that has nothing to do with the terminal under test.
async function fixtureProcessCount() {
  const { stdout } = await run("ps", ["-eo", "args"]).catch(() => ({ stdout: "" }));
  return stdout.split("\n").filter((line) => line.includes("fixtures/cli/opencode")).length;
}

const CURSOR_POSITION_REPORT = "\u001b[6n";

async function answerCursorPositionReport(terminalId) {
  const asked = await globalThis.browser
    .waitUntil(
      async () =>
        (await globalThis.browser.execute(() => globalThis.__terminalOutput)).includes(
          CURSOR_POSITION_REPORT,
        ),
      { timeout: 10000 },
    )
    .then(() => true)
    .catch(() => false);
  if (!asked) return;
  await globalThis.browser.tauri.execute(
    ({ core }, input) => core.invoke("send_agent_terminal_input", input),
    { terminalId, content: "\u001b[1;1R" },
  );
}

globalThis.describe("VaneHub AI CLI Agent terminal round trip", () => {
  globalThis.it("starts, streams, accepts input, and stops a managed CLI Agent terminal", async function () {
    this.timeout(180000);
    const repository = await mkdtemp(join(tmpdir(), "vanehub-cli-terminal-"));
    await run("git", ["init"], { cwd: repository });
    await writeFile(join(repository, "readme.md"), "fixture", "utf8");

    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120000 });
    await globalThis.browser.waitUntil(async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready", {
      timeout: 120000,
      timeoutMsg: "React bootstrap did not become ready.",
    });

    // The Agent must resolve to the fixture through the ordinary availability probe; if the
    // fixture PATH did not reach the runtime the whole layer is meaningless.
    const agents = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const agent = agents.find((candidate) => candidate.id === "opencode");
    assert.ok(agent, "the builtin opencode Agent is missing from the registry");
    assert.equal(agent.availabilityState, "available", agent.unavailableReason ?? "fixture executable was not resolved");

    // `browser.execute` does not await the promise it is handed, and `event.listen` only completes
    // its subscription after an IPC round trip. Returning that promise therefore armed nothing:
    // whatever the terminal emitted before the round trip finished was dropped, and on a slow host
    // that is the first line the assertions wait for. The registration is now confirmed before the
    // terminal is opened.
    await globalThis.browser.execute(() => {
      globalThis.__terminalOutput = "";
      globalThis.__terminalStates = [];
      globalThis.__terminalListening = false;
      void globalThis.__TAURI__.event
        .listen("agent-terminal:event", (received) => {
          const payload = received.payload;
          if (payload.type === "output") globalThis.__terminalOutput += payload.content;
          else if (payload.type === "state") globalThis.__terminalStates.push(payload.state);
        })
        .then(() => {
          globalThis.__terminalListening = true;
        });
    });
    await globalThis.browser.waitUntil(
      async () => globalThis.browser.execute(() => globalThis.__terminalListening === true),
      { timeout: 30000, timeoutMsg: "The terminal event listener was never registered." },
    );

    await globalThis.browser.tauri.execute(({ core }, projectPath) => core.invoke("create_session", {
      input: {
        agentId: "opencode",
        interactionMode: "cli",
        title: "CLI terminal round trip",
        folder: projectPath,
        projectPath,
        remoteWorkspace: null,
        worktree: null,
      },
    }), repository);
    const session = await globalThis.browser.waitUntil(async () => {
      const sessions = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === "CLI terminal round trip") ?? false;
    }, { timeout: 30000, timeoutMsg: "The CLI session was not created." });

    const baselineProcesses = await fixtureProcessCount();
    const terminal = await globalThis.browser.tauri.execute(({ core }, sessionId) => core.invoke("open_agent_terminal", {
      sessionId,
      size: { cols: 100, rows: 30 },
    }), session.id);
    assert.equal(terminal.state, "running");
    assert.equal(terminal.capability, "native");

    const diagnose = async () => {
      const captured = await globalThis.browser.execute(() => ({
        output: globalThis.__terminalOutput,
        states: globalThis.__terminalStates,
        listening: globalThis.__terminalListening,
      }));
      return ` Listener armed: ${captured.listening}. States seen: ${JSON.stringify(captured.states)}. Output so far: ${JSON.stringify(captured.output.slice(-400))}.`;
    };
    const seen = async (needle) => globalThis.browser.waitUntil(
      async () => (await globalThis.browser.execute(() => globalThis.__terminalOutput)).includes(needle),
      { timeout: 30000, timeoutMsg: `The terminal never produced ${needle}.${await diagnose()}` },
    );

    // The wrapper runs under PowerShell, whose PSReadLine asks the terminal where the cursor is
    // (`ESC[6n`) and waits for the answer before it will run anything. The real UI is xterm.js and
    // answers automatically; this harness only accumulates strings, so nothing replied and the
    // fixture never got to start. Answering the report is emulating the terminal, not working
    // around the product.
    await answerCursorPositionReport(terminal.terminalId);

    await seen(READY);
    await globalThis.browser.tauri.execute(
      ({ core }, input) => core.invoke("send_agent_terminal_input", input),
      { terminalId: terminal.terminalId, content: `${PROBE}${ENTER}` },
    );
    await seen(`${ECHO} ${PROBE}`);

    const stopped = await globalThis.browser.tauri.execute(
      ({ core }, terminalId) => core.invoke("stop_agent_terminal", { terminalId }),
      terminal.terminalId,
    );
    assert.equal(stopped, true);
    await globalThis.browser.waitUntil(async () => await fixtureProcessCount() <= baselineProcesses, {
      timeout: 30000,
      timeoutMsg: "The fixture Agent process outlived its terminal.",
    });

    assert.equal(await root.getAttribute("data-vanehub-fatal-error"), null);
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });
});
