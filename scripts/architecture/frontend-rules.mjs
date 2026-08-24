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
// 上调理由(extract-web-client-state-modules):把 web-agent-client.ts 里被多个上下文共用的可变
// 绑定搬进 state 模块时,除了上面那份固定开销,还多出一层访问器——原先一句直接读写(`sessions`、
// `activeSessionId`、`webSkills`)现在是一对 `list*`/`replace*` 函数。这层是"禁止导出可变绑定"
// 这条规则的代价,不是重复代码:访问器集合按调用点裁剪,每个都有本模块之外的调用者。
// 上调理由(extract-web-client-chat-state):同上一条,把最后 8 个共享可变绑定搬进 state 模块的
// 代价还是那两项——每个新模块的固定开销,加上一层按调用点裁剪的访问器。这一轮多出来的访问器都在
// chat 一侧:原先直接 `activeStreams.has/set/delete`、`messagesBySession.delete`,现在各是一个函数。
// 上调理由(decompose-web-send-message):`sendMessage` 的方法体移出后,新增 10 行是两个 scheduler
// 与显式上下文的固定模块/类型边界开销;旧方法已整段删除,未复制业务分支。
// 上调理由(route-user-messages-to-mentioned-seat):这一次不是拆分,是补一条原本缺失的行为——
// mock 侧此前恒把回复归属给首席位,与 native 的路由规则不一致。新增的 57 行里,`webRoutedSeatId`
// 是 native `route_user_message` 的镜像实现(mention → 上一轮发言者 → 首席位),没有可搬移的现成
// 代码;`seatHandlesFromNames` 反向减少了重复——句柄去重原先在 `seat-mention-options.ts` 里
// 单独实现一份,现在两处共用同一份,那个文件因此是净减的。
// 上调理由(optimize-loop-engineering-workbench):Loop 服务新增的项目/分支发现与只读 preflight
// 需要独立的 Tauri/Web 适配器和契约测试。两端实现同一 service boundary，避免 React 直接
// 依赖 native invoke；新增代码是跨运行时边界和确定性模拟所必需的，不是对既有业务分支的复制。
// 上调理由(improve-workspace-ui-ergonomics):这一次同样不是拆分,是补一条缺失的能力——会话运行时
// 失败后没有任何恢复入口。新增的 57 行是这条能力在服务边界上的固定开销:接口方法与
// `SessionRuntimeRecovery` 结果类型各一份,Tauri 与 Web/mock 两个适配器实现各一份,外加一条契约
// 测试。没有复制既有分支;`tauri-agent-client.ts` 反而是净减的——恢复相关的四个方法搬进了
// `tauri-session-recovery-client.ts`,与 Web 侧早就存在的 `web-session-recovery-client.ts` 对齐,
// 少一个方法就会一眼看出来。
// 合并后下调:上面两条各自按自己那一侧的增量报了上限(19471 / 19581),但同一批上游改动在
// src/services 里是净减的,合并后实测只有 19234——比两侧的预估、也比它们共同的基线 19414 都低。
// 上限按实测值收紧,不保留任何一侧凭预估留下的余量。
// 上调理由(upgrade-cli-parameter-management):CLI 参数命令切到 v2 DTO 后,Web/mock 适配器不能再
// 依赖手工维护的 catalog。新增的 273 行是这条边界的固定开销:`cli-parameter-registry.ts` 用 zod
// 解析 generated 契约(裸 `as` 会让生成器回归变成运行期形状错配,而适配器恰恰是 native 测试照不到
// 的地方),`cli-parameter-renderer.ts` 是 native 渲染策略的镜像,两者都没有可搬移的现成实现。
// 其余 +215 是 `web-cli-parameter-client.ts` 补上 revision/catalogVersion 乐观并发与结构化错误
// (mock 若放行冲突,页面的冲突分支就永远不会被跑到),以及两个适配器的新 preview 方法。
// 另有 51 行是 v1 浏览器存储的一次性迁移:v1 用 `"default"` 与 `false` 两个哨兵表示"未设置",
// 但定义里真有 `default` 选项或本身是 tri-state 时它们都不是哨兵,所以转换必须按定义而不是按字符串
// 匹配——与 native 侧同一条规则。迁移只读不写,v1 键原样保留。
// 旧的 `cli-parameter-catalog.ts`(207 行)此时还删不掉——它只剩两个测试消费者,随 task 10.4
// 一起下线,届时这条上限应当回落。
// 上调理由(add-source-aware-cli-environment-management):CLI 环境边界从 3 个方法变成 9 个,
// 因为"准备计划"和"执行计划"必须是两次调用——执行只收计划 ID 与版本号,这样"复核过的版本就是
// 实际执行的版本"是结构上成立的,而不是靠约定。9 个方法在 Tauri 与 Web/mock 两侧各实现一份,
// 是这条边界的固定开销。同一轮里 `web-cli-tool-client.ts`(183 行)整个删掉,mock 快照数据
// (185 行)搬进 JSON,没有保留任何旧路径,也没有复制既有分支。
// 与 upgrade-cli-parameter-management 合并后按合并树实测重取,不是两侧上限相加:那一侧删掉的
// `cli-parameter-catalog.ts` 抵掉了这一侧的一部分。
const SUBTREE_LINE_BUDGETS = Object.freeze([
  { root: "src/services", budget: 19803, owner: "add-source-aware-cli-environment-management" },
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
