import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, extname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { repositoryRoot } from "./docs-tooling.mjs";

const markdownRoots = [
  resolve(repositoryRoot, "README.md"),
  resolve(repositoryRoot, "README.zh-CN.md"),
  resolve(repositoryRoot, "README.ja.md"),
  resolve(repositoryRoot, "docs"),
];

function markdownFiles(path) {
  if (!existsSync(path)) return [];
  if (statSync(path).isFile()) return extname(path).toLowerCase() === ".md" ? [path] : [];
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) =>
    markdownFiles(resolve(path, entry.name)),
  );
}

function splitTarget(target) {
  const decoded = decodeURIComponent(target.replace(/^<|>$/g, ""));
  const hash = decoded.indexOf("#");
  const path = (hash === -1 ? decoded : decoded.slice(0, hash)).split("?", 1)[0];
  return { path, fragment: hash === -1 ? "" : decoded.slice(hash + 1) };
}

/**
 * mdBook derives a heading id by keeping alphanumerics, `_`, `-`, and spaces, lowercasing,
 * and turning spaces into `-`; everything else is dropped. So `Plan-Agent` keeps its hyphen
 * and `Fidelity: why…` loses its colon — a link that guesses either way lands at the top of
 * the page with nothing to indicate it missed.
 */
export function normalizeHeadingId(text) {
  let id = "";
  for (const character of text) {
    if (/[\p{L}\p{N}]/u.test(character) || character === "_" || character === "-") {
      id += character.toLowerCase();
    } else if (character === " ") {
      id += "-";
    }
  }
  return id;
}

const headingIdCache = new Map();

/** Heading ids of one Markdown file, with mdBook's `-1`, `-2` suffixes for repeats. */
export function headingIds(file, content = undefined) {
  if (content === undefined && headingIdCache.has(file)) return headingIdCache.get(file);
  const ids = new Set();
  const seen = new Map();
  let inFence = false;
  for (const line of (content ?? readFileSync(file, "utf8")).split("\n")) {
    if (line.trimStart().startsWith("```")) {
      inFence = !inFence;
      continue;
    }
    if (inFence) continue;
    const heading = /^#{1,6}\s+(.*?)\s*$/.exec(line);
    if (!heading) continue;
    // Inline markup is not part of the id, so strip links, code spans, and emphasis first.
    const text = heading[1].replace(/\[([^\]]*)]\([^)]*\)/g, "$1").replace(/[`*]/g, "");
    const base = normalizeHeadingId(text);
    const repeats = seen.get(base) ?? 0;
    seen.set(base, repeats + 1);
    ids.add(repeats === 0 ? base : `${base}-${repeats}`);
  }
  if (content === undefined) headingIdCache.set(file, ids);
  return ids;
}

function resolveAuthoredTarget(file, target) {
  const isDeveloperGuide = file.includes(`${sep}docs${sep}developer-guide${sep}`);
  if (isDeveloperGuide && (target.startsWith("../api/") || target.startsWith("../../api/"))) {
    return null;
  }
  if (
    isDeveloperGuide &&
    (target === "../reference/release-signing.md" || target === "../../reference/release-signing.md")
  ) {
    return resolve(repositoryRoot, "docs", "release-signing.md");
  }
  if (
    isDeveloperGuide &&
    (target === "../reference/native-architecture.md" || target === "../../reference/native-architecture.md")
  ) {
    return resolve(repositoryRoot, "src-tauri", "ARCHITECTURE.md");
  }
  // Cross-book links are authored as repository-relative Markdown paths and rewritten to site
  // paths when the books are built, so ordinary resolution is all that is needed here. An
  // assembled-site path authored in source is a broken link, not a shape to compensate for.
  return resolve(dirname(file), target);
}

function validateMarkdown(errors) {
  for (const file of markdownRoots.flatMap(markdownFiles)) {
    const content = readFileSync(file, "utf8");
    const display = relative(repositoryRoot, file);
    for (const match of content.matchAll(/(!?)\[([^\]]*)]\(([^)\s]+)(?:\s+"[^"]*")?\)/g)) {
      const [, imageMarker, alt, rawTarget] = match;
      if (imageMarker === "!" && alt.trim().length === 0) {
        errors.push(`${display}: image "${rawTarget}" has empty alternative text.`);
      }
      if (/^(?:https?:|mailto:|data:)/i.test(rawTarget)) continue;
      const { path: target, fragment } = splitTarget(rawTarget);
      if (!target && !fragment) continue;
      // An empty path with a fragment is a link into the same document.
      const resolved = target ? resolveAuthoredTarget(file, target) : file;
      if (resolved === null) continue;
      if (!existsSync(resolved)) {
        errors.push(`${display}: missing relative target "${rawTarget}".`);
        continue;
      }
      if (
        fragment
        && extname(resolved).toLowerCase() === ".md"
        && !headingIds(resolved).has(normalizeHeadingId(fragment))
      ) {
        errors.push(
          `${display}: "${rawTarget}" names no heading in ${relative(repositoryRoot, resolved)}.`,
        );
      }
    }
  }
}

// CommonMark 0.30 punctuation: the ASCII set plus every Unicode P category.
const ASCII_PUNCTUATION = new Set([..."!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"]);

function characterClass(character) {
  // Start and end of line behave like whitespace for flanking purposes.
  if (character === undefined) return "whitespace";
  if (/\s/u.test(character)) return "whitespace";
  if (ASCII_PUNCTUATION.has(character) || /\p{P}/u.test(character)) return "punctuation";
  return "other";
}

function codeSpanRanges(line) {
  return [...line.matchAll(/(`+)[^`]*\1/g)].map((match) => [
    match.index,
    match.index + match[0].length,
  ]);
}

/**
 * A `**` run can only close strong emphasis when it is right-flanking: the character
 * before it is not whitespace, and either that character is not punctuation or the
 * character after it is whitespace or punctuation.
 *
 * Chinese prose breaks this constantly, because the sentence-final `。` or `：` sits
 * inside the bold span and the next sentence starts with a letter. GitHub then either
 * prints the asterisks literally or re-pairs the delimiters around the wrong words, and
 * neither failure is visible in the source.
 */
export function unclosableEmphasis(line) {
  const spans = codeSpanRanges(line);
  const inCodeSpan = (index) => spans.some(([from, to]) => index >= from && index < to);
  let open = false;

  for (const match of line.matchAll(/\*+/g)) {
    // Only plain `**` runs; `*` and `***` carry other meanings and stay out of scope.
    if (match[0].length !== 2 || inCodeSpan(match.index)) continue;
    const before = characterClass(line[match.index - 1]);
    const after = characterClass(line[match.index + 2]);
    const canOpen = after !== "whitespace" && (after !== "punctuation" || before !== "other");
    const canClose =
      before !== "whitespace" && (before !== "punctuation" || after !== "other");

    if (!open) {
      open = canOpen;
    } else if (canClose) {
      open = false;
    } else {
      return line.slice(Math.max(0, match.index - 12), match.index + 14);
    }
  }
  return null;
}

function validateEmphasis(errors) {
  for (const file of markdownRoots.flatMap(markdownFiles)) {
    const display = relative(repositoryRoot, file);
    let fenced = false;
    readFileSync(file, "utf8")
      .split(/\r?\n/)
      .forEach((line, index) => {
        if (/^\s*(?:```|~~~)/.test(line)) fenced = !fenced;
        if (fenced || !line.includes("**")) return;
        const excerpt = unclosableEmphasis(line);
        if (excerpt) {
          errors.push(
            `${display}:${index + 1}: bold cannot close after punctuation in "${excerpt}". ` +
              "Move the punctuation outside the ** span.",
          );
        }
      });
  }
}

function validateScreenshotInventory(errors) {
  const inventoryPath = resolve(repositoryRoot, "docs", "user-guide", "screenshots.json");
  if (!existsSync(inventoryPath)) {
    errors.push("docs/user-guide/screenshots.json: screenshot inventory is missing.");
    return;
  }
  const inventory = JSON.parse(readFileSync(inventoryPath, "utf8"));
  const seen = new Set();
  for (const item of inventory.screenshots ?? []) {
    if (!item.id || seen.has(item.id)) errors.push(`Screenshot id "${item.id ?? ""}" is missing or duplicated.`);
    seen.add(item.id);
    if (!["web-mock", "desktop-reviewed"].includes(item.runtime)) {
      errors.push(`Screenshot "${item.id}" has unsupported runtime "${item.runtime}".`);
    }
    const asset = resolve(repositoryRoot, "docs", "user-guide", item.path ?? "");
    if (!existsSync(asset)) errors.push(`Screenshot "${item.id}" is missing asset "${item.path}".`);
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function symbolDeclarationLine(content, symbol) {
  const pattern = new RegExp(
    `\\b(?:pub(?:\\([^)]*\\))?\\s+)?(?:const|enum|fn|static|struct|trait|type)\\s+${escapeRegExp(symbol)}\\b`,
  );
  const lines = content.split(/\r?\n/);
  const index = lines.findIndex((line) => pattern.test(line));
  return { index, lines };
}

export function hasDocumentedSymbol(content, symbol) {
  const { index, lines } = symbolDeclarationLine(content, symbol);
  if (index < 0) return false;

  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const line = lines[cursor].trim();
    if (!line) return false;
    if (line.startsWith("///")) return true;
    if (line.startsWith("#[")) continue;
    return false;
  }
  return false;
}

export function validateNativeBoundaryContent(item, content) {
  const errors = [];
  if (item.moduleDoc && !/^\s*\/\/!/m.test(content)) {
    errors.push(`Native documentation boundary lacks module documentation: "${item.path}".`);
  }
  for (const symbol of item.symbols ?? []) {
    const { index } = symbolDeclarationLine(content, symbol);
    if (index < 0) {
      errors.push(`Native documentation boundary symbol is missing: "${item.path}#${symbol}".`);
    } else if (!hasDocumentedSymbol(content, symbol)) {
      errors.push(`Native documentation boundary symbol lacks Rust documentation: "${item.path}#${symbol}".`);
    }
  }
  return errors;
}

function validateNativeBoundaries(errors) {
  const inventoryPath = resolve(repositoryRoot, "docs", "developer-guide", "native-boundaries.json");
  if (!existsSync(inventoryPath)) {
    errors.push("docs/developer-guide/native-boundaries.json: native documentation inventory is missing.");
    return;
  }
  const inventory = JSON.parse(readFileSync(inventoryPath, "utf8"));
  for (const item of inventory.boundaries ?? []) {
    const path = resolve(repositoryRoot, item.path);
    if (!existsSync(path)) {
      errors.push(`Native documentation boundary is missing: "${item.path}".`);
      continue;
    }
    const content = readFileSync(path, "utf8");
    if (!Array.isArray(item.symbols) || item.symbols.length === 0) {
      errors.push(`Native documentation boundary has no selected symbols: "${item.path}".`);
      continue;
    }
    errors.push(...validateNativeBoundaryContent(item, content));
  }
}

export function boundedContextDrift(documented, actual) {
  const undocumented = actual.filter((name) => !documented.includes(name));
  const stale = documented.filter((name) => !actual.includes(name));
  return { stale, undocumented };
}

/**
 * Context names from every `| \`name\` |` row of a guide chapter, wherever those rows sit.
 *
 * The chapter groups its contexts under several headings and carries a second table of
 * facades that repeats a subset of them, so this collects across the whole file and dedupes
 * rather than reading one table the way the standards file is read. Anything the chapter
 * mentions in prose stays out: only a leading table cell counts as documenting a context.
 */
export function chapterBoundedContexts(chapter) {
  const names = new Set();
  for (const line of chapter.split(/\r?\n/)) {
    const row = line.trim().match(/^\|\s*`([a-z_]+)`\s*\|/);
    if (row) names.add(row[1]);
  }
  return [...names].sort();
}

/** Line-scanned rather than sliced by regex, because the standards file uses CRLF endings. */
export function documentedBoundedContexts(standards) {
  const lines = standards.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === "### Bounded contexts");
  if (start < 0) return [];
  const names = [];
  for (let cursor = start + 1; cursor < lines.length; cursor += 1) {
    const line = lines[cursor].trim();
    if (line.startsWith("### ")) break;
    const row = line.match(/^\|\s*`([a-z_]+)`\s*\|/);
    if (row) names.push(row[1]);
  }
  return names.sort();
}

/**
 * `native-runtime-architecture` requires the project standards to document the bounded-context
 * map, but nothing checked that they still matched. The table had drifted to seven rows while
 * `src-tauri/src/contexts/` held fifteen, so eight contexts had no documented owner.
 */
function validateBoundedContexts(errors) {
  const contextsPath = resolve(repositoryRoot, "src-tauri", "src", "contexts");
  const standardsPath = resolve(repositoryRoot, "openspec", "project.md");
  if (!existsSync(contextsPath) || !existsSync(standardsPath)) {
    errors.push("openspec/project.md: cannot verify the bounded-context map; a required path is missing.");
    return;
  }
  const actual = readdirSync(contextsPath, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  const documented = documentedBoundedContexts(readFileSync(standardsPath, "utf8"));
  if (documented.length === 0) {
    errors.push("openspec/project.md: the Bounded contexts table could not be read.");
    return;
  }

  const { stale, undocumented } = boundedContextDrift(documented, actual);
  for (const name of undocumented) {
    errors.push(`openspec/project.md: bounded context "${name}" exists in src-tauri/src/contexts/ but has no row in the Bounded contexts table.`);
  }
  for (const name of stale) {
    errors.push(`openspec/project.md: bounded context "${name}" is documented but has no directory in src-tauri/src/contexts/.`);
  }

  validateContextMapChapters(errors, actual);
}

/**
 * The developer guide's context map is the chapter a contributor opens to find where code
 * lives, and nothing checked it. It had drifted to eight rows against twenty-one directories,
 * so thirteen subsystems — browser automation, sandboxed execution, SSH, goals — were absent
 * from the map that claims to be complete.
 *
 * Only the chapters listed here are enforced. A translation joins the list once it carries the
 * full map, so an untranslated guide fails review rather than CI.
 */
const enforcedContextMapChapters = ["docs/developer-guide/zh-CN/src/native-contexts.md"];

function validateContextMapChapters(errors, actual) {
  for (const relative of enforcedContextMapChapters) {
    const path = resolve(repositoryRoot, relative);
    if (!existsSync(path)) {
      errors.push(`${relative}: the bounded-context map chapter is missing.`);
      continue;
    }
    const documented = chapterBoundedContexts(readFileSync(path, "utf8"));
    if (documented.length === 0) {
      errors.push(`${relative}: no bounded-context table rows could be read.`);
      continue;
    }
    const { stale, undocumented } = boundedContextDrift(documented, actual);
    for (const name of undocumented) {
      errors.push(`${relative}: bounded context "${name}" exists in src-tauri/src/contexts/ but has no row in the context map.`);
    }
    for (const name of stale) {
      errors.push(`${relative}: bounded context "${name}" is mapped but has no directory in src-tauri/src/contexts/.`);
    }
  }
}

function validateAssembled(errors) {
  const expected = [
    ".docs-build/index.html",
    ".docs-build/developer/index.html",
    ".docs-build/user/en/index.html",
    ".docs-build/user/zh-CN/index.html",
    ".docs-build/api/vanehub_ai_lib/index.html",
  ];
  for (const path of expected) {
    if (!existsSync(resolve(repositoryRoot, path))) errors.push(`Assembled documentation entry is missing: ${path}.`);
  }
}

export function validateDocs({ assembled = false } = {}) {
  const errors = [];
  validateMarkdown(errors);
  validateEmphasis(errors);
  validateScreenshotInventory(errors);
  validateNativeBoundaries(errors);
  validateBoundedContexts(errors);
  if (assembled) validateAssembled(errors);
  if (errors.length > 0) throw new Error(`Documentation validation failed:\n${errors.join("\n")}`);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    validateDocs({ assembled: process.argv.includes("--assembled") });
    console.log("Documentation links, media, and boundary inventories verified.");
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
