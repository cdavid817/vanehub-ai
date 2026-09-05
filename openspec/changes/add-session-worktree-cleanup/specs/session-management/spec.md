# session-management Delta Specification

## MODIFIED Requirements

### Requirement: Session mutation operations

<!-- S-01; acceptance: TC-001, TC-002, TC-003, TC-004, TC-005, TC-006, TC-007 -->
The system SHALL provide service operations to rename, pin, unpin, archive, unarchive, and delete sessions.
删除会话 SHALL 默认保留项目目录、Git worktree 和所有分支；只有经过用户显式选择且满足工作区清理规范的普通会话 worktree 才允许安全移除。删除 SHALL 以实际完成结果为准，而不是以异步命令受理为准。

#### Scenario: Rename session
- **WHEN** a user renames a session to a non-empty title
- **THEN** the system SHALL update the session title and updated timestamp

#### Scenario: Pin and unpin session
- **WHEN** a user pins or unpins a session
- **THEN** the system SHALL update the pinned flag and updated timestamp

#### Scenario: Archive active session
- **WHEN** a user archives the active session
- **THEN** the system SHALL mark the session archived and clear the active session selection

#### Scenario: Restore archived session
- **WHEN** a user restores an archived session
- **THEN** the system SHALL mark the session unarchived and keep the session available for normal listing and selection

#### Scenario: Delete active session
- **WHEN** a user deletes the active session
- **THEN** the system SHALL remove the session and clear the active session selection

#### Scenario: Preserve unrelated active session
- **WHEN** 删除目标不包含当前活动会话且会话删除事务提交成功
- **THEN** 系统 SHALL 保持原活动会话，不发布错误的 active-session-changed(null)

#### Scenario: Archive preserves worktree
- **WHEN** 用户归档或系统自动归档一个带 worktree 的会话
- **THEN** 系统 SHALL 保留 worktree 目录、Git 登记和分支，不触发本变更的清理流程

### Requirement: UI-driven multi-session deletion

<!-- S-09; acceptance: TC-096, TC-097, TC-098, TC-099 -->
The system SHALL support deleting multiple sessions from the session management UI through the frontend service boundary while preserving existing single-session deletion semantics. 批量删除 SHALL 作为一个预览/执行请求提交全部选中的 session id，由后端按真实资源分组；UI SHALL 不再逐个调用旧的单条删除接口。

#### Scenario: Delete selected sessions
- **WHEN** the user confirms deletion of multiple selected sessions
- **THEN** the UI SHALL request one deletion preview and one deletion execution through the frontend session deletion service carrying every selected session id
- **AND** React components SHALL NOT call Tauri `invoke()` or SQLite directly

#### Scenario: Refresh after multi-session deletion
- **WHEN** one or more selected sessions are deleted
- **THEN** the UI SHALL refresh active-visible sessions, archived sessions, active-session state, and workflow state

#### Scenario: Delete active session in batch
- **WHEN** the selected batch includes the active session
- **THEN** deletion SHALL clear the active session selection according to the existing active-session deletion behavior
- **AND** 当批量不包含活动会话时 SHALL 保持活动会话不变

#### Scenario: Report batch deletion failure
- **WHEN** deletion of one or more selected sessions fails
- **THEN** the UI SHALL show localized failure feedback
- **AND** it SHALL refresh session state so successful deletions and retained sessions are visible
- **AND** 失败的会话 SHALL 在批量选择中保留，可再次发起

## ADDED Requirements

### Requirement: Explicit session deletion confirmation

<!-- S-02; acceptance: TC-008, TC-009, TC-010, TC-011, TC-012 -->
所有用户可见的会话删除入口 SHALL 使用同一确认流程。普通项目或远程会话 SHALL 说明只删除会话数据；带 worktree 的会话 SHALL 显示关联信息并在核验通过时提供默认未选择的清理选项。确认按钮 SHALL 反映实际选择，不使用语义不明的“确定”。

#### Scenario: Delete a normal project session
- **WHEN** 用户从普通项目会话发起删除
- **THEN** 界面 SHALL 显示会话及聊天记录删除确认
- **AND** 界面 SHALL 明确项目目录不删除且不提供 worktree 清理选择

#### Scenario: Delete a remote session
- **WHEN** 用户从远程工作区会话发起删除
- **THEN** 界面 SHALL 明确远程目录不删除
- **AND** 预览与执行 SHALL 不为目录清理建立 SSH 操作

#### Scenario: Default keep for a worktree session
- **WHEN** 用户打开带 worktree 的会话删除确认框
- **THEN** 清理选项 SHALL 默认未选，确认按钮 SHALL 为“仅删除会话”
- **AND** 界面 SHALL 展示规范化显示路径、分支、检查完整性与引用说明

#### Scenario: Choose worktree cleanup
- **WHEN** 用户在核验通过的普通会话 worktree 行选中清理
- **THEN** 确认按钮 SHALL 显示“删除会话及 worktree”
- **AND** 说明 SHALL 明确工作目录及 Git 登记将移除而分支保留

#### Scenario: Reset destructive consent
- **WHEN** 用户重新打开弹窗、改变删除目标或收到失效的 preview
- **THEN** 清理选择及忽略文件确认 SHALL 重置
- **AND** 系统 SHALL 不从 localStorage、设置或上次操作恢复破坏性选择

### Requirement: Read-only deletion preview interaction

<!-- S-03; acceptance: TC-013, TC-014, TC-015 -->
打开、刷新或取消删除确认 SHALL 不停止 Agent、不启动清理、不删除会话、不修改工作区或 Git 配置。预检查失败 SHALL 不被渲染为“干净”。只删除会话 SHALL 不依赖 Git 检查成功，但仍受会话授权与停止完成条件约束。

#### Scenario: Cancel before confirmation
- **WHEN** 用户只打开预览后取消
- **THEN** 所有会话、受管理运行、目录、登记和分支 SHALL 保持未被本次预览修改

#### Scenario: Git is unavailable
- **WHEN** Git 缺失或目录检查失败但会话仍可以读取
- **THEN** 清理选项 SHALL 禁用并给出原因
- **AND** 仅删除会话 SHALL 仍可显式确认并按正常删除流程执行

#### Scenario: Preview remains loading
- **WHEN** worktree 检查尚未完成
- **THEN** 界面 SHALL 显示检查中并禁止清理选择
- **AND** 界面 SHALL 不把未知引用数显示为已确认的零

### Requirement: Ignored file deletion acknowledgement

<!-- S-04; acceptance: TC-016, TC-017, TC-018 -->
界面 SHALL 将忽略文件与未跟踪修改分开呈现。存在忽略文件且扫描完整时，清理 SHALL 额外要求用户确认已检查并备份必要文件。确认 SHALL 绑定当前资源和预览清单摘要，而非全局布尔值。

#### Scenario: Ignored configuration requires acknowledgement
- **WHEN** worktree 没有普通修改但包含被忽略的本地配置
- **THEN** 界面 SHALL 显示忽略文件也会被删除并提供有界文件路径样例
- **AND** 没有独立确认时 SHALL 禁止带 remove-safe 的提交

#### Scenario: Ignored inventory changes
- **WHEN** 用户确认后忽略文件清单或元数据发生变化
- **THEN** 旧确认 SHALL 失效并要求刷新和再次确认
- **AND** 系统 SHALL 不沿用之前确认执行删除

#### Scenario: Ignored inventory is incomplete
- **WHEN** 忽略文件扫描因上限、权限或不可支持的布局无法完成
- **THEN** 系统 SHALL 禁止清理且明确 incomplete 原因
- **AND** 界面 SHALL 不声称列表中的文件就是全部文件

### Requirement: Observable deletion progress and result

<!-- S-05; acceptance: TC-019, TC-020, TC-021, TC-022 -->
会话删除 SHALL 显示受理、执行阶段和实际结果。收到 operation handle SHALL 不被解释为删除成功。失败 SHALL 保留对应目标与结果，不自动切换到 keep；执行状态 SHALL 不因弹窗关闭而丢失。

#### Scenario: Operation accepted but pending
- **WHEN** 执行接口返回操作 ID 而任务尚未完成
- **THEN** 界面 SHALL 展示进行中而不是移除所有会话行并提示成功
- **AND** 重复提交 SHALL 被禁用或归并为同一请求

#### Scenario: Cleanup fails without confirmed removal
- **WHEN** Git 清理失败且复查确认目录和登记仍完整
- **THEN** 界面 SHALL 显示清理失败并保留会话
- **AND** 界面 MAY 提供重新检查或显式改为仅删除会话，不得自动降级

#### Scenario: Directory removed but finalization pending
- **WHEN** 目录已确认移除而会话数据库事务尚未成功
- **THEN** 界面 SHALL 显示待完成删除与操作关联
- **AND** 该会话 SHALL 不允许开始新执行，且不假装目录已恢复

#### Scenario: Dialog closes during work
- **WHEN** 操作执行时允许用户关闭对话框
- **THEN** 系统 SHALL 在可持续访问的操作面板保留进度和结果
- **AND** 没有该可观察入口的实现 SHALL 禁止进行中关闭，而不是遗失句柄

### Requirement: Unified deletion entrypoints and grouped batch review

<!-- S-06; acceptance: TC-023, TC-024, TC-025 -->
侧栏单条、右键菜单、搜索结果、归档列表和批量删除 SHALL 进入相同预览/授权/执行服务。批量确认 SHALL 在一个界面中按真实 worktree 身份展示唯一清理选择，并显示逐项结果。

#### Scenario: Open from every visible entrypoint
- **WHEN** 从任一可见会话入口请求删除
- **THEN** 系统 SHALL 打开统一确认，且未确认前不调用旧直接删除接口

#### Scenario: Batch shares one worktree
- **WHEN** 本次选中的多个会话实际使用同一 worktree
- **THEN** 界面 SHALL 只展示该资源的一份清理选择并列出相关会话
- **AND** 实际执行 SHALL 最多移除该 worktree 一次

#### Scenario: Batch partially fails
- **WHEN** 一组清理成功而另一组被引用或发生错误
- **THEN** 界面 SHALL 展示逐会话/资源结果且整体不能显示全部成功
- **AND** 失败组 SHALL 保留为可重试或待决目标，不在发起回调后立即清空所有选择

### Requirement: Accessible and localized deletion controls

<!-- S-07; acceptance: TC-026, TC-027 -->
删除界面 SHALL 复用项目对话框和样式约束，支持键盘、焦点管理、屏幕阅读器及项目现有语言集。路径显示 SHALL 与执行身份分离。

#### Scenario: Keyboard and assistive technology
- **WHEN** 用户用键盘或屏幕阅读器打开和操作删除弹窗
- **THEN** 界面 SHALL 提供明确标题、取消焦点、焦点约束/回送及状态播报
- **AND** 普通输入区域 SHALL 不隐式触发破坏性提交，复选框必须能通过键盘操作

#### Scenario: Long and platform-specific paths
- **WHEN** 路径包含中文、空格、Windows extended-length 前缀或较长文件名
- **THEN** 界面 SHALL 提供可复制的完整显示路径和有限宽度布局
- **AND** 身份核验 SHALL 不使用被截断、转义或有损转换后的 UI 文本

### Requirement: Session deletion service isolation and legacy keep behavior

<!-- S-08; acceptance: TC-028, TC-029 -->
React SHALL 仅依赖类型化 service；Tauri invoke SHALL 留在对应 adapter。现有只接受 sessionId 的内部删除调用 SHALL 保持仅删除会话的语义并经过同一停止与 claim 仲裁。所有 UI 入口 SHALL 使用显式确认 API。

#### Scenario: Legacy caller deletes a session
- **WHEN** 仍存活的内部调用使用 deleteSession(sessionId)
- **THEN** 系统 SHALL 保留 worktree 和所有分支
- **AND** 该调用 SHALL 不能绕过进行中的删除 claim 或在会话数据未删除前假报成功

#### Scenario: Frontend uses adapters
- **WHEN** React 发起删除预览、执行或重试
- **THEN** 组件 SHALL 只调用 service，不能直接 invoke、运行 Git 或访问 SQLite
- **AND** Desktop 与 Web/mock adapter SHALL 实现相同 DTO 契约
