import fs from "node:fs";
import path from "node:path";
import ts from "typescript";
import { architectureDiagnostic, architectureSummaryDiagnostic, RULES } from "./rules.mjs";
// Single source of truth for "what counts as a non-token color" — session-workspace/main-layout
// already enforce this via console-visual-tokens.test.ts, which reads these same two patterns.
// Importing rather than re-deriving avoids exactly the drift its own file comment warns about:
// two independently-maintained copies, where the one nobody remembers to update stops meaning
// anything. Node's built-in TypeScript support loads this directly; it has no TS-specific syntax.
import { LITERAL_COLOR, PALETTE_COLOR } from "../../src/session-workspace/visual-token-rules.ts";

// Layout-only inline styles (width/height/transform for a resizable pane or a virtualized list
// item's offset) have no token equivalent and are not what task 2.12 is guarding against — only
// color set via `style` bypasses the semantic-token utility classes the way a literal Tailwind
// arbitrary value does.
const COLOR_STYLE_PROPERTIES = new Set([
  "color", "backgroundColor", "background", "borderColor", "borderTopColor", "borderRightColor",
  "borderBottomColor", "borderLeftColor", "outlineColor", "fill", "stroke", "boxShadow",
  "textDecorationColor", "caretColor", "accentColor",
]);

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
// 上调理由(redesign-unified-workbench-ui,Task 1.1):+5,全部在 settings-service.ts,是临时迁移
// 开关 unifiedWorkbenchV2 在服务边界上的固定开销——AppSettingKey 新增一个枚举值、defaultAppSettings
// 一个默认字段、normalizeAppSettings 一个类型守卫分支。两个 Adapter(tauri-settings-client.ts、
// web-settings-client.ts)都是泛化的透传实现,不需要为这一个新字段各写一条分支,所以净增只有这
// 三处而不是通常的"接口 + 两个 Adapter"三件套。上限按实测值 24141 记录,不留余量。
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
// 再次上调(同一 change,Task Group 12.4):+58 是 Quick Open 在服务边界上的四处——接口方法与
// 它的输入类型、Tauri 侧的 invoke 与 Zod 解析、Web 侧的夹具排名,以及夹具本身那份从
// `directoryFixtures` 派生出来的扁平列表。
// 值得说明的是为什么 Web 侧那份不能省、也不能手写成第二份清单:浏览器构建的 Quick Open 必须只能
// 提供树里真实存在的路径。一份单独维护的搜索夹具迟早会给出一条树里没有的路径,而点开什么都不发生
// 的结果,比没有结果更糟——读者会以为是应用坏了,而不是以为这是个 demo。
// 上限按实测值 21684 记录,不留余量。
// 再次上调(同一 change,Task Group 12.5):+85 是内容搜索在服务边界上的四处——接口的两个方法与
// 输入类型、Tauri 侧的 invoke 与 Zod 解析、Web 侧的夹具扫描,以及取消。
// 值得说明的是为什么「取消」必须占一个独立方法而不能折进搜索请求里:取消要在搜索**还没返回**时
// 到达进程,这正是取消的全部意义。一个带 cancel 标志的搜索请求只会在下一次搜索时才被读到,
// 而那时上一次扫描已经把整个工作区读完了。
// 上限按实测值 21769 记录,不留余量。
// 再次上调(同一 change,Task Group 12.6):+25 是 Files 工具栏需要的两个既有方法的扩展——
// `openSessionFolder` 多一个可选的子目录参数、Shell 创建多一个可选的起始目录。
// 两个都做成可选而不是必填,是为了让所有既有调用点保持它原来的含义(工作区根目录);
// 而不是在服务层再加两个方法,那会让"打开工作区"和"打开工作区的某个子目录"变成两件事,
// 调用方迟早会挑错那一个。
// 上限按实测值 21794 记录,不留余量。
// 再次上调(同一 change,Task Group 12.7):+17 全部在 Web adapter 的 `readSessionFile`——从夹具
// 自身推导编码与换行,而不是写死两个常量。
// 值得说明的是为什么不能写死:这两个字段的全部意义就是"内容里看不见、但会改变读者行为"——BOM 会
// 让 shell 脚本和 JSON 解析器失败,混合换行会把一次普通编辑变成整文件 diff。一个永远返回
// `utf-8` / `lf` 的元数据行,是一行永远不变、因而永远不告诉任何人任何事的 UI。
// 上限按实测值 21811 记录,不留余量。
// 再次上调(同一 change,Task Group 12.11):+27 是"这个文件被记录过什么"这条查询在服务边界上的
// 三处——接口方法、Tauri 侧的 invoke 与 Zod 解析、Web 侧的固定零回答。
// 值得说明的是为什么 Web 侧那个不能省、也不能编:浏览器构建没有执行日志,所以每个文件的
// observations **真的**是 0。一个编出几条记录的夹具,会在界面上放一个通向不存在记录的链接——
// 那比没有链接更糟,因为读者会以为是应用坏了。
// 上限按实测值 21838 记录,不留余量。
// 再次上调(同一 change,Task Group 13.5):+27 是"这个文件读过了吗"这条状态在服务边界上的三处——
// 接口方法、Tauri 侧的 invoke、Web 侧按文件自身指纹记账的夹具。
// 值得说明的是为什么 Web 侧那个夹具不能只存一个布尔:见证必须取自文件自身的指纹,而不是 review
// 快照的指纹。快照指纹覆盖全部变更文件,任何一个文件被写就会变;把 Viewed 挂在它上面,等于
// agent 动了一个文件就把其余十一个的"已读"全部清掉,于是"8 个文件 · 4 个未读"这类计数在真实
// 会话里永远在归零,读者没法据此做任何事。夹具照着真实语义实现,才能在桌面侧走偏时先挂掉。
// 上限按实测值 21865 记录,不留余量。
// 再次上调(同一 change,Task Group 13.6):+28 全部在 Web adapter——按文件自身指纹算见证的
// `witnessOf`、每次读取重算的 `summarize`、以及包装返回的 `withSummary`。
// 值得说明的是为什么夹具要重算而不是存一个计数:计数存下来就是同一个问题的第二份答案,而 marks
// 和 files 各自会变;两者第一次不同步时,header 就在自信地报错数,并且没有任何东西会说它错了。
// 上限按实测值 21893 记录,不留余量。
// 再次上调(同一 change,Task Group 13.7):+31 是标准补丁这条读取在服务边界上的三处——接口方法、
// Tauri 侧的 invoke、Web 侧带真实文件头与 hunk 头的夹具。
// 值得说明的是为什么 Web 夹具不能只回显面板上那几行:那几行没有文件头也没有 hunk 头,并且在面板
// 截断的地方就截断了,读起来对、贴到 git apply 里必然失败。而 Web 构建里没有 git,谁也跑不出这个
// 失败;夹具照着真实结构渲染,才不会让"可读"和"可应用"的区别恰好在这一侧消失。
// 上限按实测值 21924 记录,不留余量。
// 再次上调(同一 change,Task Group 13.10):+4,全部在 Web adapter 的 `withSummary` 里——把每个
// 文件的 `viewed`、以及本次 review 的 hunk 决定一起投影到返回值上。
// 值得说明的是为什么这四行不能省:面板只有在读取里拿得到这两样,重新加载后才不会把已记录的决定
// 全部显示成"未决定"。夹具少投影一样,Web 侧就会长出一个桌面侧没有的行为差异。
// 上限按实测值 21928 记录,不留余量。
// 与 main 合并:以下两段是两条独立能力各自的上调记录,数字在合并后按实测重记。
// 由 `add-local-composer-media-tools` 从 19234 上调 351 行,全部是新增服务边界的固定开销,没有一行
// 是复制既有分支:
//   +60  `local-media-service.ts`——接口本身,15 个方法加注释;
//   +114 `tauri-local-media-client.ts`——每个方法一次 `invoke`,外加原生文件选择器那一处
//        (它必须留在服务层:React 层禁止 import `@tauri-apps/*`,而 OCR 需要一个文件选择器,
//        仓库此前只有目录选择器);
//   +154 `web-local-media-client.ts`——比其他 Web mock 长,因为它不能模拟成功。三个引擎的
//        disabled profile 与 unavailable status 必须逐字段写全,页面才能在浏览器里渲染出真实布局
//        并如实说明"仅桌面端可用";
//   +13   `runtime-local-media-client.ts`——与 `runtime-ssh-connection-client.ts` 同构的绑定层;
//   +10   `local-media-service.contract.test.ts` 不计入(测试文件不在 productionFiles 内)。
// 上限取实测值,不预留余量。
// 同一变更再上调 20 行:设置页需要为模型路径字段调起原生选择器,而 React 层不得 import
// `@tauri-apps/*`,于是 `selectProfilePath` 只能落在服务边界上——接口 8 行(含说明为何设置页持有
// 真实路径仍不算越界)、Tauri 侧 4 行、Web 侧 8 行(浏览器给不出宿主路径,返回 null 并写明理由,
// 而不是让用户手填一条注定不可达的路径)。
// 再上调 8 行:E2E fake 的接线落在 `runtime-local-media-client.ts`。fake 本体在 `src/testing/`
// 不计入本预算,这里只有一个构建期常量分支加解释它为何是构建期而非运行时的注释——运行时开关会在
// 已发布的构建里留下一个可以被打开的入口,而构建期常量在产物中根本不存在。
// 再上调 25 行:桌面 fixture 需要替换「文件选择器返回了哪个文件」这一个边界,于是
// `tauri-local-media-client.ts` 多出一个 `chooseOcrSource`——4 行逻辑加 17 行注释,说明为什么
// 只替换选择结果、为什么 fail-closed 在原生侧而不在这里,以及为什么普通 Desktop Smoke 用同一个
// 构建却必须仍然走真实对话框。选择之后的嗅探、限额、staging、one-time claim 与清理一行未改。
// 再上调 18 行:上一版的 fixture 分支 catch 住任何异常就退回真实对话框,于是命令未注册、IPC 断链
// 这类真实缺陷会在无人应答的 headless runner 上变成挂起,而不是一条能读的失败。现在只有恰好等于
// `FIXTURE_OCR_SOURCE_UNAVAILABLE` 的稳定码才回退,其余一律重新抛出——多出来的是一个从错误串尾部
// 取稳定码的小函数,以及说明为什么只有这一个码允许回退的注释。
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
// 上调理由(add-unified-personalization-governance):个性化治理在服务边界上多出的是新能力的固定
// 开销,没有一行是从别处复制来的:`personalization-service.ts` 是接口本身,
// `tauri-personalization-client.ts` 是命令的 invoke 映射,剩下的大头在 Web/mock —— 它必须真的
// 拒绝:版本冲突、reset token 与 scope 不匹配、未知枚举值、缺失的 workspace。一个一律放行的
// mock 会让页面的冲突分支一次也跑不到,而那正是真实桌面上最先触发的一条。
// 同一轮里 `agent-memory-service.ts` 与两侧的 `listAllMemories`/无 scope delete/reset 整个删掉,
// 抵掉了一部分。
// 与 origin/main 合并后按合并树实测重取:两侧各自在自己的基线上报了上限(20576 / 20327),改的是
// 不同文件,合并树的真实总数既不是两者之一也不是两者之和。下面这个数字是直接测量得到的。
// 上调理由(add-source-aware-cli-environment-management):CLI 环境边界从 3 个方法变成 9 个,
// 因为"准备计划"和"执行计划"必须是两次调用——执行只收计划 ID 与版本号,这样"复核过的版本就是
// 实际执行的版本"是结构上成立的,而不是靠约定。9 个方法在 Tauri 与 Web/mock 两侧各实现一份,
// 是这条边界的固定开销。同一轮里 `web-cli-tool-client.ts`(183 行)整个删掉,mock 快照数据
// (185 行)搬进 JSON,没有保留任何旧路径,也没有复制既有分支。
// 与 upgrade-cli-parameter-management 合并后按合并树实测重取,不是两侧上限相加:那一侧删掉的
// `cli-parameter-catalog.ts` 抵掉了这一侧的一部分。
// 合并 origin/main(local-media)后按合并树重测:两侧各自在自己的基线上报了上限(19803 / 19960),
// 改的是不同文件,合并树的真实总数既不是两者之一也不是两者之和。下面这个数字是直接测量得到的。
// 上调理由(extend-lsp-language-registry):+49,全部落在生产文件上,测试不计入。
// +45 是 `web-lsp-client.ts` 的 mock 语言注册表:Web 模式没有后端注册表可查,契约对等就要求这一侧
// 自带一份镜像。它换来的正是这个变更的目的——之后加一种语言是改这张表里的一条数据,而不是改组件。
// +7 是 `lsp-contract.ts` 的描述符归一化与 startupArguments 字段;同时删掉的 `expectedServer`
// 语言→服务器硬映射抵回去一部分,所以净额远小于新增逻辑本身。
// -3 是 `tauri-agent-client.ts`:`normalizeLspServerTestResult` 的第二个参数原本只用于那条硬映射
// 校验,映射没了参数也就没了。没有复制任何既有分支,前端也不再有任何写死的语言名。
// 上调理由(expand-lsp-read-only-methods):+35,同样全部落在生产文件上。
// 五个新的只读工具在 Web 侧各需要一个确定性的 unavailable 信封,加上 mock 协商能力列表里对应的
// 五条方法。这一侧没有可删的重复:信封的形状本来就各不相同,类型定义与实现复用了 definition 的
// 信封,已经是能复用的部分。
// 同时把 `webLspToolClient` 从 `web-lsp-client.ts` 拆出去,因为九个工具的信封让合并后的文件撞上
// 300 行硬规则。拆分只搬不抄:唯一被复制的 `clone` 帮助函数随即删掉了——那里每次都新建对象,
// 本来就没有共享状态需要防御性拷贝。
// 上调理由(add-lsp-java-jdtls):+20,全部在生产文件上。
// `lsp-contract.ts` 的描述符归一化多出 overrideTarget 与 prerequisite 两个字段;`types/lsp.ts` 加
// 了 LspOverrideTarget;`web-lsp-client.ts` 的 mock 注册表多了 Java 这一条,并把两个新字段带进
// descriptors()。这一条 mock 数据正是让 Web 侧也走 install_directory 分支的原因——否则那个分支
// 只有桌面侧跑得到。
// 合并 `upgrade-session-workspace-evidence-console` 后按实测重记为 23030。
// 这不是第三次上调,而是两条互不相交的能力各自的上调在同一棵子树上相加:上面两大段理由分别属于
// 证据控制台与本地媒体/LSP,两边都新增了 service 接口 + Tauri 客户端 + Web/mock 客户端这三件套。
// 三件套是 React 不直接 invoke 的代价,它按能力数量线性增长而不是按代码量——合并只是让两份固定
// 开销出现在同一个数字里,没有任何一行是复制来的。
// 上调理由(manage-language-server-installation):+89,全部在生产文件上。其中只有 54 行是新能力，
// 其余 35 行是两次拆分的固定开销——只搬不抄，两侧都有实测减数对应。
// 新能力：`tauri-agent-client.ts` +26(install/uninstall 两个方法，加一个把 `Result<_, String>`
// 命令抛出的裸字符串包成 `Error` 的小函数——不包的话调用方的 `instanceof Error`
// 分支会把后端给的 reason code 掉成一条通用失败)；`web-lsp-client.ts` +22(两个如实拒绝的
// 方法加 mock 注册表的 distribution/installed 两个字段)；`lsp-contract.ts` +6(描述符的
// distribution 归一化。它没有被内联掉：内联后一个既非 null 也非 record 的 distribution
// 会静默变成"没有发行信息"，而这个模块的整个存在意义就是 fail closed)。
// 拆分：`lsp-service.ts` +23 / `agent-service.ts` -14——LSP 的 9 个方法从伞型接口里搬进自己的
// 服务接口，和它已经组合的另外 30 个领域接口一致；`agent-service.ts` 降到 292 行，按
// eslint.config.js 写明的策略，它在技术债清单里那条 306 行的条目同一个 commit 删掉。
// `lsp-contract-values.ts` +99 / `lsp-contract.ts` -89(另 +16 是它的 import 块)——强制 fail closed
// 的值强转帮助函数单独成模块，`lsp-contract.ts` 从 302 降到 227。它本来就不在技术债清单上，
// 300 行是硬规则；不拆的唯一选项是把上面那个 fail-closed 检查换成 6 行额度。
//
// 合并 `upgrade-session-workspace-evidence-console` 后按合并树实测重记,先 23119,再随
// `add-unified-personalization-governance` 一起并入后重测为 24136。
// 这不是一次次上调,而是几条互不相交的能力各自的上调落在同一棵子树上:上面那些段落分属证据
// 控制台、本地媒体/LSP 与个性化治理,每一条都新增了 service 接口 + Tauri 客户端 + Web/mock
// 客户端这三件套。三件套是"React 不直接 invoke"这条规则的代价,按能力条数线性增长而不是按代码
// 量——所以每次合并都在合并后的树上重测,不把两侧的数相加,后者会把两边共有的基线算两遍。
// 上调理由(redesign-unified-workbench-ui,5.15):Loop 轮询补上 Mission Control 早就有的
// document-visibility 守卫(隐藏时跳过 fetch,focus/visibilitychange 时立即补一次追赶),全部落在
// `loop-run-polling.ts` 同一个订阅函数里——没有新文件,也没有复制既有分支。上限按实测值 24148
// 记录,不留余量。
//
// 上调理由(redesign-unified-workbench-ui,19.10):Scheduled Tasks 新增 Run Now 能力,按上面同一条
// "service 接口 + Tauri 客户端 + Web/mock 客户端" 三件套模式新增 +29 行——`scheduled-task-service.ts`
// +2(接口方法签名)、`tauri-agent-client.ts` +5(invoke 调用)、`web-scheduled-task-client.ts` +22
// (合成一条真实感的 dispatch 回执,不改动既有的 `scheduledTasks` 数组)。没有新文件,也没有复制既有
// 分支。上限按实测值 24177 记录,不留余量。
// 上调理由(redesign-unified-workbench-ui,19.8):Scheduled Tasks 新增版本感知的 `updateScheduledTask`,
// 同一条三件套模式再新增 +35 行——`scheduled-task-service.ts` +2(接口方法签名)、
// `tauri-agent-client.ts` +5(invoke 调用)、`web-scheduled-task-client.ts` +28(版本冲突检测,复用
// `create` 已有的 agent/frequency 校验而非重新实现,成功时把 version 自增一)。没有新文件,也没有
// 复制既有分支。上限按实测值 24212 记录,不留余量。
// 上调理由(redesign-unified-workbench-ui,review fix for 19.8):review 阶段发现 Rust 与 Web/mock 两侧的
// `updateScheduledTask` 都无条件用新 frequency 重算 `next_run_at`,哪怕这次编辑根本没碰 frequency——
// 一次纯改名会静默把任务的下次触发时间重置。修复只改判断,不新增分支:`web-scheduled-task-client.ts`
// +6(按 `sameScheduledTaskFrequency`——新落在 `scheduled-task-recurrence.ts`,不计入本预算——决定是
// 保留旧值还是重算)。上限按实测值 24218 记录,不留余量。
// 上调理由(redesign-unified-workbench-ui,Task 19.11):+56,全部在 web-scheduled-task-client.ts。此前
// `listScheduledTaskRuns` 只从 `task.latestRunAt` 合成一行,而这个字段从未被写入,实际恒返回空数组——
// 与 Tauri 侧真正的多行 `scheduled_task_runs` 查询完全不对齐。新增 `scheduledTaskRuns` 模块状态、
// `seedRunHistory`(合成 succeeded/backfilled/failed 三种真实状态,覆盖此前从未被行使的状态词表)、
// `ensureRunHistory`(首次访问时惰性播种,镜像 `web-prompt-hook-versions.ts` 的
// `ensureWebPromptHookVersion` 先例),并让 `runScheduledTaskNow` 真正记录手动运行——此前只生成回执,
// 从不落盘,`listScheduledTaskRuns` 永远看不到它。都是新增能力的固定开销,没有复制既有分支。
// 上限按实测值 24274 记录,不留余量。
// 上调理由(redesign-unified-workbench-ui,Task 18.6):+24,全部是 `list_evaluation_arenas` 从"返回
// 全部、硬编码 (0, 100)"改为真游标分页的三件套固定开销——`evaluation-service.ts`(接口新增
// `EvaluationArenaQuery`/`EvaluationArenaPage` 类型与方法签名)、`tauri-agent-client.ts`(+cursor/limit
// 透传)、`web-evaluation-client.ts`(内存数组真实切片分页,不再一次性吐出全部)。没有新文件,也没有
// 复制既有分支。上限按实测值 24298 记录,不留余量。
const SUBTREE_LINE_BUDGETS = Object.freeze([
  { root: "src/services", budget: 24298, owner: "redesign-unified-workbench-ui" },
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
// Denylist, not an allowlist: an allowlist would need updating every time a genuinely shared
// helper (src/lib, src/i18n, src/types) grows a new file, while this only needs updating if a
// new feature domain directory appears at the top of src/ — and `src/features/` is covered by
// prefix so domains created under it need no changes here either.
const UI_PRIMITIVE_FORBIDDEN_ROOTS = [
  "src/services/",
  "src/main-layout/",
  "src/session-workspace/",
  "src/components/chat/",
  "src/settings/",
  "src/work-board/",
  "src/goal-center/",
  "src/mission-control/",
  "src/loop-center/",
  "src/evaluation-center/",
  "src/features/",
];

function resolveRelativeSpecifier(file, specifier) {
  if (!specifier.startsWith(".")) return null;
  return path.posix.normalize(path.posix.join(path.posix.dirname(file), specifier));
}

function checkNoSemanticColor(file, report, text, node) {
  if (!file.startsWith("src/ui/")) return;
  const literalMatch = LITERAL_COLOR.exec(text);
  if (literalMatch) {
    report(RULES.nonSemanticColor, node, `src/ui/ primitive uses a literal-color arbitrary value \`${literalMatch[0]}...\` instead of a semantic token`);
  }
  const paletteMatch = PALETTE_COLOR.exec(text);
  if (paletteMatch) {
    report(RULES.nonSemanticColor, node, `src/ui/ primitive uses Tailwind default-palette class \`${paletteMatch[0]}\` instead of a semantic token`);
  }
}

function checkStyleAttributeForColor(report, node) {
  const init = node.initializer;
  if (!init || !ts.isJsxExpression(init) || !init.expression) return;
  const expr = init.expression;
  // A spread or a variable reference can't be statically resolved here; this stays conservative
  // rather than guessing, matching how the rest of this checker treats unanalyzable expressions.
  if (!ts.isObjectLiteralExpression(expr)) return;
  for (const property of expr.properties) {
    if (!ts.isPropertyAssignment(property) && !ts.isShorthandPropertyAssignment(property)) continue;
    const name = ts.isIdentifier(property.name) || ts.isStringLiteral(property.name) ? property.name.text : undefined;
    if (name && COLOR_STYLE_PROPERTIES.has(name)) {
      report(RULES.nonSemanticColor, property, `src/ui/ primitive sets color via inline style (\`${name}\`) instead of a semantic-token utility class`);
    }
  }
}

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

// The `tauri-*.ts` naming convention is the actual boundary, not the .tsx extension: hooks are
// .ts and are just as capable of calling invoke() directly as a component is, and every
// legitimate direct Tauri caller in this repo already starts with this prefix — client and
// transport suffixes both occur (tauri-agent-client.ts, tauri-native-evidence-transport.ts).
function isTauriAdapterFile(file) {
  return /(^|\/)tauri-[^/]+\.ts$/.test(file);
}

export function analyzeFrontendSource(file, source, { requiresServiceBoundary = !isTauriAdapterFile(file) } = {}) {
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
      if (requiresServiceBoundary && specifier?.startsWith("@tauri-apps/")) {
        report(RULES.tauriBoundary, node.moduleSpecifier, `React surface imports \`${specifier}\``);
      }
      if (requiresServiceBoundary && specifier && NATIVE_ADAPTERS.has(specifier)) {
        report(RULES.tauriBoundary, node.moduleSpecifier, `React surface imports native adapter \`${specifier}\``);
      }
      if (requiresServiceBoundary && specifier && RUNTIME_HELPERS.has(specifier)) {
        report(RULES.runtimeBranch, node.moduleSpecifier, `React surface imports runtime selector \`${specifier}\``);
      }
      if (requiresServiceBoundary && specifier?.startsWith("@tauri-apps/")) {
        for (const element of node.importClause?.namedBindings?.elements ?? []) {
          if ((element.propertyName ?? element.name).text === "invoke") invokeBindings.add(element.name.text);
        }
      }
      if (file.startsWith("src/ui/") && specifier?.startsWith("@tauri-apps/")) {
        report(RULES.uiPrimitiveIsolation, node.moduleSpecifier, `src/ui/ primitive imports Tauri API \`${specifier}\``);
      }
      if (file.startsWith("src/ui/") && specifier) {
        const resolved = resolveRelativeSpecifier(file, specifier);
        const forbiddenRoot = resolved && UI_PRIMITIVE_FORBIDDEN_ROOTS.find((root) => `${resolved}/`.startsWith(root));
        if (forbiddenRoot) {
          report(RULES.uiPrimitiveIsolation, node.moduleSpecifier, `src/ui/ primitive imports feature-specific module \`${specifier}\` (resolves under ${forbiddenRoot})`);
        }
      }
    }
    if (file.startsWith("src/ui/") && ts.isJsxAttribute(node) && ts.isIdentifier(node.name) && node.name.text === "style") {
      checkStyleAttributeForColor(report, node);
    }
    // Class-name strings reach a className prop indirectly as often as directly in this codebase
    // (a `const xClass = "..."` constant, or a `cn(base, condition && "...")` call) — checking
    // every string/template literal in the file, not just ones attached to a className attribute,
    // catches those without having to trace each string back to its eventual JSX usage.
    if (file.startsWith("src/ui/") && ts.isStringLiteralLike(node) && !ts.isImportDeclaration(node.parent)) {
      checkNoSemanticColor(file, report, node.text, node);
    }
    if (requiresServiceBoundary && ts.isCallExpression(node) && ts.isIdentifier(node.expression) && invokeBindings.has(node.expression.text)) {
      report(RULES.tauriBoundary, node.expression, "React surface calls Tauri invoke directly");
    }
    if (requiresServiceBoundary && ts.isPropertyAccessExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "window" && node.name.text.startsWith("__TAURI")) {
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
