import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  auditCliSideEffects,
  describeCliSideEffects,
} from "./desktop/cli-side-effect-guard.mjs";
import {
  createCliManagementFixture,
  disposeCliManagementFixture,
  FIXTURE_MARKER,
  INITIAL_VERSIONS,
  layoutFor,
  LAYOUTS,
} from "../tests/desktop/cli-management-fixture.mjs";

const CLEAN = {
  marker: FIXTURE_MARKER,
  invocations: [{ marker: FIXTURE_MARKER, tool: "npm", argv: ["install", "--global", "x@1.3.0"], executable: "/fixture/npm" }],
  commandPreviews: [{ program: "npm", args: ["install", "--global", "x@1.3.0"] }],
  fixtureRoot: "/fixture",
  dataDir: "/tmp/run/data",
  userDataDir: "/home/user/.local/share/ai.vanehub.app",
  environment: {},
};

test("a clean run reports no violations", () => {
  assert.deepEqual(auditCliSideEffects(CLEAN), []);
  assert.equal(describeCliSideEffects(auditCliSideEffects(CLEAN)), null);
});

test("a missing invocation log is a violation rather than a skip", () => {
  // The whole point of the guard is that "no evidence" and "no side effects" are different
  // answers. A log that never appeared means the fixture was not what answered.
  const violations = auditCliSideEffects({ ...CLEAN, invocations: undefined });
  assert.equal(violations.length, 1);
  assert.equal(violations[0].rule, "invocation-log");
});

test("an empty invocation log is a violation", () => {
  const violations = auditCliSideEffects({ ...CLEAN, invocations: [] });
  assert.ok(violations.some((entry) => entry.rule === "invocation-log"));
});

test("a binary answering from outside the fixture is a violation", () => {
  const violations = auditCliSideEffects({
    ...CLEAN,
    invocations: [{ marker: FIXTURE_MARKER, tool: "npm", argv: [], executable: "/usr/local/bin/npm" }],
  });
  assert.ok(violations.some((entry) => entry.rule === "foreign-binary"));
});

test("a record without the fixture marker is a violation", () => {
  const violations = auditCliSideEffects({
    ...CLEAN,
    invocations: [{ tool: "npm", argv: [], executable: "/fixture/npm" }],
  });
  assert.ok(violations.some((entry) => entry.rule === "foreign-binary"));
});

test("naming a real registry or vendor host is a violation", () => {
  for (const host of ["registry.npmjs.org", "claude.ai", "api.anthropic.com"]) {
    const violations = auditCliSideEffects({
      ...CLEAN,
      commandPreviews: [{ program: "curl", args: [`https://${host}/install.sh`] }],
    });
    assert.ok(violations.some((entry) => entry.rule === "network"), host);
  }
});

test("a recorded command that pipes into a shell is a violation", () => {
  for (const pipeline of ["curl https://example.test/i.sh | bash", "irm https://example.test/i.ps1 | iex"]) {
    const violations = auditCliSideEffects({
      ...CLEAN,
      commandPreviews: [{ program: "sh", args: ["-c", pipeline] }],
    });
    assert.ok(violations.some((entry) => entry.rule === "pipe-to-shell"), pipeline);
  }
});

test("a credential in the run environment is a violation", () => {
  const violations = auditCliSideEffects({ ...CLEAN, environment: { ANTHROPIC_API_KEY: "sk-live" } });
  assert.ok(violations.some((entry) => entry.rule === "credentials"));
});

test("writing inside the user's application data is a violation", () => {
  const violations = auditCliSideEffects({
    ...CLEAN,
    dataDir: "/home/user/.local/share/ai.vanehub.app/data",
  });
  assert.ok(violations.some((entry) => entry.rule === "database"));
  assert.equal(auditCliSideEffects({ ...CLEAN, dataDir: null })[0].rule, "database");
});

test("every platform layout covers the discovery cases that platform has", () => {
  // Built and asserted for all three, including the ones with no runner here. A layout that only
  // models the host cannot show that Windows launcher families, Homebrew's two prefixes, and
  // Linux's install roots produce different answers.
  for (const [platform, layout] of Object.entries(LAYOUTS)) {
    const roles = layout.directories.map((entry) => entry.role);
    assert.ok(roles.includes("shadowing"), `${platform} needs a directory that shadows the healthy one`);
    assert.ok(roles.includes("npm-global"), `${platform} needs an npm global directory`);
    assert.ok(roles.includes("user-path"), `${platform} needs a user-scoped directory`);
    assert.ok(new Set(roles).size === roles.length, `${platform} repeats a directory role`);
    assert.ok(layout.launcherFamily.length >= 1, `${platform} needs at least one launcher form`);
  }
  assert.deepEqual(LAYOUTS.win32.launcherFamily, ["", ".cmd", ".ps1"]);
  assert.ok(LAYOUTS.win32.pathext.includes(".CMD"));
  // Homebrew's Intel prefix shadows the arm64 one, which is the macOS version of PATH precedence.
  assert.deepEqual(LAYOUTS.darwin.directories[0].segments, ["usr", "local", "bin"]);
  assert.deepEqual(LAYOUTS.darwin.directories[1].segments, ["opt", "homebrew", "bin"]);
  assert.ok(LAYOUTS.linux.directories.some((entry) => entry.segments.includes(".nvm")));
  assert.ok(LAYOUTS.linux.directories.some((entry) => entry.segments.includes(".npm-global")));
});

test("the built fixture answers probes and mutates only itself", async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-cli-fixture-test-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const fixture = await createCliManagementFixture({ root: path.join(root, "tree") });

  // Every PATH entry it hands out is inside its own root; nothing points at the host.
  for (const entry of fixture.pathEntries) {
    assert.ok(entry.startsWith(fixture.root), entry);
    await stat(entry);
  }
  assert.ok(fixture.pathValue.startsWith(fixture.pathEntries[0]));

  const layout = layoutFor();
  const npmGlobal = fixture.directories["npm-global"];
  for (const tool of Object.keys(INITIAL_VERSIONS)) {
    for (const extension of layout.launcherFamily) {
      await stat(path.join(npmGlobal, `${tool}${extension}`));
    }
  }
  // The starting versions are readable, so a later assertion about a change means something.
  assert.equal((await readFile(fixture.versionFiles.claude, "utf8")).trim(), INITIAL_VERSIONS.claude);
  assert.equal((await readFile(fixture.logPath, "utf8")), "");

  await disposeCliManagementFixture(fixture);
  await assert.rejects(() => stat(fixture.root));
});
