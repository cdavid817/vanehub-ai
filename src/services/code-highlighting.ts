import hljs from "highlight.js/lib/common";
import cmake from "highlight.js/lib/languages/cmake";
import dockerfile from "highlight.js/lib/languages/dockerfile";
import powershell from "highlight.js/lib/languages/powershell";
import scala from "highlight.js/lib/languages/scala";

// Four languages the common bundle omits that the mention allowlist admits — this
// repository ships .ps1 scripts and a Dockerfile itself. Registering them costs a few KB
// each; everything else outside the common set renders as plain text.
for (const [name, definition] of [
  ["cmake", cmake],
  ["dockerfile", dockerfile],
  ["powershell", powershell],
  ["scala", scala],
] as const) {
  hljs.registerLanguage(name, definition);
}

export interface HighlightedLine {
  /** 1-based position in the source file, matching what prompt injection labels. */
  number: number;
  /** highlight.js output for this line. Text is already escaped by highlight.js. */
  html: string;
}

// `highlight.js/lib/common` is the same language set lowlight registers for the chat
// renderer, so this adds no bundle weight. Anything outside it renders as plain text.
const EXTENSION_LANGUAGE: Record<string, string> = {
  bash: "bash", sh: "bash", zsh: "bash", fish: "shell", ps1: "powershell", psm1: "powershell",
  c: "c", h: "c", cc: "cpp", cpp: "cpp", cxx: "cpp", hpp: "cpp", hh: "cpp",
  cs: "csharp", go: "go", java: "java", kt: "kotlin", kts: "kotlin", swift: "swift",
  rs: "rust", rb: "ruby", php: "php", pl: "perl", lua: "lua", r: "r", scala: "scala",
  m: "objectivec", mm: "objectivec", py: "python",
  js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript",
  ts: "typescript", tsx: "typescript", mts: "typescript", cts: "typescript",
  css: "css", scss: "scss", sass: "scss", less: "less",
  html: "xml", htm: "xml", xml: "xml", vue: "xml", svelte: "xml", astro: "xml",
  json: "json", jsonc: "json", yaml: "yaml", yml: "yaml",
  toml: "ini", ini: "ini", cfg: "ini", conf: "ini", properties: "ini",
  sql: "sql", graphql: "graphql", gql: "graphql",
  md: "markdown", markdown: "markdown", diff: "diff",
  gradle: "java", cmake: "cmake", makefile: "makefile",
};

const EXTENSIONLESS_LANGUAGE: Record<string, string> = {
  dockerfile: "dockerfile", makefile: "makefile", rakefile: "ruby", gemfile: "ruby",
  vagrantfile: "ruby", brewfile: "ruby",
};

const SPAN_TAG = /<\/?span[^>]*>/g;

export function languageForPath(path: string): string | null {
  const name = (path.split("/").pop() ?? path).toLowerCase();
  const extension = name.includes(".") ? (name.split(".").pop() ?? "") : "";
  const language = extension ? EXTENSION_LANGUAGE[extension] : EXTENSIONLESS_LANGUAGE[name];
  return language && hljs.getLanguage(language) ? language : null;
}

/**
 * highlight.js emits spans that can straddle newlines (a block comment, a template
 * literal). Splitting on `\n` alone would leave each line with unbalanced markup, so
 * carry the open tags across the split: close them at the end of every line and reopen
 * them at the start of the next.
 */
function splitHighlightedLines(html: string): string[] {
  const open: string[] = [];
  return html.split("\n").map((line) => {
    const prefix = open.join("");
    for (const tag of line.match(SPAN_TAG) ?? []) {
      if (tag.startsWith("</")) open.pop();
      else open.push(tag);
    }
    return `${prefix}${line}${"</span>".repeat(open.length)}`;
  });
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * Highlights a file's content and returns it one line at a time. Never throws: an
 * unhighlighted preview is acceptable, a dialog that crashed on an odd file is not.
 */
export function highlightFileLines(path: string, content: string): HighlightedLine[] {
  const language = languageForPath(path);
  let highlighted: string;
  try {
    highlighted = language
      ? hljs.highlight(content, { language, ignoreIllegals: true }).value
      : escapeHtml(content);
  } catch {
    highlighted = escapeHtml(content);
  }
  return splitHighlightedLines(highlighted).map((html, index) => ({ number: index + 1, html }));
}
