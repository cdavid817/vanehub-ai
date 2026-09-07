# Tasks

以下任务已按实际完成状态勾选（2026-09-06）；未勾选项的原因见 implementation-notes.md 与 verification/results.md。阶段是执行顺序，不是将后续 CLI 留到未来的许可。完成代码任务与真实 CLI/平台验收分别记录，不以 fake 结果代替 live。

## 0. 开工核验与规范校验

- [x] 0.1 读取当前工作区 AGENTS.md、CLAUDE.md 指向、openspec/project.md、config.yaml 和当前主规范；记录 HEAD、分支及 git status，不切换分支、不覆盖已有修改。

- [x] 0.2 读取本 change 全部文档，检查是否有重叠的未归档 changes；在 implementation-notes.md 记录已有实现、冲突和整合方式，不重复创建同名抽象。

- [x] 0.3 沿 references/repository-audit.md 追踪实际注册、启动、策略投影、session capture、参数目录、CLI delegation、MCP/Skill 注入、日志与自动化调用链。

- [x] 0.4 执行 openspec validate extend-cli-providers-with-acp --strict；修正规范错误后才改生产代码，保留主规范原 requirement/scenario 标题。

- [x] 0.5 逐家核对官方文档与可用 --help/版本，记录 provider-matrix.md 的发行形态、安装来源、认证、协议与平台差异；缺失真实程序标 NOT RUN，不虚构版本。

## 1. Provider 契约与能力模型

- [x] 1.1 扩展现有 Provider SDK 支持 transport-aware 会话执行；保留 headless 与 PTY 契约，不另建平行 registry。

- [x] 1.2 实现 supported/unsupported/unknown 与 reasonCode，区分元数据声明、握手能力、宿主实现、当前策略和实际版本。

- [x] 1.3 实现 data-only Manifest V2 及 V1 兼容解析，拒绝未知字段、argv/env/Hook/URL/任意执行入口。

- [x] 1.4 让 usage/resume/reasoning 等可选能力明确 unavailable；完整 SDK 职责允许显式不支持，必需生命周期职责仍强制。

- [x] 1.5 增加原有五种 Provider 的元数据、顺序、参数、权限、事件、会话和 V1 manifest 回归；同步内部 SDK 文档。

## 2. CLI 目录、安装身份与认证

- [x] 2.1 在现有受审计目录添加七个稳定 ID，定义 reviewed basenames、provider identity、active/legacy、平台和 source/detect-only 元数据。

- [x] 2.2 增加 source-safe npm 定义与经审查的 vendor/detect-only 定义；沿用 CliActionPlan，不新增安装器旁路。

- [x] 2.3 完善 Cursor agent 同名冲突、Kimi 新旧发行形态及 Windows launcher alias 判定；保留 PATH-selected 与 recommended 区分。

- [x] 2.4 定义有界只读 version/auth probes，无可靠认证探针保持 unknown；设置页不得启动 ACP 或自动登录。

- [x] 2.5 实现显式登录/兼容性检查操作及安全外链确认；凭据仅保存引用，账号环境与 UI 语言独立。

- [x] 2.6 对实际执行路径、来源、指纹、版本与快照失效做再次校验；处理 vendor 内部切源、平台限制与不可验证 exact-version。

## 3. ACP 连接与协议内核

- [x] 3.1 评估官方 Rust ACP SDK 的固定版本和适配性，记录依赖决策；复用已有进程能力与端口，不使用 React/Node sidecar 替代原生边界。

- [x] 3.2 实现 UTF-8 换行分帧、JSON-RPC 类型校验、有界单帧/深度/队列/待决请求及单 writer；stderr 独立脱敏。

- [x] 3.3 实现并发 reader 和请求分派，等待 prompt 响应期间仍响应 Agent 侧请求，覆盖递归交互死锁测试。

- [x] 3.4 实现 initialize、版本协商、client capabilities、认证方法和会话能力归一化；无能力不声明。

- [x] 3.5 实现 session/new、session/prompt 与支持时 session/load；正确处理 session/update、stopReason、未知请求和未知通知。

- [x] 3.6 区分 process/connection/session/turn 状态；限制单会话单活动 turn，正常 turn 结束不等待进程退出。

- [x] 3.7 实现 session/cancel、等待原 prompt cancelled、宽限期升级、owned process-tree cleanup、代理终端回收与应用退出清理。

- [x] 3.8 加入 typed errors、connectionEpoch、迟到响应丢弃、消息顺序、队列背压、超时与无不安全 fallback。

## 4. 权限、文件与终端代理

- [x] 4.1 将新 Provider 纳入现有权限投影链路，复用当前真实 policy template；所有新入口先校验策略可兑现性。

- [x] 4.2 实现 scoped PendingInteraction store，校验 session/turn/epoch/RPC/tool/policy revision，响应原 option id 并保证一次消费。

- [x] 4.3 实现审批 allow-once/deny/cancel 的精确语义，不把一次许可变永久许可；关窗、超时、断线不批准。

- [ ] 4.4 通过现有文件 API 实现代理方法；测试路径穿越、symlink/junction、父路径不存在、Windows 路径与 TOCTOU 风险处理。

- [x] 4.5 通过既有治理执行器实现 terminal create/output/wait/kill/release、cwd/env 隔离、权限和有界输出；未完整实现不声明能力。

- [x] 4.6 增加 host-enforced/provider-delegated/unverified 保障标识；无法兑现 OS sandbox 等硬要求时阻止执行，终端不能绕过。

- [x] 4.7 复用凭据安全边界、MCP/Skill 能力投影与日志脱敏；禁止导入任意全局凭据、自动同步全局配置或执行不可信协议载荷。

## 5. Qwen Code 第一条垂直链路

- [x] 5.1 实现 qwen-code 安装检测、npm 来源与 explicit login/check，识别实际 qwen 和版本；不复用 Gemini 的能力结论。

- [x] 5.2 完成 qwen --acp 的统一对话和原生终端适配、权限投影、取消、错误、会话 binding 与 optional capabilities。

- [x] 5.3 新增 Qwen 专属合约与 fake ACP 场景，验证中文长输入、分片、工具审批、两轮对话、拒绝/取消与恢复负例。

- [ ] 5.4 在获授权且有对应真实程序的环境完成 Qwen smoke，逐平台记录准确版本；缺少条件保留 live gate 未完成。（2026-09-07 Linux 完成：以用户提供的 DeepSeek 兼容端点，经 VaneHub ACP 适配器完成文本、工具审批、拒绝、取消、重启后恢复全部 live 轮次，Qwen Code 0.23.0，见 verification/results.md；Windows / macOS NOT RUN，故 live gate 按平台仍未闭合）

## 6. Kimi Code CLI 第一批接入

- [x] 6.1 实现 kimi-cli 的 npm/native/旧 Python 发行形态区分、依赖与路径检测，禁止自动运行 migrate。

- [x] 6.2 完成 kimi acp 与当前发行形态终端启动，显式绑定配置档案；禁止将 print auto 权限误映射为逐次审批。

- [x] 6.3 新增新旧 kimi 同名冲突、跨发行形态恢复拒绝、print 策略不可兑现、流式/取消/错误 fixtures。

- [ ] 6.4 完成有授权的当前 Kimi live smoke 或记录阻塞，旧发行形态未验证时只报告其真实兼容范围。

## 7. 国内补充 Qoder 与 CodeBuddy

- [x] 7.1 实现 qoder-cli 的受审计来源、qoder 身份、qoder --acp、终端、认证 profile、策略与平台限制。

- [x] 7.2 实现 codebuddy-code 的受审计来源、codebuddy --acp、终端、显式国内/国际/iOA 环境以及客户端工具代理。

- [x] 7.3 为 Qoder 添加进程级策略隔离、历史命令别名负例与不支持平台测试；为 CodeBuddy 添加代理权限和内部 Team 子事件归属测试。

- [ ] 7.4 逐家完成同一合约套件、UI 可见性、无凭据假测试及有授权的 live smoke 记录，不因第二阶段而跳过。

## 8. Copilot 与 Cursor 接入

- [x] 8.1 实现独立 copilot-cli 的检测、@github/copilot 来源、copilot --acp --stdio 与终端适配，不调用 gh 历史扩展。

- [x] 8.2 完成 Copilot 进程级 tools/reasoning/profile 隔离和专属 fixture，不把其他 CLI 参数模板套用给它。

- [x] 8.3 实现 cursor-agent-cli 的 agent 身份校验、agent acp、终端和 reviewed vendor/detect-only 来源，不只凭 basename。

- [x] 8.4 完成 Cursor ask_question/create_plan 的请求响应、取消/超时与 todo/task 通知；未知阻塞扩展立即明确失败。

- [ ] 8.5 为两家加入合约、身份冲突、权限、两轮、取消、状态与恢复测试；按实际平台分别记录 live smoke。

## 9. iFlow 历史兼容

- [x] 9.1 添加 iflow-cli legacy 目录项、官方服务关闭日期、显式启用、本地程序选择与 detect-only 来源。

- [x] 9.2 实现经验证的本地原生终端路径与适用策略检查；缺程序、未验证版本或不满足策略时明确不可用。

- [x] 9.3 禁止默认安装、托管 ACP、多 Agent/定时执行与自动配置迁移，提供用户管理自定义 API 的说明但不导入凭据。

- [x] 9.4 测试默认隐藏于历史兼容分组、用户 opt-in、无官方服务承诺、后端自动化拒绝与无全局配置副作用。

## 10. 会话持久化、恢复与观测

- [x] 10.1 通过当前数据库迁移机制增加可空 binding 元数据或扩展表；明确 migration id，旧记录可读且继续旧路径。

- [x] 10.2 持久化准确 externalSessionId、installation/distribution/profile/transport/cwd/worktree 引用，禁止 last-session 猜测。

- [x] 10.3 实现 negotiated load、replay 与 live 事件区分，避免重复消息、权限和用量，不按文本 hash 删除合法重复句。

- [x] 10.4 实现连接中断 unknown-effect、禁止盲重放、显式跨 transport 新会话与恢复失败保留历史。

- [x] 10.5 会话删除仅回收自有资源；保留 CLI 全局数据和原有 Worktree 清理语义。

- [x] 10.6 将新链路接入现有日志/观测服务；区分人等候与执行延迟、context occupancy 与计费 usage、reported 与 unavailable。

## 11. 前端契约和界面

- [x] 11.1 扩展服务/DTO，配套 Tauri 与 Web/mock，并使用仓库当前生成器同步需要生成的参数目录和 TypeScript 契约。

- [x] 11.2 扩展 CLI 管理页的安装、认证、兼容、来源、历史标记、能力、动作和 stale 状态；复用现有 action plan review UI。

- [x] 11.3 扩展创建会话选择器，使用稳定 ID 和后端 capability，不新增协议级重复 Agent 或第二套 allowlist。

- [x] 11.4 接入权限、用户问题、计划批准、等待/取消/中断状态；支持多 seat、刷新恢复与重复点击保护。

- [x] 11.5 完善 unavailable usage、真实 model/config option、MCP/Skill 不支持提示；不承诺全局配置同步。

- [x] 11.6 补齐中文/英文与主题、长路径、键盘、焦点、错误恢复；不新增 inline style、any 或直接 invoke。

- [x] 11.7 确保 Web/mock 明示模拟且无真实宿主路径/凭据声明；web-http 未实现不静默落 mock。

## 12. 多 Agent、定时任务与委派

- [x] 12.1 将有能力的新 CLI 经现有公共 runtime port 接入多 Agent seat、定时与适用 CLI delegation，移除与本次新名单相关的漏接硬编码。

- [x] 12.2 实现配置时与执行时 capability/preflight；按每 seat/run 隔离进程、外部 session、工作目录、凭据与权限。

- [x] 12.3 复用任务中心实现 needs-intervention/拒绝/取消，明确 unattended interaction policy 与 deadline，不自动批准未知工具。

- [x] 12.4 记录中断 checkpoint 与未知副作用，不通过通用 retry 重新执行整条 prompt；测试并发、局部取消、缺认证、版本漂移和 iFlow 拒绝。

## 13. 测试与验证

- [x] 13.1 实现无网络、无真实凭据/模型调用的 fake ACP 子进程与多 Provider fixtures；明确 synthetic/recorded 来源，负例不能只是 mock 返回成功。

- [x] 13.2 将 verification/test-matrix.md 的协议、权限、身份、绑定、重放、Windows launcher 和副作用边界用例落成自动测试。

- [x] 13.3 执行旧五种及新七种的通用 conformance、新增 transport suite、SQLite migration 与归属隔离测试。

- [x] 13.4 执行 npm run lint:ci、npm run test、npm run build 与 AGENTS.md 当前要求的额外前端/契约/架构/文档检查。

- [x] 13.5 执行 cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check、cargo check --workspace、cargo clippy --workspace --all-targets -- -D warnings、npm run native:panic:check、cargo test --workspace。

- [x] 13.6 执行 Playwright、desktop:unit:test、test:desktop 与相关层；本机结果不得外推成三平台通过，缺少环境记录 BLOCKED。

- [x] 13.7 分别执行 openspec validate extend-cli-providers-with-acp --strict 和 openspec validate --specs --strict，前者不能由后者替代。

- [x] 13.8 按 verification/runbook.md 记录每条命令实际结果、退出码、环境及证据；真实 CLI 联调记录版本/平台与授权范围。

## 14. 文档与交接

- [x] 14.1 更新中英文 README、用户指南、开发指南、CLI reference/参数矩阵和 provider-sdk 文档；新目录引用接入原导航且不改归档历史。

- [x] 14.2 补充使用教程：安装/来源、登录、创建会话、审批、取消、恢复限制、国内账号环境、iFlow legacy 与排障。

- [x] 14.3 逐条核对 acceptance.md 与 requirements-index，确认已声明功能不是占位或恒定 unknown；补全 implementation-notes.md 和 verification/results.md。

- [x] 14.4 最终报告范围、修改路径、测试实绩、失败/阻塞、兼容限制与剩余工作；未完成不得全打勾，不 commit/push/PR/归档，除非用户另行明确授权。
