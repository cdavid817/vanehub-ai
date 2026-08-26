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
// 上调理由(upgrade-session-workspace-evidence-console):新增 hunk 级评审决定这条能力,不是拆分。
// 之前接受一个 hunk 调的是 review 级的 `setCodeReviewDecision`,等于整份评审被接受;修掉它需要
// 一个独立的 service 方法、Web/mock 的定点变更实现、以及 Tauri 侧在 Task Group 13 落地持久化前
// 的带原因码拒绝。同一批改动里 `agent-service.ts` 是净减的——评审方法搬进了新的
// `code-review-service.ts`,它因此退出 eslint 技术债清单,由全局 300 行规则接管。
// 上限按实测值 19287 记录,不留余量。
// 再次上调(同一 change,Task Group 2):证据服务是一条全新能力,不是把既有代码挪个位置。新增的
// 692 行分成五个聚焦模块——service 接口、可注入的 native transport(含 typed unavailable 绑定)、
// 序列去重/gap 检测的共享订阅语义、Tauri 客户端、Web/mock 客户端与其确定性 fixture。没有任何一份
// 是对既有分支的复制:两个适配器实现同一个接口,这是 React 不直接 invoke 的前提;共享订阅语义写在
// 一处,正是为了让两个运行时不会在订阅行为上漂移。上限按实测值 19979 记录,不留余量。
// 再次上调(同一 change,Task Group 3.15):证据读取从 typed-unavailable 切到真实原生命令,新增
// 137 行全部在 `tauri-native-evidence-transport.ts`——真实 invoke/listen 绑定、只放行已注册命令的
// 白名单、以及把原生错误压成 reasonCode 的映射。它不是 `native-evidence-transport.ts` 的副本:后者
// 定义 seam 与 typed unavailable 绑定,前者是唯一接触 Tauri API 的实现,拆开正是为了让"哪些命令
// 已注册"只有一处写法。`tauri-session-workspace-evidence-client.ts` 里订阅改成先挂监听再取
// watermark,净增几行。上限按实测值 20116 记录,不留余量。
// 再次上调(同一 change,Task Group 7.9/7.10):Session Shell 是一条全新能力,不是把既有代码挪个
// 位置。新增的 592 行分成四个聚焦模块——service 接口(定义"离开"与"停止"是两个不同的调用,这正是
// 标签页切换不再杀掉构建的前提)、两个运行时各自的适配器、以及它们共用的帧对账模块。共用那一份
// 是刻意的:去重与 gap 检测如果两端各写一份,漂移的恰好是最难看出来的行为——重复交付的帧看起来
// 像 shell 回显,漏掉的帧看起来像命令少输出了几行。旧的 `session-workspace-shell-frames` 契约是
// 净改而非净增:它此前描述的 state 取值(connecting/connected/…)与原生注册表实际发布的六个状态
// 对不上,attachmentId、revision、foregroundProcess 三个字段则根本不存在。上限按实测值 20693
// 记录,不留余量。
// 再次上调(同一 change,Task Group 7.11-7.14):+13 是 `runtime-session-shell-client.ts`——两个适配器
// 的运行时选择器,与 `runtime-agent-client.ts`、`runtime-execution-observability-client.ts` 同一模式。
// 它必须单独存在:React 侧只能拿到这个绑定,组件里出现 `isTauri` 分支正是 ARCH-FE-002 要拦的东西。
// 另 +19 是 `SessionShellEvent`:gap 是客户端在序号跳变时算出来的,不是原生发过来的,所以监听器的
// 事件类型必须比线上通知宽一档。少了这一档,视图看到的帧会紧挨着接上,缺口那段就被当成"本来就
// 没有输出"。上限按实测值 20725 记录,不留余量。
// 再次上调(同一 change,Task Group 8.11):Session Log 的 live notice 是一条全新能力。新增 320 行
// 分成四个聚焦模块,与 Session Shell 同构而非同源——契约在 `src/types/session-log-notice.ts`,不计入
// 本预算;这里的四份是 seam 与 typed unavailable 绑定、共享的序列语义、两个运行时各自的客户端。
// 共享那一份仍然是刻意的,而且这里的漂移比 Shell 更难看出来:日志条目本来就是断续到达的,少掉几条
// 只会让列表短一截,而"短一截"和"这段时间确实没有日志"在界面上长得一模一样。原生 gap 与投递 gap
// 也必须由同一处区分——前者说桥丢了 receipt,后者说通知没送到这个订阅者,两端各写一份就会把同一次
// 丢失按两个原因报两遍。上限按实测值 21045 记录,不留余量。
// 再次上调(同一 change,Task Group 8.12):+65 全部在 web-session-workspace-client.ts,是把 Web/mock
// 从"总是回答 complete、忽略所有关联过滤"改成能被真实驱动。两件事都不是可选补全:design 的 Web/Mock
// Runtime 一节要求 mock 能走完 complete/indexing/partial/unavailable 四态,一个永远回答 complete 的
// mock 会让浏览器构建成为**唯一**看不到不完整覆盖渲染的运行时;而忽略关联过滤的 mock 会让"按 run
// 过滤"在浏览器里静默返回整个会话——那正好是过滤器最容易被误读成证据的方向。关联键到 context 键的
// 映射写成显式表而非按名推导:两者是碰巧长得像的两套词汇,推导会在其中一套改名的当天开始静默匹配
// 不到任何东西。上限按实测值 21110 记录,不留余量。
// 再次上调(同一 change,Task Group 8.14):+12 是 getSessionLogRecord 在接口与两个适配器上的三处声明。
// 它必须存在:live notice 只带标识符不带日志行——这是刻意的,否则事件通道会驮着整个语料,而且一行
// 记录会有两种可能互相矛盾的形状。于是"把这行插进列表"就必须按 id 回取,回取的入口只能在 service
// 边界上。没有它,8.14 的 insert 分支就只能靠通知里那点字段拼一个假的行,那正是这条设计要避免的第二
// 种形状。上限按实测值 21122 记录,不留余量。
// 再次上调(同一 change,Task Group 8.16):+93 是把 live notice 真正接上运行时。两份:
// tauri-native-session-log-transport.ts 是唯一接触 Tauri API 的实现,持有事件通道名与已注册命令
// 白名单——通道名与 Rust 侧写错一个字符会产生一个永不触发也永不报错的订阅,这是活视图从内部无法
// 察觉的唯一失败模式,所以它必须只有一处写法;runtime-session-log-client.ts 是运行时选择器,与
// runtime-agent-client、runtime-session-shell-client 同一模式,组件里出现 isTauri 分支正是
// ARCH-FE-002 要拦的东西。两个运行时共用 dispatcher,只有投递方式不同。
// 上限按实测值 21215 记录,不留余量。
// 再次上调(同一 change,Task Group 9.1-9.5):+121 净增,全部在 Web/mock 的 trace 夹具。夹具从 client
// 抽到 web-execution-observability-fixtures.ts 是净移动,真正的新增是两件事:每个 span 补上 kind 与
// 派生字段(depth/offset/attempt/delegated/criticalPath),以及一条**仍在运行**的 run 加上把它推进到
// 终态的函数。第二件不是可选的:本设计里有几条规则只在边界的一侧成立——运行中的 span 没有 duration,
// 有任何 span 未结束的 run 没有关键路径——一个永远停在一侧的夹具让这些规则无法被观察到,浏览器构建
// 会成为唯一永远看不到"尚不知道"这个状态的运行时,而那恰好是瀑布图最需要诚实呈现的状态。
// 派生值是手写而非计算:它们是夹具,值是固定的,而一个自己重算的 mock 就是原生派生的第二份实现,
// 两者可以在都看起来正确的情况下互相矛盾。另含把分页填充搬进夹具模块:client 在 reset 时从夹具重建,
// 填充留在外面会在第一次 reset 后消失,分页断言随之失败,而失败原因与分页无关。
// 上限按实测值 21367 记录,不留余量。
// 再次上调(同一 change,Task Group 9.11):+81 是 runtime-trace-transition-client.ts——trace 转换的
// 运行时绑定与线上载荷解析。与 runtime-session-log-client 同一模式,同一理由:通道名与 Rust 侧差一个
// 字符会产生永不触发也永不报错的订阅,这是活视图从内部唯一无法察觉的失败模式,所以它只能有一处写法。
// 浏览器构建不发任何转换,而且是**故意**不发:一个按定时器发假转换的 mock 会让合并逻辑看起来能用,
// 而实际上什么都没有转换过。上限按实测值 21448 记录,不留余量。
// 再次上调(同一 change,Task Group 10.13):+65 是报告 JSON 导出在服务边界两侧的实现——接口方法、
// Tauri 侧的目录选择与 invoke、Web 侧的 simulated 应答,以及导出结果的 Zod 解析。
// 值得说明的是为什么这 65 行必须在这里而不能更省:前端**不写文件**。一个用 Blob + download 的实现
// 只需要三行,但那是一次后端从未校验过路径的文件写入,正是导出规则要挡住的东西。所以目录来自原生
// 选择器、写入发生在 Rust 侧、文件名由应用层派生,而这条链路在服务边界上就要占三个实现。
// simulated 是独立状态而非 exported 的变体:浏览器构建无处可写,一个说"已导出"的 demo 会让人去找
// 一个不存在的文件。上限按实测值 21513 记录,不留余量。
// 再次上调(同一 change,Task Group 11.11):+50 是工作区检查能力在服务边界上的三处——接口方法、
// Tauri 侧的 invoke、Web 侧的 simulated 夹具与其类型。
// 值得说明的是为什么 Web 侧不能省:浏览器构建必须报 `simulated` 而不是 `local`。一个说自己在读
// 本机的 demo 会让人去找根本不存在的文件,而 `watchMode: "none"` 同理——夹具永远不变,说 native
// 就是在描述一个不存在的监视器。能力全部 available 是诚实的:夹具里确实有这些东西,真正的缺口
// 是 provider 名字,不是假装缺少某个前置条件。上限按实测值 21563 记录,不留余量。
// 再次上调(同一 change,Task Group 12.3):+63 是工作区失效通知在服务边界上的三处——新拆出的
// `session-workspace-inspection-service.ts`、Tauri 侧的 `listen` 与解析、Web 侧的空订阅。
// 净增没有 52 行那么少的原因是这次顺带做了一次拆分:`agent-service.ts` 已经顶到 300 行硬规则,
// 再加一个方法就过线,所以工作区检查那一组方法整体移进了自己的文件。移动本身不增加行数,增加的是
// 新文件的 import 块与那段说明「为什么这组方法值得单独一个文件」的注释——而下一个方法(12.4 的
// Quick Open)因此有了明确的去处,不必再在 300 行的边缘上挤。
// 值得说明的是为什么 Web 侧那个空订阅不能省:浏览器夹具永远不变,任何它发出的通知都是在描述一件
// 没发生的事;而按定时器造假的通知会让整条失效路由看起来被跑过,实际上没有任何东西真的过期过。
// 上限按实测值 21626 记录,不留余量。
const SUBTREE_LINE_BUDGETS = Object.freeze([
  { root: "src/services", budget: 21626, owner: "upgrade-session-workspace-evidence-console" },
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
