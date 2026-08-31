import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import console from "node:console";
import process from "node:process";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/**
 * What each ecosystem's updater needs to find in the directory it is pointed at.
 *
 * A lockfile rather than a manifest wherever one exists: the manifest can sit in a member directory
 * whose every dependency is `workspace = true`, which is a manifest with nothing in it to bump. That
 * is the exact shape that made the cargo updater idle — `src-tauri/Cargo.toml` was there, so the
 * directory looked right.
 */
const ECOSYSTEM_MANIFESTS = Object.freeze({
  npm: ["package-lock.json", "package.json"],
  cargo: ["Cargo.lock"],
  "github-actions": [".github/workflows"],
});

/** A YAML reader for the shape this one file has, rather than a dependency for one call site. */
function parseEcosystems(yaml) {
  const entries = [];
  let current = null;
  for (const raw of yaml.split(/\r?\n/)) {
    const line = raw.replace(/#.*$/, "");
    const ecosystem = line.match(/^\s*-\s*package-ecosystem:\s*"?([\w-]+)"?\s*$/);
    if (ecosystem) {
      current = { ecosystem: ecosystem[1], directory: undefined };
      entries.push(current);
      continue;
    }
    const directory = line.match(/^\s*directory:\s*"?([^"\s]+)"?\s*$/);
    if (directory && current && current.directory === undefined) {
      current.directory = directory[1];
    }
  }
  return entries;
}

/**
 * Every configured ecosystem points at something its updater can read.
 *
 * The failure this catches is silent by construction: an updater pointed at a directory with no
 * lockfile opens no pull request and reports no error, so the repository looks maintained while one
 * ecosystem has not been updated at all. It went unnoticed here from the commit that created the
 * cargo workspace until somebody asked why there had never been a `deps(cargo)` pull request.
 */
export function checkDependencyUpdates(root = repositoryRoot) {
  const configuration = resolve(root, ".github", "dependabot.yml");
  if (!existsSync(configuration)) {
    throw new Error("`.github/dependabot.yml` is missing, so no ecosystem is updated at all.");
  }
  const entries = parseEcosystems(readFileSync(configuration, "utf8"));
  if (entries.length === 0) {
    throw new Error("`.github/dependabot.yml` configures no ecosystem.");
  }

  const problems = [];
  for (const { ecosystem, directory } of entries) {
    if (!directory) {
      problems.push(`\`${ecosystem}\` names no directory.`);
      continue;
    }
    const wanted = ECOSYSTEM_MANIFESTS[ecosystem];
    if (!wanted) {
      // Unknown to this check rather than wrong. Adding an ecosystem should not require editing
      // this file first, but it should not silently opt out of the check either.
      problems.push(
        `\`${ecosystem}\` is configured but this check does not know what its updater reads; ` +
          "add it to ECOSYSTEM_MANIFESTS.",
      );
      continue;
    }
    const base = resolve(root, `.${directory}`);
    const found = wanted.find((name) => existsSync(resolve(base, name)));
    if (!found) {
      problems.push(
        `\`${ecosystem}\` points at \`${directory}\`, which holds none of ${wanted.join(", ")}. ` +
          "The updater will run and produce nothing.",
      );
    }
  }

  if (problems.length > 0) {
    throw new Error(`Dependency update configuration is unusable:\n- ${problems.join("\n- ")}`);
  }
  return entries;
}

const invokedDirectly = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  try {
    const entries = checkDependencyUpdates();
    console.log(
      `Dependency update configuration verified for ${entries
        .map((entry) => entry.ecosystem)
        .join(", ")}.`,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
