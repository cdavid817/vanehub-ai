import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { architectureDiagnostic, architectureSummaryDiagnostic, RULES } from "./rules.mjs";

// 每文件预算(eslint.config.js)管不住"把一个超大文件拆成十个大文件",也管不住
// 拆分时把代码复制而非搬移。聚合预算才能。预算只能下调;上调必须在同一个 commit 写明原因。
// 上调理由(split-web-agent-client):把 web-agent-client.ts 的方法搬进 web-* 模块时,方法体
// 是平移的,涨的只是每个模块的固定开销——import 行、`export const x: XService = {`、收尾的 `};`。
// 窄接口的签名同样是从 AgentService 搬出去的,不是复制的。所以聚合只按新模块个数线性微涨;
// 一旦涨幅超出这个量级,说明方法被重写或复制了,那时预算失败才是正确结果。
const SUBTREE_LINE_BUDGETS = Object.freeze([
  { root: "src/services", budget: 18368, owner: "split-web-agent-client" },
]);

const STATE_PACKAGES = new Set([
  "redux",
  "react-redux",
  "@reduxjs/toolkit",
  "zustand",
  "mobx",
  "mobx-react",
  "mobx-react-lite",
]);
const NATIVE_ADAPTERS = new Set([
  "./services/tauri-agent-client",
  "../services/tauri-agent-client",
  "../../services/tauri-agent-client",
]);
const RUNTIME_HELPERS = new Set([
  "./services/runtime-mode",
  "../services/runtime-mode",
  "../../services/runtime-mode",
]);

function location(sourceFile, node) {
  return sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
}

function moduleName(node) {
  return ts.isStringLiteral(node.moduleSpecifier) ? node.moduleSpecifier.text : undefined;
}

function packageRoot(specifier) {
  if (specifier.startsWith("@")) return specifier.split("/").slice(0, 2).join("/");
  return specifier.split("/")[0];
}

export function analyzeFrontendSource(file, source, { reactSurface = file.endsWith(".tsx") } = {}) {
  const sourceFile = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const diagnostics = [];
  const invokeBindings = new Set();

  function report(rule, node, message) {
    diagnostics.push(architectureDiagnostic(rule, file, location(sourceFile, node), message));
  }

  function visit(node) {
    if (ts.isImportDeclaration(node)) {
      const specifier = moduleName(node);
      if (specifier && STATE_PACKAGES.has(packageRoot(specifier))) {
        report(RULES.stateManagement, node.moduleSpecifier, `prohibited production dependency \`${specifier}\``);
      }
      if (reactSurface && specifier?.startsWith("@tauri-apps/")) {
        report(RULES.tauriBoundary, node.moduleSpecifier, `React surface imports \`${specifier}\``);
      }
      if (reactSurface && specifier && NATIVE_ADAPTERS.has(specifier)) {
        report(RULES.tauriBoundary, node.moduleSpecifier, `React surface imports native adapter \`${specifier}\``);
      }
      if (reactSurface && specifier && RUNTIME_HELPERS.has(specifier)) {
        report(RULES.runtimeBranch, node.moduleSpecifier, `React surface imports runtime selector \`${specifier}\``);
      }
      if (reactSurface && specifier?.startsWith("@tauri-apps/")) {
        for (const element of node.importClause?.namedBindings?.elements ?? []) {
          if ((element.propertyName ?? element.name).text === "invoke") invokeBindings.add(element.name.text);
        }
      }
    }
    if (reactSurface && ts.isCallExpression(node) && ts.isIdentifier(node.expression) && invokeBindings.has(node.expression.text)) {
      report(RULES.tauriBoundary, node.expression, "React surface calls Tauri invoke directly");
    }
    if (reactSurface && ts.isPropertyAccessExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "window" && node.name.text.startsWith("__TAURI")) {
      report(RULES.runtimeBranch, node, `React surface reads native runtime global \`window.${node.name.text}\``);
    }
    ts.forEachChild(node, visit);
  }
  visit(sourceFile);
  return diagnostics;
}

function productionFiles(root) {
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const target = path.join(root, entry.name);
    if (entry.isDirectory()) files.push(...productionFiles(target));
    else if (/\.(ts|tsx)$/.test(entry.name) && !/\.(test|spec)\.(ts|tsx)$/.test(entry.name) && !entry.name.endsWith(".d.ts")) files.push(target);
  }
  return files.sort();
}

function adapterConformance(projectRoot, relativeFile, exportName) {
  const file = path.join(projectRoot, relativeFile);
  const source = fs.readFileSync(file, "utf8");
  const sourceFile = ts.createSourceFile(relativeFile, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  for (const statement of sourceFile.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    for (const declaration of statement.declarationList.declarations) {
      if (ts.isIdentifier(declaration.name) && declaration.name.text === exportName && declaration.type?.getText(sourceFile) === "AgentService") return [];
    }
  }
  return [architectureDiagnostic(RULES.adapterParity, relativeFile, 1, `\`${exportName}\` is not explicitly checked against AgentService`)];
}

// 与 wc -l 和 Rust 侧的 str::lines().count() 一致:结尾换行不额外算一行,缺结尾换行
// 也不少算最后一行,空文件是 0 行——"".split("\n") 会给出 [""],不特判就会变成 1。
export function physicalLines(source) {
  if (source === "") return 0;
  return source.split("\n").length - (source.endsWith("\n") ? 1 : 0);
}

export function subtreeBudgetDiagnostics(projectRoot, budgets = SUBTREE_LINE_BUDGETS) {
  return budgets.flatMap((entry) => {
    const measured = productionFiles(path.join(projectRoot, entry.root)).reduce(
      (total, file) => total + physicalLines(fs.readFileSync(file, "utf8")),
      0,
    );
    if (measured <= entry.budget) return [];
    return [
      architectureSummaryDiagnostic(
        RULES.lineBudget,
        entry.root,
        `${measured} aggregate physical lines exceeds budget ${entry.budget}. Owner: ${entry.owner}.`,
      ),
    ];
  });
}

export function checkFrontendArchitecture(projectRoot) {
  const srcRoot = path.join(projectRoot, "src");
  const diagnostics = productionFiles(srcRoot).flatMap((file) => {
    const relative = path.relative(projectRoot, file).split(path.sep).join("/");
    return analyzeFrontendSource(relative, fs.readFileSync(file, "utf8"));
  });
  diagnostics.push(...adapterConformance(projectRoot, "src/services/tauri-agent-client.ts", "tauriAgentClient"));
  diagnostics.push(...adapterConformance(projectRoot, "src/services/web-agent-client.ts", "webAgentClient"));
  diagnostics.push(...subtreeBudgetDiagnostics(projectRoot));
  return diagnostics;
}
