import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { repositoryRoot } from "./docs-tooling.mjs";

const defaultFiles = ["README.md", "README.zh-CN.md", "README.ja.md"];
const requiredFactIds = [
  "project-version",
  "tauri-major",
  "react-major",
  "node-minimum",
];
const visibleFactPatterns = new Map([
  ["project-version", /badge\/version-([0-9]+\.[0-9]+\.[0-9]+)-/i],
  ["tauri-major", /badge\/Tauri-([0-9]+\.x)-/i],
  ["react-major", /badge\/React-([0-9]+\.x)-/i],
  ["node-minimum", /Node\.js\s+([0-9]+\+)/i],
]);

function collect(pattern, content, transform = (match) => match[1]) {
  return [...content.matchAll(pattern)].map(transform);
}

function normalizeRelativeLink(target) {
  const withoutAnchor = target.split("#", 1)[0];
  if (/^README(?:\.zh-CN|\.ja)?\.md$/i.test(withoutAnchor)) return "<README-LANGUAGE>";
  return withoutAnchor;
}

export function analyzeReadme(content) {
  const sections = collect(
    /<!--\s*docs-section:([a-z0-9-]+)\s*-->/g,
    content,
  );
  const commands = collect(
    /```(?:powershell|bash|shell)\r?\n([\s\S]*?)\r?\n```/g,
    content,
    (match) => match[1].replaceAll("\r\n", "\n").trim(),
  );
  const links = collect(
    /!?\[[^\]]*]\(([^)\s]+)(?:\s+"[^"]*")?\)/g,
    content,
  )
    .filter((target) => !/^(?:https?:|mailto:|#)/i.test(target))
    .map(normalizeRelativeLink)
    .sort();
  const facts = collect(
    /<!--\s*docs-fact:([a-z0-9-]+)\s+value:([^\s]+)\s*-->/g,
    content,
    (match) => `${match[1]}:${match[2]}`,
  );
  const featureStates = collect(
    /<!--\s*feature:([a-z0-9-]+)\s+status:([a-z-]+)\s*-->/g,
    content,
    (match) => `${match[1]}:${match[2]}`,
  );
  return { sections, commands, links, facts, featureStates };
}

function compareArray(label, canonicalName, canonical, candidateName, candidate, errors) {
  if (JSON.stringify(canonical) === JSON.stringify(candidate)) return;
  errors.push(
    `${candidateName}: ${label} differ from ${canonicalName}.\n` +
      `  expected: ${JSON.stringify(canonical)}\n` +
      `  received: ${JSON.stringify(candidate)}`,
  );
}

function majorVersion(range, dependency) {
  const match = String(range ?? "").match(/\d+/);
  if (!match) throw new Error(`Unable to determine the ${dependency} major version from package.json.`);
  return `${match[0]}.x`;
}

function manifestFacts(root) {
  const manifest = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8"));
  return new Map([
    ["project-version", manifest.version],
    ["react-major", majorVersion(manifest.dependencies?.react, "React")],
    [
      "tauri-major",
      majorVersion(
        manifest.dependencies?.["@tauri-apps/api"] ?? manifest.devDependencies?.["@tauri-apps/cli"],
        "Tauri",
      ),
    ],
  ]);
}

function validateFacts(file, content, facts, errors, expectedManifestFacts) {
  const entries = facts.map((fact) => {
    const separator = fact.indexOf(":");
    return [fact.slice(0, separator), fact.slice(separator + 1)];
  });
  const factMap = new Map(entries);
  if (factMap.size !== entries.length) {
    errors.push(`${file}: stable documentation facts contain duplicate ids.`);
  }

  for (const id of requiredFactIds) {
    if (!factMap.has(id)) errors.push(`${file}: missing stable documentation fact "${id}".`);
  }

  for (const [id, pattern] of visibleFactPatterns) {
    const visibleValue = content.match(pattern)?.[1];
    if (!visibleValue) {
      errors.push(`${file}: visible value for stable documentation fact "${id}" is missing.`);
    } else if (factMap.get(id) !== visibleValue) {
      errors.push(
        `${file}: visible fact "${id}" is "${visibleValue}", but its marker is "${factMap.get(id) ?? "missing"}".`,
      );
    }
  }

  if (expectedManifestFacts) {
    for (const [id, expected] of expectedManifestFacts) {
      if (factMap.get(id) !== expected) {
        errors.push(
          `${file}: stable fact "${id}" differs from package.json.\n` +
            `  expected: "${expected}"\n` +
            `  received: "${factMap.get(id) ?? "missing"}"`,
        );
      }
    }
  }
}

export function checkReadmeParity(files = defaultFiles, root = repositoryRoot) {
  const analyses = files.map((file) => {
    const path = resolve(root, file);
    const content = readFileSync(path, "utf8");
    return { file, content, analysis: analyzeReadme(content) };
  });
  const [canonical, ...translations] = analyses;
  const errors = [];

  if (canonical.analysis.sections.length === 0) {
    errors.push(`${canonical.file}: no docs-section markers found.`);
  }
  if (canonical.analysis.featureStates.length === 0) {
    errors.push(`${canonical.file}: no feature status markers found.`);
  }
  validateFacts(
    canonical.file,
    canonical.content,
    canonical.analysis.facts,
    errors,
    manifestFacts(root),
  );
  for (const translation of translations) {
    validateFacts(translation.file, translation.content, translation.analysis.facts, errors);
  }

  for (const translation of translations) {
    for (const key of ["sections", "commands", "links", "facts", "featureStates"]) {
      compareArray(
        key,
        canonical.file,
        canonical.analysis[key],
        translation.file,
        translation.analysis[key],
        errors,
      );
    }
  }

  if (errors.length > 0) {
    throw new Error(`README parity check failed:\n${errors.join("\n")}`);
  }
  return analyses;
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    checkReadmeParity();
    console.log(`README parity verified: ${defaultFiles.join(", ")}`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
