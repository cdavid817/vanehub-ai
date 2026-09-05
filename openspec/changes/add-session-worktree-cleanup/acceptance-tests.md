# 验收与故障注入矩阵

状态：以下是必须实现/执行的测试计划，全部尚未在项目中运行。包生成时只检查映射完整性，不把它视为实现测试通过。

## 1. 证据等级

纯策略测试用于决定 allowedPolicies；adapter 测试用于 DTO/错误/模拟一致性；真实 Git integration 用于目录/登记/分支事实；桌面 E2E 用于实际 IPC 和对话框。四者不能相互替代。

只使用系统临时目录内新建的仓库、专用数据库、独立操作目录和固定 CLI 桩。每个 fixture 需记录 base HEAD、branch refs、worktree list、主目录内容摘要及目标外哨兵；执行后验证未授权对象未变。禁用依赖用户全局身份、凭据、模型服务和生产数据库的测试路径。测试清理只允许处理自己创建并登记的临时根。

## 2. 需求到场景的一一映射

每项 test ID 对应 specs 中同名 Scenario。实现报告填写实际测试文件和名称，不只写“覆盖”。完整机器映射见 [traceability.json](traceability.json)。

| ID | 需求 | 场景 | 必须证明的结果 |
| --- | --- | --- | --- |
| TC-001 | S-01 | Rename session | the system SHALL update the session title and updated timestamp |
| TC-002 | S-01 | Pin and unpin session | the system SHALL update the pinned flag and updated timestamp |
| TC-003 | S-01 | Archive active session | the system SHALL mark the session archived and clear the active session selection |
| TC-004 | S-01 | Restore archived session | the system SHALL mark the session unarchived and keep the session available for normal listing and selection |
| TC-005 | S-01 | Delete active session | the system SHALL remove the session and clear the active session selection |
| TC-006 | S-01 | Preserve unrelated active session | 系统 SHALL 保持原活动会话，不发布错误的 active-session-changed(null) |
| TC-007 | S-01 | Archive preserves worktree | 系统 SHALL 保留 worktree 目录、Git 登记和分支，不触发本变更的清理流程 |
| TC-008 | S-02 | Delete a normal project session | 界面 SHALL 显示会话及聊天记录删除确认；界面 SHALL 明确项目目录不删除且不提供 worktree 清理选择 |
| TC-009 | S-02 | Delete a remote session | 界面 SHALL 明确远程目录不删除；预览与执行 SHALL 不为目录清理建立 SSH 操作 |
| TC-010 | S-02 | Default keep for a worktree session | 清理选项 SHALL 默认未选，确认按钮 SHALL 为“仅删除会话”；界面 SHALL 展示规范化显示路径、分支、检查完整性与引用说明 |
| TC-011 | S-02 | Choose worktree cleanup | 确认按钮 SHALL 显示“删除会话及 worktree”；说明 SHALL 明确工作目录及 Git 登记将移除而分支保留 |
| TC-012 | S-02 | Reset destructive consent | 清理选择及忽略文件确认 SHALL 重置；系统 SHALL 不从 localStorage、设置或上次操作恢复破坏性选择 |
| TC-013 | S-03 | Cancel before confirmation | 所有会话、受管理运行、目录、登记和分支 SHALL 保持未被本次预览修改 |
| TC-014 | S-03 | Git is unavailable | 清理选项 SHALL 禁用并给出原因；仅删除会话 SHALL 仍可显式确认并按正常删除流程执行 |
| TC-015 | S-03 | Preview remains loading | 界面 SHALL 显示检查中并禁止清理选择；界面 SHALL 不把未知引用数显示为已确认的零 |
| TC-016 | S-04 | Ignored configuration requires acknowledgement | 界面 SHALL 显示忽略文件也会被删除并提供有界文件路径样例；没有独立确认时 SHALL 禁止带 remove-safe 的提交 |
| TC-017 | S-04 | Ignored inventory changes | 旧确认 SHALL 失效并要求刷新和再次确认；系统 SHALL 不沿用之前确认执行删除 |
| TC-018 | S-04 | Ignored inventory is incomplete | 系统 SHALL 禁止清理且明确 incomplete 原因；界面 SHALL 不声称列表中的文件就是全部文件 |
| TC-019 | S-05 | Operation accepted but pending | 界面 SHALL 展示进行中而不是移除所有会话行并提示成功；重复提交 SHALL 被禁用或归并为同一请求 |
| TC-020 | S-05 | Cleanup fails without confirmed removal | 界面 SHALL 显示清理失败并保留会话；界面 MAY 提供重新检查或显式改为仅删除会话，不得自动降级 |
| TC-021 | S-05 | Directory removed but finalization pending | 界面 SHALL 显示待完成删除与操作关联；该会话 SHALL 不允许开始新执行，且不假装目录已恢复 |
| TC-022 | S-05 | Dialog closes during work | 系统 SHALL 在可持续访问的操作面板保留进度和结果；没有该可观察入口的实现 SHALL 禁止进行中关闭，而不是遗失句柄 |
| TC-023 | S-06 | Open from every visible entrypoint | 系统 SHALL 打开统一确认，且未确认前不调用旧直接删除接口 |
| TC-024 | S-06 | Batch shares one worktree | 界面 SHALL 只展示该资源的一份清理选择并列出相关会话；实际执行 SHALL 最多移除该 worktree 一次 |
| TC-025 | S-06 | Batch partially fails | 界面 SHALL 展示逐会话/资源结果且整体不能显示全部成功；失败组 SHALL 保留为可重试或待决目标，不在发起回调后立即清空所有选择 |
| TC-026 | S-07 | Keyboard and assistive technology | 界面 SHALL 提供明确标题、取消焦点、焦点约束/回送及状态播报；普通输入区域 SHALL 不隐式触发破坏性提交，复选框必须能通过键盘操作 |
| TC-027 | S-07 | Long and platform-specific paths | 界面 SHALL 提供可复制的完整显示路径和有限宽度布局；身份核验 SHALL 不使用被截断、转义或有损转换后的 UI 文本 |
| TC-028 | S-08 | Legacy caller deletes a session | 系统 SHALL 保留 worktree 和所有分支；该调用 SHALL 不能绕过进行中的删除 claim 或在会话数据未删除前假报成功 |
| TC-029 | S-08 | Frontend uses adapters | 组件 SHALL 只调用 service，不能直接 invoke、运行 Git 或访问 SQLite；Desktop 与 Web/mock adapter SHALL 实现相同 DTO 契约 |
| TC-030 | W-01 | Register a newly created worktree | 系统 SHALL 持久化创建意图、Git 身份、来源类型和会话绑定；记录创建意图失败时 SHALL 不执行 Git add |
| TC-031 | W-01 | Session persistence fails after Git creation | 系统 SHALL 保留可恢复的创建意图并标记 needs_attention；系统 SHALL 不自动递归删除目录或把未知资源当成可清理资源 |
| TC-032 | W-01 | Verify a legacy session worktree | 系统 SHALL 可以记录 legacy_verified 来源后执行常规安全检查；仅名称匹配 SHALL 不能替代来源证据 |
| TC-033 | W-01 | Legacy or external ownership is unproven | 系统 SHALL 将清理策略限制为 keep；历史迁移 SHALL 不执行任何磁盘清理 |
| TC-034 | W-02 | Reject an ordinary or main workspace | 系统 SHALL 拒绝目录清理并保持原目录内容不变 |
| TC-035 | W-02 | Reject a replaced root | 系统 SHALL 拒绝执行旧授权；系统 SHALL 不删除链接目标或新建的同名目录 |
| TC-036 | W-02 | Resolve Git metadata rather than guessing | 系统 SHALL 使用 Git 查询结果进行身份核验；系统 SHALL 不通过拼接 .git/worktrees/<name> 推测删除对象 |
| TC-037 | W-02 | Do not trust arbitrary client paths | 服务端 SHALL 拒绝非法请求，且不执行文件系统删除 |
| TC-038 | W-03 | Preserve unmerged committed work | 在其他检查通过后系统 SHALL 允许移除工作目录；该分支和原提交 SHALL 保留，不要求自动合并也不删除引用 |
| TC-039 | W-03 | Reject detached or changed identity | 系统 SHALL 拒绝清理且不根据旧 worktreeBranch 假设提交已被保留 |
| TC-040 | W-03 | Reject nested worktree layouts | 系统 SHALL 拒绝快捷清理，包括历史 repo/src-feature-a 布局 |
| TC-041 | W-03 | Reject locked or unsupported layouts | 系统 SHALL 返回结构化阻止原因；系统 SHALL 不执行 unlock、repair、修改 index 标志或使用 force 绕过 |
| TC-042 | W-04 | Reject modified tracked files | 系统 SHALL 阻止清理并保留会话与目录直到用户重新决定 |
| TC-043 | W-04 | Reject untracked files and conflicts | 系统 SHALL 阻止清理，不自动提交、stash、reset 或 clean |
| TC-044 | W-04 | Bounded parsing is incomplete | 系统 SHALL 返回 incomplete 或检查失败；系统 SHALL 不以已解析的空集合判定清理安全 |
| TC-045 | W-04 | Handle unusual filenames | 系统 SHALL 使用 NUL/字节安全解析并维护真实路径身份；有损展示 SHALL 不影响执行目标，无法安全处理的平台 SHALL 拒绝清理 |
| TC-046 | W-05 | Acknowledge the current ignored inventory | 只有匹配本资源及当前摘要的确认 SHALL 允许清理；系统 SHALL 不将确认自动用于另一个资源 |
| TC-047 | W-05 | Do not enumerate private content | 系统 SHALL 只返回必要路径/元数据及完整性；日志和前端 SHALL 不接收这些文件的正文或秘密值 |
| TC-048 | W-05 | Ignore files change during stop | 系统 SHALL 使旧确认失效并返回新的风险预览；系统 SHALL 不因已取得停止回执而跳过文件复查 |
| TC-049 | W-06 | A second session opens the worktree as a folder | 该会话 SHALL 被识别为引用并阻止目标清理 |
| TC-050 | W-06 | Two sessions share only the source project | 系统 SHALL 不仅因原项目路径相同就把它们视为同一 worktree 引用 |
| TC-051 | W-06 | A dormant binding still uses the directory | 系统 SHALL 将其列为引用并阻止清理；系统 SHALL 不自动删除或解绑这些引用 |
| TC-052 | W-06 | References cannot be fully resolved | 系统 SHALL 返回引用检查不完整并禁止清理 |
| TC-053 | W-07 | Remove a validated worktree | 系统 SHALL 只执行不带 -f/--force 的 Git worktree remove；目录及登记确认移除后 SHALL 保留全部已有 Git 引用 |
| TC-054 | W-07 | Git refuses removal | 系统 SHALL 记录并重新观察效果；系统 SHALL 不改用 recursive delete、prune、clean、reset 或 unlock |
| TC-055 | W-07 | Git times out with uncertain effect | 系统 SHALL 进入 needs_attention 并阻止重复清理；系统 SHALL 不假定没有副作用或自动启动第二个 remove |
| TC-056 | W-07 | No surviving execution anchor | 系统 SHALL 拒绝清理而非从待删目录执行删除 |
| TC-057 | W-08 | Keep a managed worktree | 系统 SHALL 保留目录、登记和分支并记录 retained 状态；结果 SHALL 提供已保留路径，不声称磁盘空间已释放 |
| TC-058 | W-08 | Delete session records with associated resource history | 最小 worktree 来源/状态和未完成删除 journal SHALL 不被 session 外键级联删除 |
| TC-059 | W-08 | Preserve Loop and temporary subagent policies | 本能力 SHALL 不执行这些目录的清理；现有 Loop 保留和子 Agent 独立回收机制 SHALL 保持各自边界 |
| TC-060 | O-01 | Produce a deletion preview | 服务 SHALL 返回会话与去重 worktree 分组、风险/完整性、允许策略、有效期和 runtimeEffect |
| TC-061 | O-01 | Reject stale or altered authorization | 执行 SHALL 被拒绝且不得删除目录或会话 |
| TC-062 | O-01 | Keep works without filesystem access | 系统 SHALL 允许在会话授权和停止成功后删除会话；系统 SHALL 不对文件系统发起删除或要求 Git 健康才允许 keep |
| TC-063 | O-02 | Journal creation fails | 系统 SHALL 不停止会话或执行 Git 删除；会话数据 SHALL 保持存在 |
| TC-064 | O-02 | Record before Git removal | 系统 SHALL 先提交 remove_started 和身份快照；提交失败 SHALL 不执行 remove |
| TC-065 | O-02 | Protect recovery records from retention | 未完成删除和未解决资源的最小恢复 journal SHALL 保留 |
| TC-066 | O-03 | Cancellation was accepted but execution still runs | 协调器 SHALL 继续等待真实静止证据或报告停止超时；系统 SHALL 不执行 Git remove 或会话数据删除 |
| TC-067 | O-03 | A managed process cannot stop | 对应组 SHALL 失败并保留会话/目录；已停止的进程 SHALL 不自动重启来假装回滚 |
| TC-068 | O-03 | One seat is still active | 会话 SHALL 不被视为静止；清理 SHALL 等待全部受管理写入者退出 |
| TC-069 | O-04 | A new session targets a removing worktree | 新引用提交 SHALL 被拒绝或等待门禁；系统 SHALL 不产生指向已删目录的新会话 |
| TC-070 | O-04 | A task changes its target during cleanup | 该变更或执行入场 SHALL 经相同资源门禁仲裁 |
| TC-071 | O-04 | Only one cleanup owner is permitted | 最多一个 owner SHALL 能进入 Git remove；租约过期但旧 owner/进程状态不明 SHALL 不允许新的破坏性执行 |
| TC-072 | O-04 | Keep does not freeze other users of a directory | 系统 SHALL 只停止目标会话活动；其他未删除会话 SHALL 不因共享路径而被停止或冻结 |
| TC-073 | O-05 | Tracked file changes after preview | 系统 SHALL 在实际 remove 前拒绝清理并刷新风险信息 |
| TC-074 | O-05 | A reference appears before gate acquisition | 最终核验 SHALL 阻止清理 |
| TC-075 | O-05 | Runtime stopping changes only lifecycle metadata | 系统 SHALL 不仅因 session.updatedAt 变化而否决所有正在运行的会话删除 |
| TC-076 | O-06 | Worktree removed but database commit fails | journal SHALL 保留 worktree_removed/finalize_pending；系统 SHALL 不重复移除、不恢复假目录，也不让目标会话重新执行 |
| TC-077 | O-06 | Deletion transaction commits | 会话数据和原有消息级联 SHALL 一致删除；活动选择 SHALL 只在其 ID 属于已删集合时清空，提交后才发布事件 |
| TC-078 | O-06 | Git fails before confirmed removal | 系统 SHALL 不自动删除会话记录或改为 keep；重新选择 keep SHALL 是新的明确授权且只在未决效果已核实后允许；已证明目标完整且没有在途清理进程时 SHALL 持久化失败结果并释放本次删除 claims；效果不明或目录已移除时 SHALL 保持必要隔离 |
| TC-079 | O-07 | Duplicate identical request | 系统 SHALL 返回同一 operationId；Git 移除和会话数据库删除 SHALL 不重复启动 |
| TC-080 | O-07 | Request ID reused for other targets | 系统 SHALL 返回幂等冲突并保持既有操作不变 |
| TC-081 | O-07 | Retry partial results | 已成功分组 SHALL 不重放；再次可能产生磁盘副作用的组 SHALL 需要新 preview，DB-only finalize SHALL 不执行 Git |
| TC-082 | O-08 | All referencing sessions are selected | 后端 SHALL 形成一个资源组并最多移除一次；该组会话 SHALL 统一进入最终删除事务 |
| TC-083 | O-08 | An unselected reference remains | 资源清理 SHALL 被阻止；用户仍 SHALL 可以显式仅删除选中的会话 |
| TC-084 | O-08 | Independent groups have different outcomes | 系统 SHALL 保留成功组结果与失败组现场；聚合 SHALL 显示 partial 或相应非全成功结果，不能用 Promise.all 拒绝丢掉已完成证据 |
| TC-085 | O-09 | Crash after removal receipt | 恢复 SHALL 验证原目录和登记仍缺失且仓库可访问后仅重试数据库完成；恢复 SHALL 不再运行 Git remove |
| TC-086 | O-09 | Crash between Git effect and receipt | 恢复 SHALL 记录 removed_observed_after_interruption 并完成原授权数据库收尾；记录 SHALL 区分效果观测与丢失的 Git 返回码 |
| TC-087 | O-09 | No removal observed after interruption | 恢复 SHALL 要求新的预览和确认；启动扫描 SHALL 不自动重新执行破坏性命令 |
| TC-088 | O-09 | Ambiguous or offline resource on restart | 系统 SHALL 进入 needs_attention 并保留会话/证据；系统 SHALL 不 prune、repair、递归删除或假报成功 |
| TC-089 | O-09 | Same path recreated by another actor | 恢复 SHALL 不删除新对象并要求人工处理 |
| TC-090 | O-09 | Other lifecycle maintenance runs concurrently | 这些入口 SHALL 尊重删除 claim；目标 SHALL 不被重新置为可执行或清除未完成删除状态 |
| TC-091 | O-10 | Sensitive files and Git diagnostics | 界面 SHALL 只展示必要的结构化原因；持久化诊断 SHALL 经过统一脱敏且不保存文件正文或完整环境 |
| TC-092 | O-10 | Any bounded check exceeds its budget | 系统 SHALL 返回有界失败/incomplete 状态并保留未完成目标；系统 SHALL 不无限等待或把截断结果当完整安全检查 |
| TC-093 | O-11 | Web mock simulates cleanup | 预览、handle 和结果 SHALL 标明 simulated；界面 SHALL 不宣称已删除用户本机目录或释放真实空间 |
| TC-094 | O-11 | Native cleanup integration | 验收 SHALL 检查临时目录不存在、Git 登记不存在且分支仍可解析；mock 返回值或截图 SHALL 不能替代这些原生证据 |
| TC-095 | O-11 | HTTP adapter is absent | 系统 SHALL 显式报告不支持；系统 SHALL 不静默调用 Web/mock 返回清理成功 |
| TC-096 | S-09 | Delete selected sessions | UI SHALL 以一个预览和一个执行请求提交全部选中 id；React 组件 SHALL 不直接 invoke 或访问 SQLite |
| TC-097 | S-09 | Refresh after multi-session deletion | UI SHALL 刷新可见会话、归档会话、活动会话与工作流状态 |
| TC-098 | S-09 | Delete active session in batch | 包含活动会话时 SHALL 清空活动选择；不包含时 SHALL 保持活动会话不变 |
| TC-099 | S-09 | Report batch deletion failure | UI SHALL 显示本地化失败反馈并刷新状态；失败会话 SHALL 保留在批量选择中 |
| TC-100 | O-01 | System activity sessions are refused | 服务 SHALL 拒绝或排除系统活动会话 id、空集合、超限与重复；被拒绝请求 SHALL 不产生 journal、停止或目录删除 |

## 3. 关键 fixture 配方

### F1：真实成功与默认保留

建立有两个提交的主仓库，创建独立 linked worktree 和本地分支，注册普通会话来源。先通过 keep 删除一份会话，断言目录、Git 登记、分支与工作文件完整保留。再在独立 fixture 中通过真实 Rust service 预览、确认、执行 remove-safe，断言目标目录和登记消失，分支仍可解析到原 HEAD，主仓库文件与引用未被错误修改。不用 mock Git 代替。

### F2：各种未保存内容

分别制造 tracked unstaged、staged、冲突、普通 untracked、ignored 文件、ignored 目录中的 .env、符号链接、特殊文件及子模块。非忽略修改必须阻止。ignored-only fixture 没有额外确认时阻止，有正确 fingerprint 且所有安全检查通过后才可清理；改动 ignored 路径/元数据后旧 fingerprint 必须失败。对未支持布局返回具体拒绝而不是 force。

### F3：引用与归属

会话 A 持有来源记录，会话 B 通过 folder 指向同一 worktree 子目录且 worktreePath 为空；A 单删不能清理。把 B 归档仍阻止；把一条禁用定时任务的工作目录指向资源也阻止。另建 worktree C 使用同一原 projectPath，不应误判为同一引用。测试只靠 `vanehub/` 前缀不能授权；历史可信来源三方证据完整时才可打开清理选择。

### F4：受控竞态

使用 barrier 而非 sleep 碰运气。在 preview 后、门禁前插入新的会话引用，复查拒绝。门禁已取得后从另一线程/进程发起新绑定，不能成功入场。分别在 preview 后改变 tracked 内容、ignored 元数据、branch/HEAD、root identity、登记；执行必须检测。只改变停止流程的 lifecycle/消息不应导致所有运行中会话都永久无法删除。

### F5：进程静止

固定 CLI 桩收到 cancel 后继续写一段时间，用可控回执释放。确认 coordinator 在 cancel accepted 时不进入 Git。多 seat 只停一个不足以清理。模拟句柄持有与退出超时，保留会话与目录。停止后不自动重启原进程来伪装事务回滚。Windows 额外以实际文件占用验证拒绝/失败结果。

### F6：数据库和进程崩溃注入

依次在 journal 创建前后、quiesce 后、remove_started 提交后、Git 调用前/中/返回后、receipt 写入前后、会话事务内/commit 后、事件发布前后注入故障。记录 Git 调用计数和原资源身份。

- journal 初次失败：零停止、零删除。
- remove_started 无 receipt 且原目录/登记完整：重启不自动重放 remove。
- Git 成功 receipt 丢失且目录/登记均缺失：恢复观测效果，原授权 DB 收尾，无第二次 remove。
- receipt 存在 DB 失败：只做 DB finalize，不能重新执行 Git。
- 目录/登记只消失一方：needs_attention，无 prune/recursive delete。
- 相同路径被创建成新的对象：不得删除新对象。
- 已证明无不可逆效果且无在途清理进程：失败结果先落盘再释放 claim，新授权能够重新进入；不能永远锁住会话。
- 效果不明或 worktree 已移除：不得通过关闭弹窗或选择 keep 解除隔离。
- 仓库所在卷离线或访问拒绝：不能视为已移除。

### F7：批量与幂等

同资源两个被选会话形成一个组，移除调用计数为一；保留外部引用时阻止。两个独立资源一个成功、一个失败，聚合为 partial，成功结果不丢。重试仅处理剩余组。相同 requestId+相同 hash 返回同 operationId；相同 ID+不同 choices/targets 冲突。网络响应丢失不重复动作。跨实例抢占不只用进程内锁，旧 owner 未确认退出不能再次 remove。

### F8：运行时与 UI

用 service mock 验证所有入口同一对话框、取消无副作用、默认不勾、目标切换重置、键盘/屏幕阅读器、长路径、错误和部分完成。Web/mock 必须显式 simulated；其截图不能当原生文件删除证据。真实桌面在临时测试数据库中完成同样点击并从 Git/磁盘核验结果。web-http 无 adapter 仍显式拒绝。

## 4. 崩溃恢复观察表

| Journal | 原目录 | Git 登记 | 仓库/身份可验证 | 预期动作 |
| --- | --- | --- | --- | --- |
| remove_started | 原身份存在 | 存在 | 是 | awaiting_decision；新 preview 后才能重试 Git |
| remove_started | 确认不存在 | 确认不存在 | 是且旧 owner 已失效 | 记录恢复观测 receipt，只进行 DB finalize |
| worktree_removed | 确认不存在 | 确认不存在 | 是 | 仅 DB finalize |
| 任意未决阶段 | 存在新身份 | 任意 | 是 | needs_attention；不触碰新对象 |
| 任意未决阶段 | 一方存在 | 另一方缺失 | 是 | needs_attention；不 prune 或直接删目录 |
| 任意未决阶段 | 无法访问 | 不确定 | 否 | needs_attention；离线不等于删除成功 |

## 5. 平台验收记录模板

| 平台 | Git 版本 | 单元/真实 Git | 桌面 IPC/E2E | 重启/占用/路径 | 状态与证据 |
| --- | --- | --- | --- | --- | --- |
| Windows | 实施时填写 | NOT RUN | NOT RUN | NOT RUN | 尚未执行 |
| macOS | 实施时填写 | NOT RUN | NOT RUN | NOT RUN | 尚未执行 |
| Linux | 实施时填写 | NOT RUN | NOT RUN | NOT RUN | 尚未执行 |

状态仅使用 PASSED/FAILED/BLOCKED/NOT RUN。当前平台通过不得替代其他平台；用源码静态检查不能勾选 Windows 占用或 junction 的真实验证。
