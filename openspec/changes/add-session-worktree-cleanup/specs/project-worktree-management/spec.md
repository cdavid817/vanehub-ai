# project-worktree-management Delta Specification

## ADDED Requirements

### Requirement: Ordinary session worktree provenance

<!-- W-01; acceptance: TC-030, TC-031, TC-032, TC-033 -->
系统 SHALL 维护独立于会话生命周期的轻量 worktree 来源记录。新普通会话 worktree SHALL 在 Git 创建前记录意图、成功后记录可验证身份并与会话绑定。历史记录 SHALL 仅在已有可信来源证据和当前 Git 身份完整对应时成为可清理资源。

#### Scenario: Register a newly created worktree
- **WHEN** 普通会话创建新的 Git worktree
- **THEN** 系统 SHALL 持久化创建意图、Git 身份、来源类型和会话绑定
- **AND** 记录创建意图失败时 SHALL 不执行 Git add

#### Scenario: Session persistence fails after Git creation
- **WHEN** Git worktree 创建成功但会话绑定或来源身份写入失败
- **THEN** 系统 SHALL 保留可恢复的创建意图并标记 needs_attention
- **AND** 系统 SHALL 不自动递归删除目录或把未知资源当成可清理资源

#### Scenario: Verify a legacy session worktree
- **WHEN** 历史会话拥有可信成功创建操作、完整元数据及对应的 Git 身份
- **THEN** 系统 SHALL 可以记录 legacy_verified 来源后执行常规安全检查
- **AND** 仅名称匹配 SHALL 不能替代来源证据

#### Scenario: Legacy or external ownership is unproven
- **WHEN** 资源只有 vanehub 前缀、目录名、孤立 worktreePath 或外部创建证据
- **THEN** 系统 SHALL 将清理策略限制为 keep
- **AND** 历史迁移 SHALL 不执行任何磁盘清理

### Requirement: Verified linked worktree identity

<!-- W-02; acceptance: TC-034, TC-035, TC-036, TC-037 -->
安全移除 SHALL 同时验证可信资源来源、canonical worktree root、Git common/admin directory、Git 登记及当前文件系统身份。主工作区、普通项目、根路径替换、symlink/junction 跳转和身份不一致 SHALL 被拒绝。执行目标 SHALL 来自后端可信资源，而不是客户端任意路径。

#### Scenario: Reject an ordinary or main workspace
- **WHEN** 目标是主 worktree、普通目录或裸仓库根
- **THEN** 系统 SHALL 拒绝目录清理并保持原目录内容不变

#### Scenario: Reject a replaced root
- **WHEN** 预览后同路径目录被替换、移动或改为符号链接/junction
- **THEN** 系统 SHALL 拒绝执行旧授权
- **AND** 系统 SHALL 不删除链接目标或新建的同名目录

#### Scenario: Resolve Git metadata rather than guessing
- **WHEN** worktree admin directory 名与目录 basename 不同或 common dir 独立
- **THEN** 系统 SHALL 使用 Git 查询结果进行身份核验
- **AND** 系统 SHALL 不通过拼接 .git/worktrees/<name> 推测删除对象

#### Scenario: Do not trust arbitrary client paths
- **WHEN** 客户端试图通过未知资源 ID 或附加 path 改变目标
- **THEN** 服务端 SHALL 拒绝非法请求，且不执行文件系统删除

### Requirement: Worktree topology and branch preservation checks

<!-- W-03; acceptance: TC-038, TC-039, TC-040, TC-041 -->
普通会话清理 SHALL 仅支持已验证的独立 linked worktree，其 attached HEAD SHALL 被仍存在的本地分支准确引用。本功能 SHALL 不修改或删除任何分支、tag、stash 或提交。嵌套、锁定或不完整 Git 布局 SHALL 保守拒绝。

#### Scenario: Preserve unmerged committed work
- **WHEN** worktree 干净、attached HEAD 由实际本地分支持有且未合并到其他分支
- **THEN** 在其他检查通过后系统 SHALL 允许移除工作目录
- **AND** 该分支和原提交 SHALL 保留，不要求自动合并也不删除引用

#### Scenario: Reject detached or changed identity
- **WHEN** worktree 处于 detached HEAD、实际分支不匹配来源，或保留分支不能解析到 HEAD
- **THEN** 系统 SHALL 拒绝清理且不根据旧 worktreeBranch 假设提交已被保留

#### Scenario: Reject nested worktree layouts
- **WHEN** 目标位于另一个 worktree 内部或包围另一个 worktree
- **THEN** 系统 SHALL 拒绝快捷清理，包括历史 repo/src-feature-a 布局

#### Scenario: Reject locked or unsupported layouts
- **WHEN** 目标 locked/prunable，含子模块、嵌套仓库、特殊挂载、稀疏检出或会掩盖修改的 index 标志
- **THEN** 系统 SHALL 返回结构化阻止原因
- **AND** 系统 SHALL 不执行 unlock、repair、修改 index 标志或使用 force 绕过

### Requirement: Complete worktree change inspection

<!-- W-04; acceptance: TC-042, TC-043, TC-044, TC-045 -->
清理前系统 SHALL 区分 tracked unstaged、staged、unmerged、untracked 和 ignored。任一非忽略修改存在时 SHALL 禁止 remove-safe。检查失败、不完整或超限 SHALL 不等价于没有修改。

#### Scenario: Reject modified tracked files
- **WHEN** worktree 包含已跟踪文件修改或暂存区修改
- **THEN** 系统 SHALL 阻止清理并保留会话与目录直到用户重新决定

#### Scenario: Reject untracked files and conflicts
- **WHEN** worktree 有非忽略未跟踪文件、冲突或未完成合并/变基状态
- **THEN** 系统 SHALL 阻止清理，不自动提交、stash、reset 或 clean

#### Scenario: Bounded parsing is incomplete
- **WHEN** Git 输出被截断、解析失败或探针超时
- **THEN** 系统 SHALL 返回 incomplete 或检查失败
- **AND** 系统 SHALL 不以已解析的空集合判定清理安全

#### Scenario: Handle unusual filenames
- **WHEN** 状态或 worktree 路径含换行、空格、引号、Unicode、前导横线或非 UTF-8 字节
- **THEN** 系统 SHALL 使用 NUL/字节安全解析并维护真实路径身份
- **AND** 有损展示 SHALL 不影响执行目标，无法安全处理的平台 SHALL 拒绝清理

### Requirement: Ignored inventory authorization binding

<!-- W-05; acceptance: TC-046, TC-047, TC-048 -->
ignored 文件存在时，remove-safe SHALL 要求完整且有界的文件元数据清单及绑定当前资源/清单摘要的显式确认。系统 SHALL 不读取敏感文件正文来生成清单，也 SHALL 不把忽略规则作为可删除白名单。

#### Scenario: Acknowledge the current ignored inventory
- **WHEN** 当前 worktree 仅含忽略文件且所有扫描与其他安全检查完成
- **THEN** 只有匹配本资源及当前摘要的确认 SHALL 允许清理
- **AND** 系统 SHALL 不将确认自动用于另一个资源

#### Scenario: Do not enumerate private content
- **WHEN** 扫描 .env 等忽略文件以提示删除风险
- **THEN** 系统 SHALL 只返回必要路径/元数据及完整性
- **AND** 日志和前端 SHALL 不接收这些文件的正文或秘密值

#### Scenario: Ignore files change during stop
- **WHEN** 停止 Agent 的过程中新增或修改忽略文件元数据
- **THEN** 系统 SHALL 使旧确认失效并返回新的风险预览
- **AND** 系统 SHALL 不因已取得停止回执而跳过文件复查

### Requirement: Complete worktree reference protection

<!-- W-06; acceptance: TC-049, TC-050, TC-051, TC-052 -->
清理 SHALL 检查目标的持久化及活跃引用，包括会话、归档会话、Loop 审查所有权、定时任务、后台命令和 Shell。引用 SHALL 按有效 worktree 身份解析；未选中的会话及非会话业务引用 SHALL 阻止清理。检查不完整 SHALL 阻止清理。

#### Scenario: A second session opens the worktree as a folder
- **WHEN** 另一个会话以 folder 或其子目录使用该 worktree 但没有 worktreePath 元数据
- **THEN** 该会话 SHALL 被识别为引用并阻止目标清理

#### Scenario: Two sessions share only the source project
- **WHEN** 两个会话使用不同 worktree 但 projectPath 相同
- **THEN** 系统 SHALL 不仅因原项目路径相同就把它们视为同一 worktree 引用

#### Scenario: A dormant binding still uses the directory
- **WHEN** 归档会话、禁用的定时任务或 Loop review 记录仍绑定目标
- **THEN** 系统 SHALL 将其列为引用并阻止清理
- **AND** 系统 SHALL 不自动删除或解绑这些引用

#### Scenario: References cannot be fully resolved
- **WHEN** 相关引用查询或路径解析失败
- **THEN** 系统 SHALL 返回引用检查不完整并禁止清理

### Requirement: Non-forced Git worktree removal only

<!-- W-07; acceptance: TC-053, TC-054, TC-055, TC-056 -->
本能力唯一允许的磁盘清理动作 SHALL 是对已验证目标执行非 force 的 git worktree remove。命令 SHALL 从目标外的可信存活仓库锚点执行，并具有有界超时及退出确认。结果 SHALL 以退出信息与实际目录/登记观测共同判定。

#### Scenario: Remove a validated worktree
- **WHEN** 用户授权 remove-safe 且最终核验通过
- **THEN** 系统 SHALL 只执行不带 -f/--force 的 Git worktree remove
- **AND** 目录及登记确认移除后 SHALL 保留全部已有 Git 引用

#### Scenario: Git refuses removal
- **WHEN** Git 返回非零、目录占用或权限不足
- **THEN** 系统 SHALL 记录并重新观察效果
- **AND** 系统 SHALL 不改用 recursive delete、prune、clean、reset 或 unlock

#### Scenario: Git times out with uncertain effect
- **WHEN** Git remove 超时且无法证明命令退出或资源最终状态
- **THEN** 系统 SHALL 进入 needs_attention 并阻止重复清理
- **AND** 系统 SHALL 不假定没有副作用或自动启动第二个 remove

#### Scenario: No surviving execution anchor
- **WHEN** 无法确定目标外的可信仓库锚点或 journal 位于待删目录内
- **THEN** 系统 SHALL 拒绝清理而非从待删目录执行删除

### Requirement: Retained worktree resources outlive sessions

<!-- W-08; acceptance: TC-057, TC-058, TC-059 -->
用户选择 keep 时，目录、登记、分支和最小资源来源记录 SHALL 保留。会话删除 SHALL 不级联清除未处理 worktree 的资源与恢复记录。资源记录不代表会话聊天数据继续保留。

#### Scenario: Keep a managed worktree
- **WHEN** 会话删除成功且该资源未选清理
- **THEN** 系统 SHALL 保留目录、登记和分支并记录 retained 状态
- **AND** 结果 SHALL 提供已保留路径，不声称磁盘空间已释放

#### Scenario: Delete session records with associated resource history
- **WHEN** 数据库事务删除会话及其消息
- **THEN** 最小 worktree 来源/状态和未完成删除 journal SHALL 不被 session 外键级联删除

#### Scenario: Preserve Loop and temporary subagent policies
- **WHEN** 普通会话删除流程遇到 Loop 或子 Agent 所有的 worktree
- **THEN** 本能力 SHALL 不执行这些目录的清理
- **AND** 现有 Loop 保留和子 Agent 独立回收机制 SHALL 保持各自边界
