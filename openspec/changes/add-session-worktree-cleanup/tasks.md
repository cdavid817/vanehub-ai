# 实施任务：add-session-worktree-cleanup

状态：待实现。只有代码存在且对应验证取得证据后才能把任务改为 `[x]`。不要因已有设计或 mock 测试通过就勾选真实 Git、恢复或跨平台任务。

任务顺序以门禁依赖为准，可分小提交。若目标分支代码已调整，先修订定位和契约，不放宽安全不变量。建议提交信息使用英文 Conventional Commits，不自动 push、合并或归档。

## 0. 基线与范围确认

- [x] 0.1 读取根 AGENTS.md、CLAUDE.md 指向、openspec/config.yaml、openspec/project.md 和相关上下文规范；记录 git status、当前分支与 HEAD，不覆盖已有修改。
- [x] 0.2 执行 openspec list 和相关规范查询；检查当前分支是否存在重叠 change；对本 change 的 MODIFIED 需求重读原文，保留其他变更增加的场景。
- [x] 0.3 跟踪所有单条/批量/搜索/归档删除入口、旧 deleteSession 调用者、Tauri 命令、会话事务、运行停止、创建 worktree 和操作恢复注册点，更新 code-map.md。
- [x] 0.4 查清 session、folder、worktree、Loop、定时任务、Shell/CLI、后台命令和派生 watcher 的引用/准入入口，形成门禁覆盖清单；未覆盖的入口不得默认为无风险。
- [x] 0.5 核实迁移顶号、数据库事务约定、跨进程锁/claim 和 Operation 设施，记录复用方案；不使用本包臆测的迁移号或新建重复调度基础设施。
- [x] 0.6 在改业务代码前执行 openspec validate add-session-worktree-cleanup --strict，记录原有测试基线及环境限制。

## 1. 契约与纯策略

- [x] 1.1 定义 SessionDeletionService、预览/执行/重试/查询 DTO、runtimeEffect、分组结果、typed reason codes；保持 React 不直接 invoke。
- [x] 1.2 定义 keep/remove-safe 选择与 ignored fingerprint 确认，禁止前端传任意删除路径、force 或 branch-delete 参数。
- [x] 1.3 实现纯策略函数：未知/不完整拒绝、普通项目和远程只 keep、脏目录拒绝、ignored 必须额外确认、attached HEAD 保护、来源与引用保护。
- [x] 1.4 定义 journal 的阶段/效果/修订状态，区分 removal_unknown、worktree_removed、finalize_pending；不扩展无关的全局运行状态枚举。
- [x] 1.5 明确旧 deleteSession(sessionId) 的 keep-only 包装语义与同一 claim 仲裁，添加契约测试防止后续默认启用磁盘删除。
- [x] 1.6 为所有纯策略编写失败优先测试，覆盖 S-02/S-03/S-04、W-02 至 W-06 与 O-01。

## 2. Git 身份与安全探针

- [x] 2.1 在现有 Git/Filesystem adapter 中增加 bounded、只读身份探针；使用可信 cwd 和参数数组，隔离继承 Git 环境，不改全局配置或 safe.directory。
- [x] 2.2 实现 worktree list --porcelain -z 及 Git 状态 NUL/字节安全解析，覆盖 rename、多行/Unicode/非 UTF-8 文件名和分支前导特殊字符。
- [x] 2.3 实现 root/common-dir/admin-dir/登记与可用 file identity 核验，拒绝主 worktree、普通目录、symlink/junction 跳转、同名对象替换与失效锚点。
- [x] 2.4 检查真实 attached HEAD 和本地分支引用；拒绝 detached、身份偏离、锁定/prunable、未完成 Git 操作及不支持的布局。
- [x] 2.5 检查目标不嵌套在其他 worktree 内且不包围其他 worktree，阻止历史子目录布局的危险清理。
- [x] 2.6 实现 tracked/staged/conflict/untracked/ignored 独立检查、元数据摘要和扫描上限；不读取敏感文件正文，不跟随外部链接或特殊挂载。
- [x] 2.7 实现唯一非 force Git remove 和移除后观测；测试拒绝后没有 recursive delete、prune、clean、reset、unlock、repair 或引用删除兜底。
- [x] 2.8 为超时实现进程退出确认与 unknown 效果，不可确认时禁止重复 remove；校验 Git 版本/能力不足的安全拒绝。
- [x] 2.9 用临时真实 Git 仓库验证 W-02 至 W-07；测试结束检查主仓库文件、分支/HEAD、Git 登记与目标外哨兵文件。

## 3. 来源记录与数据迁移

- [x] 3.1 根据当前迁移体系增加或复用 managed worktree 来源/状态、session 绑定、删除操作/分组/claim 表；加必要唯一约束与索引。
- [x] 3.2 设计资源/journal 与 session 删除的外键关系，确保聊天数据正常级联而未处理资源和删除回执不被级联清除。
- [x] 3.3 在普通 worktree 创建链路接入持久化意图、创建后身份和会话绑定；意图失败不运行 Git，后续失败进入 needs_attention 而不是自动删除成果。
- [x] 3.4 实现按需历史来源核验；仅完整来源证据与当前 Git 身份匹配时 legacy_verified，其余只 keep；迁移不执行 Git 删除。
- [x] 3.5 实现 keep 后 retained 记录、remove 后 removed 回执，保留最小来源 ID 而非聊天正文；设置未解决记录的保留保护。
- [x] 3.6 测试空库升级、当前库升级、历史会话读取、重复迁移/初始化、外键级联和创建副作用后的数据库故障恢复。

## 4. 引用保护与跨实例门禁

- [x] 4.1 实现真实有效工作目录引用解析，覆盖 folder 指向 worktree 或子目录、历史 worktreePath、归档/隐藏会话；避免把同源不同 worktree 混为共享。
- [x] 4.2 接入 Loop review/运行、定时任务（包含禁用但仍有绑定）、后台命令、Shell/CLI 及其他已定位的持久化引用；失败时返回 incomplete。
- [x] 4.3 引入或复用 session deletion claim 和 workspace-use gate，定义一致身份与锁顺序，确保获取门禁与最终引用核验之间无应用内入场漏洞。
- [x] 4.4 所有相关创建/绑定/改路径/启动执行入口接入同一门禁；明确派生观察器可释放、业务引用不能自动解绑。
- [x] 4.5 实现多窗口/多实例仲裁、owner 失效判断和有界等待；不能仅依赖进程内 Mutex 或按 TTL 盲目重抢破坏性任务。
- [x] 4.6 与自动归档、会话恢复、Agent/seat 启动和调度器联动，防止 deleting/finalize_pending 会话被重新执行。
- [x] 4.7 用 barrier/fake clock 与真实临时数据库测试新增引用竞态、重复清理、同源不同 worktree、不完整引用和 keep 不影响其他会话。

## 5. 删除协调器、幂等和恢复

- [x] 5.1 实现只读预览、opaque preview 存储、有效期、目标集合绑定、风险摘要和允许策略；UI 预览不产生停止/删除副作用。
- [x] 5.2 实现 requestId + canonical request hash 幂等创建、活动 session claim 唯一性、资源分组和选择冲突校验。
- [x] 5.3 实现 quiesce barrier：等待所有本应用生成/seat/工具、CLI、后台命令、Shell 和应释放句柄退出，超时保留会话。
- [x] 5.4 在静止且门禁持有后执行最终核验，忽略仅由正常停止导致的 lifecycle 更新，但拒绝安全相关文件/资源/引用变化。
- [x] 5.5 持久化 remove_started 后执行 Git，观测并写入 receipt；写失败、超时、部分移除均不得声称零副作用。
- [x] 5.6 以每资源组单一事务完成会话/消息删除、条件清空 active selection、更新资源/绑定/journal；事件仅在 commit 后发出。
- [x] 5.7 实现 keep-only 同一路径、失败不自动降级、已确认无不可逆效果后释放 claim、显式新授权重新获取 claim，以及仅 DB finalize 的受限重试。
- [x] 5.8 实现按原资源身份的启动恢复：receipt 后收尾、remove_started 重新观察、完整保留需新授权、部分/离线/同名新对象进 needs_attention。
- [x] 5.9 实现逐组结果聚合与只重试未完成组；相同 common dir 的 Git 删除串行，批量不承诺磁盘事务原子性。
- [x] 5.10 为每个不可逆边界、数据库提交/receipt/事件失败以及崩溃点增加注入测试，并核验不会重复 Git remove。

## 6. Tauri 与 Web/mock 接入

- [x] 6.1 添加预览、执行、查询、待处理列表和重试 commands，更新 registry、DTO/生成契约及当前所需权限声明。
- [x] 6.2 阻塞 Git、扫描与有界进程等待放入受管理线程任务；命令返回 handle，状态由 journal/操作查询提供，不阻塞主线程。
- [x] 6.3 新增 Tauri session deletion adapter 并纳入现有 agent/runtime service 组合，错误转结构化 reason code，不靠 stderr 子串匹配业务原因。
- [x] 6.4 实现 Web/mock 相同契约、可控失败/竞态/部分完成场景、simulated 标记与重复请求语义；不伪造真实磁盘清理。
- [x] 6.5 保留 web-http 无 adapter 时显式错误；测试不退回 mock，更新 adapter conformance 与 contracts 测试。
- [x] 6.6 保持旧 keep-only 命令/调用者在同一协调器中受仲裁，清查所有会话删除旁路。

## 7. 单条与批量 UI

- [x] 7.1 拆分删除 dialog、worktree row、result 和 hook/model，复用 ApplicationDialog；新增生产 TS/TSX ≤300 行，不增加豁免。
- [x] 7.2 接入侧栏单条、右键、搜索、归档及其他所有可见删除入口；移除绕过确认的直接删除回调。
- [x] 7.3 实现默认 keep、危险选择不持久化、目标变化重置、检查中/失败/不完整状态与具体禁用原因。
- [x] 7.4 展示真实路径/分支、完整性、共享引用和忽略文件风险；提供有作用域的复制/打开目录/查看变更，不传任意路径执行。
- [x] 7.5 实现 ignored fingerprint 二次确认、过期重检与安全变更使授权失效；不自动复选或沿用旧授权。
- [x] 7.6 确认按钮按选择切换；防重复提交，显示真实阶段；执行中关闭必须有持续操作面板，否则禁止关闭。
- [x] 7.7 实现失败、部分完成、finalize_pending、needs_attention 与重试入口，删除非活动会话不清空当前会话。
- [x] 7.8 批量按资源一行选择并列出引用会话，保留失败目标；不在 onBatchDelete 回调后立即 exitBatch 丢失反馈。
- [ ] 7.9 补齐现有全部语言键、ARIA/focus/keyboard、长路径、小窗口、深浅色模式与必要样式；不引入新 UI/状态库。
- [x] 7.10 完成组件/交互和 Playwright 用例，包含所有入口、取消无副作用、默认不勾、忽略确认、批量部分成功和恢复展示。

## 8. 原生验收与安全回归

- [x] 8.1 完成 acceptance-tests.md 的需求到自动化映射；未自动化项写明原因、替代证据及阻塞，不当作通过。
- [x] 8.2 原生 integration 测试创建临时 Git worktree、执行真实 service 删除，并核验目录/登记均消失、分支/提交保留；同时测试 keep 完整保留。
- [x] 8.3 验证 dirty/staged/untracked/ignored、symlink/junction、主目录/外部目录/嵌套目录、detached/locked/submodule 等拒绝行为。
- [x] 8.4 验证多实例门禁、引用竞态、停止超时、目录替换、失效预览、同 requestId/冲突 requestId 与安全停止后的正常推进。
- [x] 8.5 验证 Git 成功 DB 失败、receipt 失败、Git 部分效果、重启离线和同路径新对象，确保无重复 remove 或隐式 prune/force。
- [x] 8.6 使用现有桌面测试夹具隔离用户数据库/CLI/凭据/项目；为真实 IPC 与对话框增加端到端用例，不用模型调用。
- [x] 8.7 在当前平台运行桌面分层测试；Windows/macOS/Linux 结果分别记录 PASSED/FAILED/BLOCKED/NOT RUN，不跨平台外推。

## 9. 文档、门禁与交付

- [x] 9.1 更新用户指南中的会话删除和 worktree 章节，说明默认保留、分支保留、忽略文件风险、来源不明/共享阻止和失败恢复；保留 Loop 的原有规则。
- [x] 9.2 更新开发指南/契约说明的领域边界、journal/门禁/原生与 mock 区别，不改无关历史文档或 openspec archive。
- [x] 9.3 运行 npm run lint:ci、npm run test、npm run build；根据当前 AGENTS.md 执行 coverage、contracts 和 architecture 等相应门禁并记录结果。
- [x] 9.4 运行 cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check、cargo check --workspace、cargo clippy --workspace --all-targets -- -D warnings、npm run native:panic:check、cargo test --workspace。
- [x] 9.5 运行 npx playwright test、npm run desktop:unit:test 与当前平台 npm run test:desktop（分层运行先构建）；缺少系统依赖时标记 BLOCKED，不伪造跨平台通过。
- [x] 9.6 运行 openspec validate add-session-worktree-cleanup --strict、openspec validate --specs --strict 和 git diff --check；涉及文档再执行当前实际 docs 门禁。
- [x] 9.7 更新 verification.md 的实际命令/版本/平台/证据，逐项勾选已完成任务；检查 force/递归删除/任意路径/旧旁路/日志秘密未引入。
- [x] 9.8 输出实现摘要、改动文件、测试结果、剩余限制与风险；未全部完成/未获归档授权前不 archive，不擅自 push 或合并分支。
