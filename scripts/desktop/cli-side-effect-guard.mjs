/**
 * Proves a CLI Management run touched nothing real.
 *
 * A desktop layer that drives install and upgrade is one PATH mistake away from running the
 * machine's actual package manager against the user's actual global prefix. That failure is silent
 * -- the test still passes, because the real npm also installs the package -- so the only way to
 * know is to check afterwards and refuse to report a pass without the evidence.
 *
 * Every rule here is stated as "what would have to be true if something real were touched", and
 * every one of them fails closed: a missing invocation log, an unreadable path, or a record the
 * fixture did not write is a violation, not a skip.
 */

/** Hosts a real vendor installer or registry would be fetched from. */
const FORBIDDEN_HOSTS = [
  "registry.npmjs.org",
  "claude.ai",
  "anthropic.com",
  "openai.com",
  "googleapis.com",
  "opencode.ai",
  "cdn.winget.microsoft.com",
  "api.anthropic.com",
];

/** Environment variables that would carry a real credential into a fake CLI. */
const FORBIDDEN_ENVIRONMENT = [
  "ANTHROPIC_API_KEY",
  "OPENAI_API_KEY",
  "GEMINI_API_KEY",
  "GOOGLE_API_KEY",
  "NPM_TOKEN",
  "GH_TOKEN",
  "GITHUB_TOKEN",
];

function violation(rule, detail) {
  return { rule, detail };
}

/**
 * Checks one run's evidence.
 *
 * `invocations` are the JSON lines the fixture's fakes appended, `commandPreviews` are the argv
 * the application recorded on its plans, and `paths` are the directories the run was confined to.
 */
export function auditCliSideEffects({
  marker,
  invocations,
  commandPreviews = [],
  fixtureRoot,
  dataDir,
  userDataDir,
  environment = {},
}) {
  const violations = [];

  if (!marker) violations.push(violation("fixture-marker", "no fixture marker was supplied"));
  if (!Array.isArray(invocations)) {
    // Fail closed: no log at all is indistinguishable from a run that never used the fixture.
    return [violation("invocation-log", "the fixture invocation log was missing or unreadable")];
  }
  if (invocations.length === 0) {
    violations.push(violation("invocation-log", "no fixture binary was invoked, so nothing proves the real ones were not"));
  }

  for (const record of invocations) {
    if (record.marker !== marker) {
      violations.push(violation("foreign-binary", `an invocation was recorded without the fixture marker: ${JSON.stringify(record).slice(0, 200)}`));
      continue;
    }
    if (fixtureRoot && typeof record.executable === "string" && !record.executable.startsWith(fixtureRoot)) {
      violations.push(violation("foreign-binary", `${record.tool} answered from outside the fixture: ${record.executable}`));
    }
    for (const argument of record.argv ?? []) {
      const lowered = String(argument).toLowerCase();
      for (const host of FORBIDDEN_HOSTS) {
        if (lowered.includes(host)) {
          violations.push(violation("network", `${record.tool} was asked to reach ${host}`));
        }
      }
    }
  }

  for (const preview of commandPreviews) {
    const text = [preview.program, ...(preview.args ?? [])].join(" ");
    const lowered = text.toLowerCase();
    for (const host of FORBIDDEN_HOSTS) {
      if (lowered.includes(host)) violations.push(violation("network", `a recorded command names ${host}: ${text}`));
    }
    // A shell pipeline is the shape this change exists to remove; seeing one here means a source
    // adapter regressed to piping a downloaded script into an interpreter.
    if (/\|\s*(bash|sh|iex|invoke-expression)/i.test(text)) {
      violations.push(violation("pipe-to-shell", `a recorded command pipes into a shell: ${text}`));
    }
  }

  for (const name of FORBIDDEN_ENVIRONMENT) {
    if (environment[name]) violations.push(violation("credentials", `${name} was present in the run environment`));
  }

  if (!dataDir) {
    violations.push(violation("database", "no isolated application data directory was reported"));
  } else if (userDataDir && normalize(dataDir).startsWith(normalize(userDataDir))) {
    violations.push(violation("database", `the run wrote inside the user's application data: ${dataDir}`));
  }

  return violations;
}

function normalize(value) {
  return process.platform === "win32" ? value.toLowerCase().replaceAll("\\", "/") : value;
}

/** Formats violations for a verification failure, or returns `null` when the run is clean. */
export function describeCliSideEffects(violations) {
  if (violations.length === 0) return null;
  return violations.map(({ rule, detail }) => `[${rule}] ${detail}`).join("\n");
}
