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

function cleanTarget(target) {
  const decoded = decodeURIComponent(target.replace(/^<|>$/g, ""));
  return decoded.split("#", 1)[0].split("?", 1)[0];
}

function resolveAuthoredTarget(file, target) {
  if (
    file.includes(`${sep}docs${sep}user-guide${sep}`) &&
    target.startsWith("../assets/")
  ) {
    return resolve(repositoryRoot, "docs", "user-guide", target.slice("../".length));
  }
  if (
    file.includes(`${sep}docs${sep}developer-guide${sep}`) &&
    target.startsWith("../api/")
  ) {
    return null;
  }
  if (
    file.includes(`${sep}docs${sep}developer-guide${sep}`) &&
    target.startsWith("../reference/architecture/")
  ) {
    return resolve(
      repositoryRoot,
      "docs",
      "architecture",
      target.slice("../reference/architecture/".length),
    );
  }
  if (
    file.includes(`${sep}docs${sep}developer-guide${sep}`) &&
    target === "../reference/release-signing.md"
  ) {
    return resolve(repositoryRoot, "docs", "release-signing.md");
  }
  if (
    file.includes(`${sep}docs${sep}developer-guide${sep}`) &&
    target === "../reference/native-architecture.md"
  ) {
    return resolve(repositoryRoot, "src-tauri", "ARCHITECTURE.md");
  }
  // `docs/zh` links to the sibling books by their assembled-site paths, the same way
  // `developer-guide` links to `../reference/`. Map them back to the authored sources.
  if (file.includes(`${sep}docs${sep}zh${sep}`) && target === "../user/zh-CN/index.html") {
    return resolve(repositoryRoot, "docs", "user-guide", "zh-CN", "src", "index.md");
  }
  if (file.includes(`${sep}docs${sep}zh${sep}`) && target === "../developer/index.html") {
    return resolve(repositoryRoot, "docs", "developer-guide", "src", "index.md");
  }
  if (
    file.includes(`${sep}docs${sep}user-guide${sep}zh-CN${sep}`) &&
    target.startsWith("../../zh/")
  ) {
    return resolve(repositoryRoot, "docs", "zh", "src", "README.md");
  }
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
      if (/^(?:https?:|mailto:|#|data:)/i.test(rawTarget)) continue;
      const target = cleanTarget(rawTarget);
      if (!target) continue;
      const resolved = resolveAuthoredTarget(file, target);
      if (resolved && !existsSync(resolved)) {
        errors.push(`${display}: missing relative target "${rawTarget}".`);
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

function validateAssembled(errors) {
  const expected = [
    ".docs-build/index.html",
    ".docs-build/developer/index.html",
    ".docs-build/user/en/index.html",
    ".docs-build/user/zh-CN/index.html",
    ".docs-build/zh/index.html",
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
