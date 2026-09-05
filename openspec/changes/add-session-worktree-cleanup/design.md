# 技术设计：会话删除与可选 worktree 安全清理

## 1. 产品决定与执行边界

### 1.1 固定决定

| 决定 | 第一版语义 |
| --- | --- |
| 默认选择 | `keep`，每次新打开或更换目标重新默认，不记住破坏性选择 |
| 可选动作 | 只删除会话 / 删除会话并安全移除关联 worktree |
| 分支处理 | 始终保留，不提供删分支开关 |
| 脏目录 | 阻止清理；用户可另行选择只删除会话 |
| 忽略文件 | 显式展示风险，完成本次快照二次确认后才可能允许清理 |
| 不可核验目录 | 阻止清理，不阻止正常的仅删除会话流程 |
| Git 失败 | 不升级为 force，不递归删除，不静默降级 |
| 失败恢复 | 有独立持久化记录，能区分目录已移除和会话数据待完成 |
| 外部并发 | 不保证对任意外部写入原子隔离，明确提示并采取执行前复查和原生 Git 保护 |

“始终保留分支”是本功能不修改引用的承诺，不是防止用户在外部删分支的全局保证。清理前要求实际 worktree 为 attached HEAD，实际本地分支存在并指向 HEAD；不要求已合并。detached HEAD 即使旧记录中还有分支名，也不能清理。

### 1.2 范围收敛

轻量资源注册和删除恢复是安全实施所需，不扩展为任意 worktree 管理平台。Loop、子 Agent、外部创建目录和远程路径只参与引用/排除检查，不在此处新增删除能力。

历史会话若缺乏可信的创建证据，显示「无法确认此 worktree 的创建来源，已保留」。本版不在危险动作弹窗内提供“我确认归属”的绕过按钮。后续可以单独设计显式导入/接管功能。

## 2. 已知基线及需重新核实的内容

基线证据见 [code-map.md](code-map.md)。当前 service 签名为 `deleteSession(sessionId): Promise<void>`；前端批量确认会在发起回调后退出选择模式；Rust 普通删除依次停止活动、删除会话记录，未清理 worktree。

以下不是已核实的存量设施，实施前必须检查并记录：现有迁移顶号、跨进程锁原语、操作恢复注册点、路径引用索引、各执行入口和停止完成回执。不能假设一个 `stop_generation()` 返回成功就等于所有写入者已退出。

根 `AGENTS.md` 在本基准要求 React 19、npm、组件 service 边界、生产 TS/TSX 文件不超过 300 行、统一日志和 workspace Rust 检验。不得套用旧对话里 React 18 的描述。

## 3. 架构与依赖方向

```text
React 删除确认组件 / 主布局模型
            │ preview / execute / retry / get status
            ▼
SessionDeletionService（前端统一接口）
       ┌────┴────────────┐
       ▼                 ▼
Tauri adapter       Web/mock adapter
       │                 └─ 明确标识 simulated，无本机 Git 副作用
       ▼
Tauri commands（映射、鉴权、入参边界、派发；不承载领域决策）
       ▼
Sessions deletion coordinator（操作、分组、停止与会话事务编排）
   ┌─────────┬──────────┬──────────────┬───────────┐
   ▼         ▼          ▼              ▼           ▼
删除日志端口  运行静止端口  工作区清理端口    引用快照端口   操作/统一日志端口
   │                    │
 SQLite             Workspaces application service
                            │
                  Git / Filesystem / workspace-use gate
```

依赖约束：sessions/application 不直接调用 Git，不直连 workspaces/infrastructure。跨上下文由基础设施组合器通过 API/port 注入。workspaces 不反向依赖 sessions/application 来查询引用；通过其端口接受只读引用查询实现。新的门禁原语放到已有适合的共享平台/工作区设施，不造第二套并行运行调度器。

可复用已有 Operation 展示和查询，但删除 journal 是可靠状态真源，不是从日志文本推断。保持已有 OperationStatus 枚举，删除 phase 和分组结果放入本能力的结构化 DTO；聚合部分成功可以投影为现有 `failed` 状态并带 `outcome=partial`，不得污染全局状态语义。

## 4. 用户交互

### 4.1 无 worktree

```text
删除会话「分析项目」？
会话及聊天记录将被删除，此操作不可撤销。
项目目录及其中的文件不会被删除。

                         [取消] [删除会话]
```

普通目录没有 worktree 删除复选框；在远程会话中注明远程目录不会被删除，不建立 SSH 删除连接。

### 4.2 有可识别的 worktree

```text
删除会话「修复登录」？
默认保留工作目录和 Git 分支。

目录：D:\code\vanehub-ai-fix-login      [复制路径] [打开目录]
分支：vanehub/fix-login
状态：检查中 / 未发现未提交修改 / 有变更 / 无法确认
引用：其他会话 0、任务 0（仅在检查完整时显示）

[查看变更]
☐ 同时删除关联的 worktree
  将移除该工作目录及其 Git 登记；对应 Git 分支保留。

                    [取消] [仅删除会话]
```

勾选后按钮变为「删除会话及 worktree」。存在忽略文件时增加独立区域：数量或完整性标记、有限路径样例、「被忽略不等于可丢弃」，以及默认未勾选的「已检查并备份需要保留的文件」。它绑定当前 preview 的 ignored manifest，不可跨目录、跨预览或跨重试沿用。

检查中可允许用户只删除会话，不允许选清理。实现可以优先返回会话级信息，再异步刷新 Git 检查；取消弹窗不能停止 Agent、删除会话、修改 Git 配置或工作区文件。预览只读业务资源，允许写入有 TTL 的应用内部预览缓存，不产生运行/清理操作。

### 4.3 明确的禁用原因

禁用清理时保留对应行和原因，而不是隐藏所有关联信息。典型原因：未提交修改、其他引用、来源不明、目录缺失、实际 Git 身份不一致、Git 检查失败、锁定、detached HEAD、嵌套工作目录或子模块。本次无法清理仍可选择只删除会话。

“打开目录/查看变更”必须走有作用域的 service，服务端重新解析 session/resource/preview，不接受前端任意路径命令。会话已经进入不可逆删除阶段后，禁止再启动新的查看器或 Shell；可查看此前取得的只读诊断。

### 4.4 执行与关闭

确认后禁用重复提交，显示阶段而不是假百分比：「停止执行」「重新检查」「移除工作目录」「删除会话记录」。可以关闭弹窗继续在操作面板观察，但关闭不等于取消，操作句柄必须留存。若没有可持续观察的现有面板，第一版阻止执行中关闭，不可丢掉结果。

错误保留弹窗和目标，提供「重新检查」「重试」；仅在工作目录未发生不可逆变更且状态已确认时，提供「改为仅删除会话」。此动作创建新的显式授权，不改写原操作。

采用 ApplicationDialog 与现有按钮/反馈组件，默认焦点在取消或说明而非破坏性按钮，focus trap、Esc、焦点回送、aria-live、键盘勾选齐全。确认按钮 Enter 仅在按钮明确聚焦时执行；普通输入区不能触发隐式破坏性提交。支持现有语言集、长中文/英文路径、小窗口、深浅色主题。新增组件保持 300 行限制，状态机放到独立 hook/model。

## 5. 工作区身份、来源与引用

### 5.1 不把路径字符串等同于资源身份

`repositoryId`：以规范化 Git common directory 的实际文件系统身份为依据的应用内标识。

`worktreeId`：应用分配的不可变资源 ID；关联该仓库 common dir、worktree admin dir、工作根目录规范路径及可用的文件系统身份。Git 路径由 `rev-parse` 查询，不按 `.git/worktrees/<basename>` 拼装。

服务端至少核验：

1. 工作根目录存在，且确实是记录中的 linked worktree；不是主工作区、普通目录或裸仓库根。
2. common dir、admin dir、工作根与 Git worktree 登记相互一致；不跟随目标根的 symlink/junction 去删除另一个目录。
3. 目录没有移动、被替换、转为挂载点或不支持的网络/可移动布局。无法稳定识别时 fail-closed。
4. 目标不位于另一个 worktree 根内部，也不包围另一个登记的 worktree。历史 `repo/src-feature-a` 这种嵌套布局禁止从本功能清理。
5. 实际分支与保留引用有效；源记录与当前分支变化必须通过新预览展示。第一版对偏离来源快照的身份/分支变化保守阻止，不能“修正”记录后立即删除。
6. 应用操作目录、journal、数据库以及本次 Git 命令的 cwd 不在待删除目录内；由可信的存活仓库锚点执行 Git。找不到锚点时禁止清理。

Windows 展示路径可去掉 `\?\`，但存储/比较身份不能仅 lower-case 全路径。大小写敏感卷、UNC、盘符、junction、Unicode 和 Linux 大小写路径须单测。名称显示是标签，不是唯一键。

### 5.2 来源证据

新增普通会话 worktree：在外部 Git 创建前持久化 `provisioning` 来源意图；Git 成功后记录可核验身份，再把资源与会话绑定。记录意图失败则不执行 Git；Git 成功但绑定失败则记录 `needs_attention`，不在此变更自动回滚删除成果。创建功能正常返回前，来源绑定必须持久化。

历史会话：只有原有受信任会话 worktree 元数据、成功创建操作的结构化证据，以及当前 Git 身份全部对应，才可标记 `legacy_verified`。证据缺失、被保留策略淘汰、Loop/子 Agent 来源或只有命名前缀时，保持 `legacy_unverified` / `external` 并禁用清理。禁止批量仅凭 `vanehub/` 前缀授权。迁移本身不执行 Git/目录删除；历史核验是有界、按需的只读操作。

### 5.3 引用判定

目标清理前查询所有持久化与运行中引用，不能仅比较 session.worktreePath。普通会话可能通过 folder 指向该 worktree 或其子目录；按有效工作根解析后应计为同一资源。反之，两个会话共享原 projectPath 但使用不同 worktree 时，不构成对同一资源的引用。

引用包含：正常/归档/隐藏会话、运行中的生成/CLI/后台命令/Shell、Loop 的运行与持久化 review 所有权、定时任务及其他能够重新启动执行的持久化工作目录绑定。禁用但仍保存路径的任务仍算引用。失效且不可解析的相关引用不能静默跳过，标记检查不完整并阻止清理。

候选删除集合中的会话可以构成同一删除分组；只有同组所有相关会话都被选中且静止，才允许移除一次 worktree。其他资源的任务或非会话引用永远不能因为“此次批量选择”被忽略。不自动删除或解绑引用方。

派生索引、LSP、文件 watcher、UI 预览等可以作为可释放运行句柄处理，但必须等待停止并收到回执；失败则按占用处理。不能把可释放句柄误当作永远阻塞的业务引用，也不能不经等待就忽略它。

### 5.4 门禁而不是仅做两次 SELECT

预检查是快照，不是锁。真正执行需要持久化 session deletion claim 和 worktree-use gate。所有相关的创建/绑定、启动执行、任务修改、目录复用、Shell/工具写入入口必须在提交引用或开始执行前检查同一门禁，并使用同样的资源身份规则。

一个进程内 Mutex 不足以协调同一数据库的多个实例；使用仓库已有的跨进程设施，或实现应用范围的数据库 claim + 操作所有者心跳/epoch/OS 锁。不得仅 TTL 到期就允许另一个实例再次启动 Git remove。必须证明原 owner 和其子进程不再有效；证明不了就进入 needs_attention。

本门禁只保证 VaneHub 管理的参与者。执行前再检查、禁 force 和 Git 自身保护共同降低外部竞态风险，但并不提供对外部写入的原子快照保证。尤其忽略文件内容可在检查后被外部修改；文案不得宣称“绝对安全”，应要求停止外部使用。

## 6. Git 与文件系统预检查

### 6.1 只读探针

通过现有 GitAdapter 执行参数数组，禁用 shell 字符串拼接，隔离继承的 `GIT_DIR`、`GIT_WORK_TREE`、`GIT_INDEX_FILE` 等可能改变目标的环境。不得修改全局 Git 配置或 safe.directory。使用当前适配器支持的无可选写锁模式，禁用可触发外部程序的 fsmonitor；跨版本不支持所需安全能力时阻止清理。

建议的原生命令类别，具体 flags 须用仓库支持的 Git 版本验证：

```bash
git -C <trusted-anchor> worktree list --porcelain -z
git -C <target> rev-parse --show-toplevel
git -C <target> rev-parse --path-format=absolute --git-common-dir
git -C <target> rev-parse --absolute-git-dir
git -C <target> symbolic-ref -q HEAD
git -C <target> rev-parse --verify HEAD
git -C <target> status --porcelain=v1 -z --untracked-files=all --ignored=traditional --ignore-submodules=none
```

`-z` 输出按 NUL/字节解析，不能 split newline，不根据本地化 stderr 判定权限或分支身份。解析 rename 两路径、非 UTF-8 文件名、空格、换行、引号、路径前导横线、长路径。UI 可安全转义显示，删除身份不能从有损显示字符串反解。

### 6.2 文件安全条件

- tracked unstaged、staged、conflict、非忽略的 untracked 任意存在，禁止 remove-safe。
- ignored 文件不会因 Git 忽略规则而获得安全豁免。列出有限样例、总量完整性与元数据清单摘要；用户必须确认对应摘要。禁止把目录名 node_modules/target 当作无条件可删白名单。
- ignored 清单要覆盖内部文件/目录且不追踪外部符号链接；可用 lstat 元数据（相对路径、文件类型、大小、mtime 与可用 file id）形成有界摘要，不读取敏感文件内容。发现嵌套仓库、特殊挂载/重解析点或无法扫描的目录，禁止清理。
- 第一版对包含子模块、嵌套仓库、sparse-checkout、assume-unchanged/skip-worktree 等可能导致“干净”含义不完整的布局，返回 unsupported_layout。不得修改 index 标志来绕过检查。
- 仓库处于 merge/rebase/cherry-pick 等未结束状态，或登记 locked/prunable、目录不存在、admin 不一致，禁止清理。
- 扫描、输出或引用上限被触发时为 `incomplete`，不返回 `clean`；展示样例截断与后台实际扫描不完整是两个不同字段。

### 6.3 忽略文件确认失效

preview 保存 resource identity、HEAD/ref、tracked 状态摘要、ignored manifest、引用版本、选择集摘要、过期时间。确认绑定资源 ID 与 ignored manifest fingerprint，不绑定界面 index。

执行阶段停止活动后重算。受授权的运行停止导致的 lifecycle/消息变化，不应单独使计划过期；但文件内容状态、资源身份、忽略文件元数据、实际 HEAD/ref、相关引用或目标集合变化必须拒绝旧授权并刷新预览。不可用全局 session.updatedAt 相等作为唯一安全令牌。

### 6.4 唯一破坏性命令

```text
GitAdapter.execute(trusted_anchor, ["worktree", "remove", absolute_verified_target], timeout)
```

不添加 `-f`/`--force`，不调用 recursive delete、git clean/reset、git worktree prune/unlock/repair 或任何引用删除。Git 返回成功后重新核验原目标目录与登记都不在；返回失败或超时也要重新观察，不能假设零副作用。运行中命令超时须尝试有界终止并确认子进程退出；结果不确定时冻结重试并进入 needs_attention。

## 7. 数据与契约

以下名称是建议，允许按仓库现有原语映射，语义不可省略。迁移号由实施时当前顶号确定。

### 7.1 轻量持久化模型

| 数据 | 最小字段及不变量 |
| --- | --- |
| `managed_worktrees` | id、repository identity、canonical root/admin/common-dir identity、origin（ordinary_session/loop/subagent/external）、provenance、branch snapshot、creation operation id、status、revision、timestamps；不得由 session 删除级联移除 |
| worktree-session association | resource id、session id、binding kind；会话删除可移除此关联，但资源历史需保留不敏感来源 ID |
| `session_deletion_operations` | operation id、request id UNIQUE、规范化请求 hash、状态、phase、revision、owner/epoch、timestamps、错误码；不可依赖即将被删的会话外键 |
| deletion groups/items | 每个真实资源一个组或无资源会话组；session ID 集合、选择、可信执行快照、worktree effect、db effect、尝试号、receipt、error；活动 session claim 必须唯一 |
| preview store | 随机 opaque preview ID、受信任目标集合、完整性、资源快照、选择许可、fingerprints、expiresAt；可用短期内存存储，重启后旧 preview 失效 |

资源状态建议：`provisioning → attached → retained/removing → removed`，异常进入 `needs_attention`。只有已完成并受保留策略允许的操作才可被常规日志清理；有未完成删除、retained worktree 或 needs_attention 的最小恢复依据不能随普通日志淘汰。

删除会话后保留资源表不是承诺本版提供全局管理列表。用户可从操作结果复制保留路径；后续管理中心再使用这些记录。日志不保存聊天内容、文件正文或凭据，最小恢复快照置于应用数据库并遵循现有权限。

### 7.2 前端 TypeScript 契约草案

```ts
export type WorktreeDeletionPolicy = "keep" | "remove-safe";
export type CheckCompleteness = "complete" | "incomplete";

export interface PreviewSessionDeletionInput {
  sessionIds: string[]; // 去重，有界；仅 ID，不接受目录路径
}

export interface WorktreeDeletionChoice {
  worktreeId: string;
  policy: WorktreeDeletionPolicy;
  ignoredFilesAcknowledgement?: { fingerprint: string };
}

export interface ExecuteSessionDeletionInput {
  requestId: string; // 相同网络重试保持不变
  previewId: string;
  worktreeChoices: WorktreeDeletionChoice[];
}

export interface SessionDeletionService {
  previewSessionDeletion(input: PreviewSessionDeletionInput): Promise<SessionDeletionPreview>;
  executeSessionDeletion(input: ExecuteSessionDeletionInput): Promise<SessionDeletionHandle>;
  getSessionDeletionOperation(operationId: string): Promise<SessionDeletionOperation>;
  listPendingSessionDeletions(): Promise<SessionDeletionOperation[]>;
  retrySessionDeletion(input: RetrySessionDeletionInput): Promise<SessionDeletionHandle>;
}
```

`SessionDeletionPreview` 包含：runtimeEffect=`native|simulated`、previewId、expiresAt、选中会话快照、去重 worktree 行、外部引用方、allowedPolicies、checks completeness、ignored 摘要、typed reason codes。可展示路径，但执行请求只接受 opaque ID。

`SessionDeletionHandle`：operationId、runtimeEffect，必要时关联现有 OperationTask ID。返回 handle 不意味着会话或文件已删除。

`SessionDeletionOperation`：overall outcome=`pending|succeeded|failed|partial|needs_attention`、phase、revision、分组以及逐会话/资源结果。每个资源明确 effect=`not_requested|retained|remove_started|removed|removal_unknown`；每个会话 dbEffect=`pending|deleted|retained`。不使用单一 success boolean 覆盖部分副作用。

`RetrySessionDeletionInput`：operationId、expectedRevision、retryRequestId、新 previewId（再次清理必须提供）、本次 choices。只允许针对未完成组创建受约束的新 attempt，已成功组不重放。DB-only finalize retry 不接受新的磁盘目标。

旧 deleteSession(sessionId) 只走同一 coordinator 的 keep 模式，等待会话删除最终状态再 resolve。不能保留一个不检查 deletion claim 的旁路。UI 不再直接使用旧入口。

### 7.3 原生服务与端口建议

```text
SessionDeletionCoordinator::preview(session_ids)
SessionDeletionCoordinator::execute(confirmed_plan)
SessionDeletionCoordinator::get(operation_id)
SessionDeletionCoordinator::retry(retry_request)
SessionDeletionCoordinator::reconcile_pending()

WorkspaceCleanupPort::inspect(trusted_resource_id)
WorkspaceCleanupPort::remove_safely(verified_permit)
WorkspaceCleanupPort::observe_effect(execution_snapshot)
SessionDeletionRuntimePort::quiesce(session_ids, deadline)
WorkspaceUseGatePort::claim / authorize_use / release
DeletionJournalPort::create_once / cas_phase / append_receipt
```

`verified_permit` 是 Rust 内部、绑定资源身份/操作/epoch 的值，不是前端任意构造的布尔值。quiesce 必须等待生成、后台命令、Shell/CLI 与本应用派生观察器停止，不能把“取消已发送”当作终止证据。

## 8. 执行算法、幂等与批量

### 8.1 分组算法

先对 sessionIds 去重并限定数量；后端解析有效工作根和资源身份。一个 worktree 对应一个组，所有本次选中的引用会话进入同组，UI 为该 worktree 提供唯一策略。无 worktree 的会话各自成组或按实现做纯 DB 分组。未选中的引用会话/任务使目标不可清理。

输入混用同一资源的 keep/remove-safe 或重复 choices 时拒绝，不以最后一个覆盖前一个。与同一 common dir 关联的 Git 删除串行执行；第一版可整批串行以降低风险。批量只保证每组可解释和可恢复，不承诺所有磁盘副作用原子化。

### 8.2 正常流程

1. 验证 preview 未过期、目标和 choices 合法、requestId 与请求 hash 对应。
2. 一个短事务创建 journal、分组和目标 session claims；已被其他操作占用的目标明确冲突。相同 requestId+相同 hash 返回既有 handle；hash 不同拒绝。
3. 对将清理的资源取得独占门禁，阻止新增引用和本应用执行。对 keep 组只取得必要的 session 删除 claim，不冻结其他共享目录会话。
4. 停止本组会话活动，并确认本应用写入者已经退出；失败则本组保留会话与目录，不继续删除。
5. remove-safe 组重检身份、文件状态、忽略确认与引用；有新风险进入 awaiting_decision，释放可安全释放的门禁，不把旧 preview 当新授权。
6. 持久化 `remove_started` 和执行快照，再执行唯一允许的非 force Git remove。没有持久化成功则不执行命令。
7. 重新观察结果，持久化 `worktree_removed` receipt；如状态不确定进入 needs_attention，不触发第二次 remove。
8. 在一个 SQLite 事务中删除该组会话及原有级联数据、仅在活动会话属于此组时清空 active selection、更新关联/资源状态并将该组 journal 标记完成。
9. commit 后发布真实活动会话状态和逐项结果；事件发布失败可重发/查询恢复，不重新执行 Git。
10. 释放门禁；有不可确定副作用的资源继续隔离以待人工处理。其他独立组可以继续，最后聚合结果。

keep 组跳过文件状态要求与 Git 删除；即使 Git 不存在、源目录离线、或归属未知，也可在会话授权和运行停止成功后删除会话。共享引用只约束目录删除，不限制用户删除自己的会话记录。

### 8.3 失败时不夸大“回滚”

| 故障点 | 结果 |
| --- | --- |
| journal/claim 创建失败 | 不启动停止/删除副作用，不删除会话 |
| 停止超时 | 会话记录和目录保留，报告停止失败；已停的进程不自动重启 |
| Git 前复查失败 | 保留目录/会话，刷新授权，不自动 keep |
| Git 非零且目录完整 | failed/awaiting_decision，可重新预览；不能声称一定零副作用 |
| Git 非零、超时或 app 崩溃且部分路径/登记变化 | needs_attention，冻结自动重放 |
| Git 成功、receipt 持久化失败 | 利用 remove_started 及身份快照恢复观察，不直接再次 remove |
| receipt 成功、会话事务失败 | worktree_removed + finalize_pending；会话以不可执行的待完成删除状态展示 |
| 会话事务成功、前端丢响应 | 同 requestId 查询同一最终结果，不重放动作 |
| 事件发布失败 | 数据为真源；重新读取，不能删除其他活动会话状态 |

用户取消在提交前没有副作用。执行后不承诺任意阶段取消或撤销；第一版不开放 worktree remove 进行中的取消按钮，超时是系统故障处理而非用户撤销。

### 8.4 失败后的 claim 释放与新授权

`awaiting_decision` 表示当前 attempt 已停止、需要用户决定，不应无条件永久占用会话。Git 尚未开始，或已证明原目录/登记/身份完整且没有在途清理进程时，先持久化该 attempt 的失败/保留结果，再释放其 session deletion claims 和 worktree-use gate。旧 journal 保留用于审计。会话是否可再次执行仍由正常 runtime/lifecycle 判断；释放删除 claim 不代表把仍在运行的生成标成已停止。

此后「重新检查」「重试」或「改为仅删除会话」形成新的授权 attempt，必须重新取得 claims、核实原会话仍存在且没有被其他操作接管。同一网络请求重传仍返回原结果；改变策略不能沿用同一 requestId。不能因为旧失败操作持有 claim 而使用户永远无法继续。

若 worktree 已移除或效果不明，保留 finalize_pending/needs_attention 和必要隔离；不允许通过关闭弹窗、改为 keep 或租约过期绕过。只有可靠观察结果允许解锁或收尾。受管理进程未确认退出的情况，不得启动第二次 remove。

## 9. 重启恢复与生命周期交互

journal 是独立状态，不给现有 SessionLifecycle 增加随意的 deleting 字符串。查询层可追加 `deletionState`/关联 operationId，启动入口统一检查 claims；发送消息、启动 Shell/CLI、切换参与者并启动工作都不能绕过删除门禁。

启动恢复先确认前一 owner/进程已失效，获取恢复权限，再只读观察：

- `worktree_removed` 已记录：验证可信仓库可用、原目录与登记仍不存在、会话未被重新绑定；满足条件可仅完成原授权的数据库事务，不再执行 Git。
- `remove_started` 无 receipt：两者都不存在且仓库/身份可核验时记录 `removed_observed_after_interruption`，完成原授权的 DB finalize；此为观测到的效果，不伪称获得原 Git 成功返回码。
- 目录与登记仍完整：转 awaiting_decision，必须新 preview 后才能再次执行 Git；启动扫描不自动发起新的破坏性动作。
- 仅一方存在、目标被同名重新创建、目录已替换、磁盘离线、身份不明：needs_attention，不删除、不 prune、不修复。
- keep 操作没有未决磁盘副作用：可按原授权幂等完成会话删除或记录具体 DB 错误。

不能用不存在的路径当作“已经删成功”的充分条件；磁盘离线和权限拒绝必须与确定不存在区分。若同名目录被重建，恢复绝不删除新对象。恢复中的 session 保持不可执行，直到完成或明确安全取消操作。

与自动归档、会话恢复、调度器和多窗口同时运行时，以删除 claim 为优先仲裁条件。自动归档不能清除该 claim 或把 finalize_pending 会话恢复成可执行状态。删除非当前会话不能无条件发 active-session-changed(null)。

## 10. 建议实现模块

新增模块名称可按项目结构调整，禁止把数百行逻辑堆回 main-layout 或现有巨型 adapter。

```text
src/types/session-deletion.ts
src/services/session-deletion-service.ts
src/services/tauri-session-deletion-client.ts
src/services/web-session-deletion-client.ts
src/main-layout/session-deletion-dialog.tsx
src/main-layout/session-deletion-worktree-row.tsx
src/main-layout/session-deletion-result.tsx
src/main-layout/use-session-deletion.ts

src-tauri/src/commands/sessions/preview_session_deletion.rs
src-tauri/src/commands/sessions/execute_session_deletion.rs
src-tauri/src/commands/sessions/get_session_deletion_operation.rs
src-tauri/src/commands/sessions/list_pending_session_deletions.rs
src-tauri/src/commands/sessions/retry_session_deletion.rs
src-tauri/src/contexts/sessions/application/deletion_*.rs
src-tauri/src/contexts/sessions/infrastructure/deletion_*.rs
src-tauri/src/contexts/workspaces/application/worktree_cleanup.rs
src-tauri/src/contexts/workspaces/infrastructure/worktree_*.rs
```

新增命令还要更新命令 registry、DTO/生成契约、权限声明（按当前 Tauri capability 配置核实）、Web mock 组成、错误规范化及测试，不得只写 command 文件而未注册。

## 11. 日志、预算与性能

统一日志类别建议 `session.delete`、`git.worktree.cleanup`。记录 operationId、requestId、group/resource ID、phase、reason code、耗时、退出码、检查完整性和效果，不保存文件内容、凭据、完整进程环境。原始 Git 输出只进受限脱敏诊断；UI 展示结构化错误。

建议初始预算作为可测试常量，而非已测性能承诺：批次最多 100 会话，预览最多并行 2 个仓库，UI 样例最多 100 条，清单扫描最多 10000 条/2 MiB 元数据，单资源预览 deadline 10 秒，运行停止 deadline 15 秒，Git remove deadline 30 秒。遇到实际仓库超限返回 incomplete/timeout，不提高权限、不无限等待。实施可按现有平台标准调整数值，但必须同步规格测试和说明，不得删掉上限。

不要在 UI 渲染时反复跑 git status；按本次 preview 请求与资源去重。动态进度只显示可确知阶段/已完成组数，未知磁盘容量不显示虚假的回收空间值。

## 12. 验证与落地顺序

先契约/纯策略和真实 Git 探针，再来源记录/门禁/journal，再协调器与恢复，再 UI 与两端 adapter，最后批量与桌面集成。可分提交实施，但在后端保护未完成时不发布可点的清理复选框。

验收表和故障注入见 [acceptance-tests.md](acceptance-tests.md)；完整任务见 [tasks.md](tasks.md)。测试只能使用隔离临时仓库、临时数据库和固定 CLI 桩，不使用用户真实项目、凭据、外部模型或真实工作目录执行删除。先跑针对性测试，再执行根 AGENTS.md 的实际门禁。三平台结果独立记录 PASSED/FAILED/BLOCKED/NOT RUN。

## 13. ADR 摘要

- **ADR-01：默认 keep。** 删除聊天数据不暗含删除代码；不用记忆型危险选项。
- **ADR-02：目录与分支解耦。** 第一版保留分支，避免引入合并目标推断与成果丢失。
- **ADR-03：身份和来源分离。** Git 证明是什么，来源记录证明为何允许应用代为清理；两者不可相互替代。
- **ADR-04：非 force 和不自动降级。** 失败可见优先于“尽量删除成功”。
- **ADR-05：外部副作用 journal。** Git + SQLite 不是单事务；记录真实效果并向前恢复。
- **ADR-06：按资源分组。** 批量不是循环点击单条删除，处理共享引用和去重是后端职责。
- **ADR-07：门禁限制承诺。** 协调应用内参与者，不冒充 OS 沙箱或外部写入锁。
- **ADR-08：历史数据保守核验。** 无证据不授权；不根据惯用目录名或旧 UI 显示推断可删。
