import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { checkReadmeParity } from "./check-readme-parity.mjs";

const base = `<!-- docs-section:overview -->
# Title
[Language](README.zh-CN.md)
[Reference](docs/build-performance.md)
<!-- docs-locale-guides -->
[Guide](docs/user-guide/en/src/index.md)
<!-- /docs-locale-guides -->
<!-- docs-fact:project-version value:0.1.0 -->
<!-- docs-fact:tauri-major value:2.x -->
<!-- docs-fact:react-major value:19.x -->
<!-- docs-fact:node-minimum value:22+ -->
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](package.json)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue.svg)](package.json)
[![React](https://img.shields.io/badge/React-19.x-blue.svg)](package.json)
Node.js 22+
<!-- feature:coordination status:preview -->
\`\`\`powershell
npm run dev
\`\`\`
`;

function fixture(contents = [base, base, base], version = "0.1.0") {
  const root = mkdtempSync(join(tmpdir(), "vanehub-readme-parity-"));
  const files = ["README.md", "README.zh-CN.md", "README.ja.md"];
  files.forEach((file, index) => writeFileSync(join(root, file), contents[index], "utf8"));
  writeFileSync(
    join(root, "package.json"),
    JSON.stringify({
      version,
      dependencies: {
        "@tauri-apps/api": "^2.0.0",
        react: "^19.2.8",
      },
    }),
    "utf8",
  );
  return { root, files };
}

// Shields.io escapes a literal hyphen as `--`, so the badge and the fact marker never carry the
// same spelling of a pre-release version.
function withVersion(content, version) {
  return content
    .replace("project-version value:0.1.0", `project-version value:${version}`)
    .replace("badge/version-0.1.0-", `badge/version-${version.replaceAll("-", "--")}-`);
}

test("accepts translated prose with matching stable facts", () => {
  const translated = base.replace("# Title", "# 标题");
  const { root, files } = fixture([base, translated, base.replace("# Title", "# タイトル")]);
  assert.doesNotThrow(() => checkReadmeParity(files, root));
});

test("reports the translated file and mismatched feature state without rewriting it", () => {
  const changed = base.replace("status:preview", "status:delivered");
  const { root, files } = fixture([base, changed, base]);
  const before = readFileSync(join(root, files[1]), "utf8");
  assert.throws(
    () => checkReadmeParity(files, root),
    /README\.zh-CN\.md: featureStates differ/,
  );
  assert.equal(readFileSync(join(root, files[1]), "utf8"), before);
});

test("reports command and link drift", () => {
  const changed = base
    .replace("npm run dev", "npm run preview")
    .replace("docs/build-performance.md", "docs/missing.md");
  const { root, files } = fixture([base, changed, base]);
  assert.throws(() => checkReadmeParity(files, root), /commands differ[\s\S]*links differ/);
});

test("accepts guide links that differ inside the locale block", () => {
  const translated = base.replace(
    "[Guide](docs/user-guide/en/src/index.md)",
    "[用户指南](docs/user-guide/zh-CN/src/index.md)\n[架构与实现](docs/zh/src/README.md)",
  );
  const { root, files } = fixture([base, translated, base]);
  assert.doesNotThrow(() => checkReadmeParity(files, root));
});

test("reports a translation that drops its locale block", () => {
  const changed = base
    .replace("<!-- docs-locale-guides -->\n", "")
    .replace("<!-- /docs-locale-guides -->\n", "");
  const { root, files } = fixture([base, changed, base]);
  assert.throws(() => checkReadmeParity(files, root), /localeGuides differ/);
});

test("reports a visible badge that drifts from its stable fact marker", () => {
  const changed = base.replace("badge/React-19.x", "badge/React-18.x");
  const { root, files } = fixture([base, changed, base]);
  assert.throws(
    () => checkReadmeParity(files, root),
    /README\.zh-CN\.md: visible fact "react-major" is "18\.x"/,
  );
});

test("accepts a pre-release version whose badge escapes the hyphen", () => {
  const preview = withVersion(base, "0.1.0-preview.1");
  const { root, files } = fixture([preview, preview, preview], "0.1.0-preview.1");
  assert.doesNotThrow(() => checkReadmeParity(files, root));
});

test("accepts a multi-segment pre-release identifier", () => {
  const candidate = withVersion(base, "0.1.0-rc.1");
  const { root, files } = fixture([candidate, candidate, candidate], "0.1.0-rc.1");
  assert.doesNotThrow(() => checkReadmeParity(files, root));
});

test("reports a stale pre-release identifier against package.json", () => {
  const stale = withVersion(base, "0.1.0-preview.1");
  const { root, files } = fixture([stale, stale, stale], "0.1.0-preview.2");
  assert.throws(
    () => checkReadmeParity(files, root),
    /README\.md: stable fact "project-version" differs from package\.json/,
  );
});

test("reports a badge that drops the pre-release identifier its marker declares", () => {
  const mismatched = withVersion(base, "0.1.0-preview.1").replace(
    "badge/version-0.1.0--preview.1-",
    "badge/version-0.1.0-",
  );
  const { root, files } = fixture(
    [mismatched, mismatched, mismatched],
    "0.1.0-preview.1",
  );
  assert.throws(
    () => checkReadmeParity(files, root),
    /README\.md: visible fact "project-version" is "0\.1\.0", but its marker is "0\.1\.0-preview\.1"/,
  );
});

test("reports canonical stable facts that drift from package.json", () => {
  const stale = base
    .replace("react-major value:19.x", "react-major value:18.x")
    .replace("badge/React-19.x", "badge/React-18.x");
  const { root, files } = fixture([stale, stale, stale]);
  assert.throws(
    () => checkReadmeParity(files, root),
    /README\.md: stable fact "react-major" differs from package\.json/,
  );
});
