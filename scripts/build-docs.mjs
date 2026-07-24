import {
  cpSync,
  mkdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { resolve } from "node:path";
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
  resolve(repositoryRoot, "docs", "user-guide", "assets"),
  resolve(outputRoot, "user", "assets"),
  { recursive: true },
);
cpSync(
  resolve(repositoryRoot, "docs", "architecture"),
  resolve(outputRoot, "reference", "architecture"),
  { recursive: true },
);
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
