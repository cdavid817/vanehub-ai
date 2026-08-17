import {
  cpSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { extname, resolve } from "node:path";
import { repositoryRoot, run, verifyMdbook } from "./docs-tooling.mjs";

const outputRoot = resolve(repositoryRoot, ".docs-build");
const rustTarget = resolve(repositoryRoot, ".docs-target");

verifyMdbook();
rmSync(outputRoot, { recursive: true, force: true });
rmSync(resolve(rustTarget, "doc"), { recursive: true, force: true });
mkdirSync(outputRoot, { recursive: true });

const books = [
  ["docs/developer-guide", "developer"],
  ["docs/user-guide/en", "user/en"],
  ["docs/user-guide/zh-CN", "user/zh-CN"],
];

for (const [source, destination] of books) {
  run("mdbook", [
    "build",
    resolve(repositoryRoot, source),
    "--dest-dir",
    resolve(outputRoot, destination),
  ]);
}

/**
 * Cross-book links are authored as repository-relative Markdown paths, because that is the form
 * that resolves for a reader on the repository page — the only surface this project publishes.
 * The authored directory layout does not exist in the assembled site, so rewriting them here
 * keeps the site correct without authoring against a layout nobody reads.
 *
 * mdBook already swaps `.md` for `.html` even on a path that leaves the book, so the built
 * markup carries `developer-guide/src/x.html`. Both extensions are matched so the rewrite does
 * not silently stop working if that behaviour changes.
 */
const crossBookRewrites = [
  // From user/<locale>/, the developer guide sits at ../../developer/.
  [
    /\.\.\/\.\.\/\.\.\/developer-guide\/src\/([A-Za-z0-9-]+)\.(?:md|html)/g,
    "../../developer/$1.html",
  ],
  // From developer/, a user guide sits at ../user/<locale>/.
  [
    /\.\.\/\.\.\/user-guide\/(en|zh-CN)\/src\/([A-Za-z0-9-]+)\.(?:md|html)/g,
    "../user/$1/$2.html",
  ],
];

function rewriteCrossBookLinks(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) {
      rewriteCrossBookLinks(path);
      continue;
    }
    if (extname(entry.name).toLowerCase() !== ".html") continue;
    const before = readFileSync(path, "utf8");
    let after = before;
    for (const [pattern, replacement] of crossBookRewrites) {
      after = after.replace(pattern, replacement);
    }
    if (after !== before) writeFileSync(path, after, "utf8");
  }
}

rewriteCrossBookLinks(outputRoot);

run("cargo", [
  "doc",
  "--manifest-path",
  resolve(repositoryRoot, "src-tauri", "Cargo.toml"),
  "--no-deps",
  "--document-private-items",
], {
  env: {
    ...process.env,
    CARGO_TARGET_DIR: rustTarget,
    RUSTDOCFLAGS: `${process.env.RUSTDOCFLAGS ?? ""} -D warnings`.trim(),
  },
});

cpSync(resolve(rustTarget, "doc"), resolve(outputRoot, "api"), { recursive: true });
cpSync(
  resolve(repositoryRoot, "docs", "release-signing.md"),
  resolve(outputRoot, "reference", "release-signing.md"),
);
cpSync(
  resolve(repositoryRoot, "src-tauri", "ARCHITECTURE.md"),
  resolve(outputRoot, "reference", "native-architecture.md"),
);

writeFileSync(
  resolve(outputRoot, "index.html"),
  `<!doctype html>
<html lang="en">
  <head><meta charset="utf-8"><title>VaneHub AI Documentation</title></head>
  <body>
    <main>
      <h1>VaneHub AI Documentation</h1>
      <ul>
        <li><a href="user/en/index.html">User Guide — English</a></li>
        <li><a href="user/zh-CN/index.html">用户指南 — 简体中文</a></li>
        <li><a href="developer/index.html">Developer Guide</a></li>
        <li><a href="api/vanehub_ai_lib/index.html">Native API Reference</a></li>
      </ul>
    </main>
  </body>
</html>
`,
  "utf8",
);

run(process.execPath, [resolve(repositoryRoot, "scripts", "validate-docs.mjs"), "--assembled"]);
console.log(`Documentation assembled at ${outputRoot}`);
