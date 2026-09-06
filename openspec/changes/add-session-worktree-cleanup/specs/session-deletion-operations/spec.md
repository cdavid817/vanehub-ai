# session-deletion-operations Delta Specification

## ADDED Requirements

### Requirement: Authoritative bounded deletion previews

<!-- O-01; acceptance: TC-060, TC-061, TC-062, TC-100 -->
删除预览 SHALL 接受有界 session ID 集合，并由后端解析目标、允许策略、真实资源身份、引用和风险。previewId SHALL 是有有效期且绑定目标快照的 opaque 引用。预览 SHALL 不把路径决定权交给前端。

#### Scenario: Produce a deletion preview
- **WHEN** 用户请求一组可访问会话的删除预览
- **THEN** 服务 SHALL 返回会话与去重 worktree 分组、风险/完整性、允许策略、有效期和 runtimeEffect

#### Scenario: Reject stale or altered authorization
- **WHEN** preview 已过期、目标集合改变、choice 引用未在 preview 中的资源或重复冲突选择
- **THEN** 执行 SHALL 被拒绝且不得删除目录或会话

#### Scenario: Keep works without filesystem access
- **WHEN** 用户仅确认删除会话但 Git 不可用或目录离线
- **THEN** 系统 SHALL 允许在会话授权和停止成功后删除会话
- **AND** 系统 SHALL 不对文件系统发起删除或要求 Git 健康才允许 keep

#### Scenario: System activity sessions are refused
- **WHEN** 预览或执行请求包含系统活动会话 id、空集合、超过批次上限或重复 id
- **THEN** 服务 SHALL 拒绝或按既有不可变系统会话规则排除该 id，去重后仍 SHALL 遵守批次上限
- **AND** 被拒绝的请求 SHALL 不创建 journal、不停止会话、不删除目录

### Requirement: Durable deletion journal before side effects

<!-- O-02; acceptance: TC-063, TC-064, TC-065 -->
执行 SHALL 在停止与删除副作用前持久化请求、每组选择及排他 session claims，在 Git remove 前持久化 remove_started 和可信身份快照。持久化失败 SHALL 阻止尚未开始的副作用。journal SHALL 独立于被删除的会话存在。

#### Scenario: Journal creation fails
- **WHEN** 删除操作或排他 claim 的首次事务失败
- **THEN** 系统 SHALL 不停止会话或执行 Git 删除
- **AND** 会话数据 SHALL 保持存在

#### Scenario: Record before Git removal
- **WHEN** 最终核验通过且准备调用 Git remove
- **THEN** 系统 SHALL 先提交 remove_started 和身份快照
- **AND** 提交失败 SHALL 不执行 remove

#### Scenario: Protect recovery records from retention
- **WHEN** 普通日志保留策略清理历史操作展示数据
- **THEN** 未完成删除和未解决资源的最小恢复 journal SHALL 保留

### Requirement: Quiescence before session deletion

<!-- O-03; acceptance: TC-066, TC-067, TC-068 -->
会话删除 SHALL 停止并等待本应用管理的生成、CLI、后台命令、Shell 及相关工作区句柄释放。取消请求受理 SHALL 不作为退出回执。停止必须有界，失败 SHALL 保留会话记录并阻止 worktree 移除。

#### Scenario: Cancellation was accepted but execution still runs
- **WHEN** runtime 返回取消已受理但进程或生成仍在写入
- **THEN** 协调器 SHALL 继续等待真实静止证据或报告停止超时
- **AND** 系统 SHALL 不执行 Git remove 或会话数据删除

#### Scenario: A managed process cannot stop
- **WHEN** Shell/CLI/后台命令或需释放的观察器未在期限内退出
- **THEN** 对应组 SHALL 失败并保留会话/目录
- **AND** 已停止的进程 SHALL 不自动重启来假装回滚

#### Scenario: One seat is still active
- **WHEN** 多 Agent 会话中部分 seat 已停但仍有其他 seat/工具运行
- **THEN** 会话 SHALL 不被视为静止
- **AND** 清理 SHALL 等待全部受管理写入者退出

### Requirement: Workspace use gates cover reference and execution admission

<!-- O-04; acceptance: TC-069, TC-070, TC-071, TC-072 -->
清理从最终引用核验到效果确认期间 SHALL 持有应用范围的独占资源门禁。会话/任务绑定、执行启动、目录复用和其他写入入口 SHALL 使用同一身份与门禁仲裁；多实例 SHALL 不只依赖单进程 Mutex。对外部不受管理进程 SHALL 不宣称具有排他隔离。

#### Scenario: A new session targets a removing worktree
- **WHEN** 清理门禁持有期间另一窗口或实例试图创建使用该目录的会话
- **THEN** 新引用提交 SHALL 被拒绝或等待门禁
- **AND** 系统 SHALL 不产生指向已删目录的新会话

#### Scenario: A task changes its target during cleanup
- **WHEN** 清理期间定时任务或执行配置试图绑定目标
- **THEN** 该变更或执行入场 SHALL 经相同资源门禁仲裁

#### Scenario: Only one cleanup owner is permitted
- **WHEN** 多个应用实例同时请求清理同一真实 worktree
- **THEN** 最多一个 owner SHALL 能进入 Git remove
- **AND** 租约过期但旧 owner/进程状态不明 SHALL 不允许新的破坏性执行

#### Scenario: Keep does not freeze other users of a directory
- **WHEN** 用户只删除一个共享目录会话且选择 keep
- **THEN** 系统 SHALL 只停止目标会话活动
- **AND** 其他未删除会话 SHALL 不因共享路径而被停止或冻结

### Requirement: Final revalidation before removal

<!-- O-05; acceptance: TC-073, TC-074, TC-075 -->
系统 SHALL 在停止完成且持有资源门禁后，重新验证身份、文件状态、ignored 确认、Git HEAD/ref 和引用。任何与安全相关的变化 SHALL 使授权失效。正常停止导致的 lifecycle/消息更新 SHALL 不被误当作资源风险变化。

#### Scenario: Tracked file changes after preview
- **WHEN** 用户确认后 Agent 或外部工具修改 tracked 文件
- **THEN** 系统 SHALL 在实际 remove 前拒绝清理并刷新风险信息

#### Scenario: A reference appears before gate acquisition
- **WHEN** 预览后至获得门禁前出现新的外部会话或任务引用
- **THEN** 最终核验 SHALL 阻止清理

#### Scenario: Runtime stopping changes only lifecycle metadata
- **WHEN** 停止活动只更新 lifecycle/消息而资源、文件与引用不变
- **THEN** 系统 SHALL 不仅因 session.updatedAt 变化而否决所有正在运行的会话删除

### Requirement: Group finalization preserves observed side effects

<!-- O-06; acceptance: TC-076, TC-077, TC-078 -->
remove-safe 组 SHALL 先确认 worktree 已移除，再以单一数据库事务删除该组会话及原有级联数据、更新活动选择与分组结果。不能完整回滚外部副作用时 SHALL 明确区分移除状态和数据库状态。keep 组 SHALL 不执行 Git 删除。

#### Scenario: Worktree removed but database commit fails
- **WHEN** Git 已确认移除且会话删除事务失败
- **THEN** journal SHALL 保留 worktree_removed/finalize_pending
- **AND** 系统 SHALL 不重复移除、不恢复假目录，也不让目标会话重新执行

#### Scenario: Deletion transaction commits
- **WHEN** 该组会话删除和 journal 完成状态事务提交
- **THEN** 会话数据和原有消息级联 SHALL 一致删除
- **AND** 活动选择 SHALL 只在其 ID 属于已删集合时清空，提交后才发布事件

#### Scenario: Git fails before confirmed removal
- **WHEN** 用户选择 remove-safe 但资源未确认完整移除
- **THEN** 系统 SHALL 不自动删除会话记录或改为 keep
- **AND** 重新选择 keep SHALL 是新的明确授权且只在未决效果已核实后允许
- **AND** 已证明目标完整且没有在途清理进程时 SHALL 持久化失败结果并释放本次删除 claims；效果不明或目录已移除时 SHALL 保持必要隔离

### Requirement: Idempotent deletion requests and scoped retries

<!-- O-07; acceptance: TC-079, TC-080, TC-081 -->
requestId SHALL 唯一绑定规范化请求内容。相同请求重传 SHALL 返回同一操作；不同内容复用同 ID SHALL 冲突。重试 SHALL 仅作用于未完成分组并保持不可逆效果记录，再次 Git 清理 SHALL 要求新预览与确认。

#### Scenario: Duplicate identical request
- **WHEN** 重复点击或响应丢失后使用同 requestId 和相同内容重试
- **THEN** 系统 SHALL 返回同一 operationId
- **AND** Git 移除和会话数据库删除 SHALL 不重复启动

#### Scenario: Request ID reused for other targets
- **WHEN** 相同 requestId 被用于不同目标或不同策略
- **THEN** 系统 SHALL 返回幂等冲突并保持既有操作不变

#### Scenario: Retry partial results
- **WHEN** 一批操作部分完成后用户重试剩余目标
- **THEN** 已成功分组 SHALL 不重放
- **AND** 再次可能产生磁盘副作用的组 SHALL 需要新 preview，DB-only finalize SHALL 不执行 Git

### Requirement: Resource-grouped batch deletion

<!-- O-08; acceptance: TC-082, TC-083, TC-084 -->
批量执行 SHALL 后端按真实资源身份分组，并保证同一资源最多一次成功移除。分组内的所选引用会话 SHALL 全部静止并在最终事务中一并删除。批量结果 SHALL 保留逐组成功、失败与待决信息，不承诺跨资源文件系统原子性。

#### Scenario: All referencing sessions are selected
- **WHEN** 同一 worktree 的所有会话引用都在本次选择集且没有其他业务引用
- **THEN** 后端 SHALL 形成一个资源组并最多移除一次
- **AND** 该组会话 SHALL 统一进入最终删除事务

#### Scenario: An unselected reference remains
- **WHEN** 批量只选择同一 worktree 的部分引用会话
- **THEN** 资源清理 SHALL 被阻止
- **AND** 用户仍 SHALL 可以显式仅删除选中的会话

#### Scenario: Independent groups have different outcomes
- **WHEN** 一个资源组完成而另一个组停止/清理失败
- **THEN** 系统 SHALL 保留成功组结果与失败组现场
- **AND** 聚合 SHALL 显示 partial 或相应非全成功结果，不能用 Promise.all 拒绝丢掉已完成证据

### Requirement: Crash reconciliation without blind destructive replay

<!-- O-09; acceptance: TC-085, TC-086, TC-087, TC-088, TC-089, TC-090 -->
启动恢复 SHALL 从持久化阶段及可信仓库/资源身份重新观察真实效果。恢复 SHALL 不盲目重放 Git remove、不把离线视为删除成功、不删除同名新对象。恢复中的会话 SHALL 保持删除 claim，不被普通会话恢复或自动归档重新激活。

#### Scenario: Crash after removal receipt
- **WHEN** 应用在 worktree_removed receipt 后、会话事务前退出
- **THEN** 恢复 SHALL 验证原目录和登记仍缺失且仓库可访问后仅重试数据库完成
- **AND** 恢复 SHALL 不再运行 Git remove

#### Scenario: Crash between Git effect and receipt
- **WHEN** journal 为 remove_started，原 owner 已失效，原目录和登记都确认不存在且仓库身份可验证
- **THEN** 恢复 SHALL 记录 removed_observed_after_interruption 并完成原授权数据库收尾
- **AND** 记录 SHALL 区分效果观测与丢失的 Git 返回码

#### Scenario: No removal observed after interruption
- **WHEN** journal 为 remove_started 但目录和登记均保持原身份完整
- **THEN** 恢复 SHALL 要求新的预览和确认
- **AND** 启动扫描 SHALL 不自动重新执行破坏性命令

#### Scenario: Ambiguous or offline resource on restart
- **WHEN** 只剩目录或登记之一、仓库离线、权限拒绝或目标身份未知
- **THEN** 系统 SHALL 进入 needs_attention 并保留会话/证据
- **AND** 系统 SHALL 不 prune、repair、递归删除或假报成功

#### Scenario: Same path recreated by another actor
- **WHEN** 旧操作后同路径出现新目录/新 worktree 身份
- **THEN** 恢复 SHALL 不删除新对象并要求人工处理

#### Scenario: Other lifecycle maintenance runs concurrently
- **WHEN** 自动归档、会话状态恢复或调度器遇到删除中的会话
- **THEN** 这些入口 SHALL 尊重删除 claim
- **AND** 目标 SHALL 不被重新置为可执行或清除未完成删除状态

### Requirement: Deletion diagnostics and resource budgets

<!-- O-10; acceptance: TC-091, TC-092 -->
删除操作 SHALL 经统一日志记录结构化阶段、资源 ID、错误码、完整性、效果与耗时，不记录秘密正文。预览、停止、Git 执行、输出及批次数量 SHALL 有显式上限，超限 SHALL 安全失败。每个平台的验证结果 SHALL 独立呈现。

#### Scenario: Sensitive files and Git diagnostics
- **WHEN** 清理失败涉及敏感文件名、Git stderr 或运行环境
- **THEN** 界面 SHALL 只展示必要的结构化原因
- **AND** 持久化诊断 SHALL 经过统一脱敏且不保存文件正文或完整环境

#### Scenario: Any bounded check exceeds its budget
- **WHEN** 批次数量、扫描条目、输出字节或执行 deadline 超限
- **THEN** 系统 SHALL 返回有界失败/incomplete 状态并保留未完成目标
- **AND** 系统 SHALL 不无限等待或把截断结果当完整安全检查

### Requirement: Honest runtime parity for session deletion

<!-- O-11; acceptance: TC-093, TC-094, TC-095 -->
Desktop/Tauri SHALL 执行真实 Git 和数据库流程；Web/mock SHALL 提供相同类型与可测试决策但明确标为 simulated。缺少 HTTP adapter 的运行环境 SHALL 明确拒绝，不回退为伪造的真实磁盘成功。

#### Scenario: Web mock simulates cleanup
- **WHEN** 在 Web/mock 中执行模拟 remove-safe 流程
- **THEN** 预览、handle 和结果 SHALL 标明 simulated
- **AND** 界面 SHALL 不宣称已删除用户本机目录或释放真实空间

#### Scenario: Native cleanup integration
- **WHEN** 在桌面测试环境中完成真实 worktree 清理
- **THEN** 验收 SHALL 检查临时目录不存在、Git 登记不存在且分支仍可解析
- **AND** mock 返回值或截图 SHALL 不能替代这些原生证据

#### Scenario: HTTP adapter is absent
- **WHEN** 当前服务在 web-http 运行时没有相应 adapter
- **THEN** 系统 SHALL 显式报告不支持
- **AND** 系统 SHALL 不静默调用 Web/mock 返回清理成功
