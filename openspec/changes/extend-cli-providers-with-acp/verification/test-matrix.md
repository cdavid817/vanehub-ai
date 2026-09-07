# 测试矩阵（待实现）

这些是应实现的测试，不是本包已包含的运行时代码。优先复用仓库测试框架和临时 CLI fixture 路径；新增测试文件位置由当前结构确定。所有 fake CLI/ACP 均是合成协议桩，不要把它们标成上游真实输出。

| Case ID | 目标 | 构造/动作 | 必须观察的断言 |
| --- | --- | --- | --- |
| DISC-01 | PATH 与身份 | 放置非 Cursor 的 agent，另有真实身份 fixture | 不可选错程序；保留冲突与选择原因 |
| DISC-02 | Kimi 多发行形态 | 多个 kimi shim 指向当前/旧版 | 安装身份不同，不迁移/覆盖配置 |
| DISC-03 | Windows launcher | 同目录无扩展/.cmd/.ps1 别名、大小写、空格 | 同一逻辑安装去重；safe argv；不拼接用户 shell |
| DISC-04 | Readiness 无副作用 | 打开设置页，记录所有 spawn/network/file mutation | 无 ACP 会话、登录/浏览器、安装或模型调用 |
| DISC-05 | 来源与版本 | fake npm/vendor 精确目标、失效 plan、内部 fallback | 目标一致，过期失败，不透明切源不自动管理 |
| DISC-06 | 环境指纹变化 | preflight 后替换选中 binary | 重新验证或拒绝，不能用旧身份运行 |
| RPC-01 | UTF-8 任意分片 | 逐字节及固定随机分片传入中文 NDJSON | 归一化序列等价，字节不丢失 |
| RPC-02 | 一读多帧 | 文本/工具/final 同一个 buffer | 先投影更新再终态，下一 turn 不串输出 |
| RPC-03 | stderr 与 stdout | stderr 有中文诊断，stdout 有 banner 或畸形 JSON | 诊断不污染聊天；协议污染明确失败 |
| RPC-04 | 双向死锁 | prompt 未响应时 Agent 反向 request_permission | reader 继续处理，用户答复能解除等待 |
| RPC-05 | 协议/能力 | 未知版本、缺 loadSession、未实现 terminal | 握手失败或能力关闭，不能乐观兼容 |
| RPC-06 | 有界资源 | 超过单帧/队列/JSON 深度/pending 上限 | 内存有界、控制消息不静默丢弃、typed error |
| RPC-07 | 未知方法 | 未知带 ID 请求和不带 ID 通知 | 请求正确 error；通知不回复，不无限等待 |
| TURN-01 | 长驻连接 | 两次 prompt，各收到 end_turn 而进程持续 | 两轮独立完成，不等待 EOF，不重启无关进程 |
| TURN-02 | 并发同会话 | 活动 turn 中再次提交 prompt | 按明确策略排队/拒绝，不交叉输出 |
| TURN-03 | 多种 stopReason | end_turn/refusal/max_tokens/max_turn_requests | 保留原因，不全标业务任务验证通过 |
| CANCEL-01 | 协作取消 | session/cancel 后更新及 cancelled prompt response | 一次终态，结束前更新正确处理 |
| CANCEL-02 | 取消升级 | Agent 忽略 cancel，并拥有代理终端 | 宽限期后仅回收拥有的进程/终端并 wait/reap |
| CANCEL-03 | 审批中取消 | 待审批时取消，然后收到旧 UI 同意 | 回复 cancelled；旧答复不生效 |
| AUTH-01 | 无 auth probe | version 成功但无受支持 auth status | unknown 不等于登录；显式检查/登录可见 |
| AUTH-02 | 秘密与环境 | fake 输出 token；另一个供应商 env 放哨兵值 | 日志/UI 摘要脱敏；子进程没有无关秘密 |
| AUTH-03 | 登录链接 | 返回非法 scheme/未受信 host 或命令片段 | 不自动打开/执行，给明确验证结果 |
| PERM-01 | 批准 scope | Agent 给 once/always optionIds，用户选 once | 返回同一原 optionId，未扩大授权 |
| PERM-02 | 拒绝有证据 | 工具请求写临时标记文件，用户拒绝 | 标记不存在且工具未执行，不只检查 UI toast |
| PERM-03 | 归属与单次消费 | 跨 session/epoch 响应、重复点击、policy revision 变更 | 后端拒绝或重新评估，最多一次有效响应 |
| PERM-04 | 等待与刷新 | 保持 live 待审批刷新 UI，另测试 deadline | 刷新恢复正确请求；到期拒绝/取消不批准 |
| PERM-05 | 不可兑现策略 | Kimi print 或其他模式不支持当前硬约束 | spawn/prompt 前拒绝，无自动 bypass/headless fallback |
| FS-01 | 越界路径 | ../、symlink/junction、父目录换链、设备/UNC | 未授权路径没有读写，执行器实际返回拒绝 |
| TERM-01 | 终端归属 | session B 用 session A 的 terminalId | 无输出泄漏，kill/release 也拒绝 |
| BIND-01 | 双 seat | 同 Provider 两进程，不同 cwd/profile/session | prompt、审批、工具、usage 不串线 |
| BIND-02 | 兼容恢复 | loadSession 开/关、错误 ID、不同账号环境/分发形态 | 精确恢复或明确失败，保留本地历史 |
| BIND-03 | 回放 | load 时重放文本/工具/usage，与 live 重复文本混合 | 不重复存储/审批/记账；合法 live 重复保留 |
| BIND-04 | 迁移与回滚 | 旧五种会话 DB 加载后写新字段并禁用新 Provider | 旧路由/记录可读，不删除用户数据 |
| BIND-05 | 副作用与 EOF | 工具已写文件后进程断线 | interrupted/unknown-effect，不重新发送 prompt |
| VEND-01 | Cursor 阻塞扩展 | ask_question、create_plan，拒绝/取消/超时 | UI 可操作且响应 schema 正确，不一直转圈 |
| VEND-02 | CodeBuddy 代理与子任务 | 不同账号环境和内部 member 事件 | 环境显式，代理过权限；不新增 seat/重复账单 |
| VEND-03 | Copilot 进程级配置 | 不同 tools/reasoning 策略的两个任务 | 启动参数和进程隔离，不共享错误配置 |
| VEND-04 | Qoder 平台 | 不支持的平台/架构 fixture | 清楚不兼容，不默默改为 WSL/远程执行 |
| VEND-05 | iFlow legacy | 无本地安装/未 opt-in/已 opt-in/自动化请求 | 说明准确，local-only，后端拒绝自动化 |
| UI-01 | 能力 UI | installed+needs-auth、unknown、unsupported、stale | 正交展示原因；卡片不自行决定原生能力 |
| UI-02 | Web/mock / web-http | mock 环境及 HTTP 未实现场景 | 演示明确，无假宿主；HTTP 失败不回退 |
| UI-03 | i18n/a11y | 中文英文、深浅主题、键盘审批、长路径 | 键值完整、焦点可用、布局无截断关键信息 |
| AUTO-01 | 无人值守 | 审批/问题无响应者、超期限窄预授权 | 干预/取消可观察，不 blanket approval |
| AUTO-02 | 启动时漂移 | 保存任务后换 CLI 版本/策略/安装路径 | 每次执行重检，不依赖 UI 旧快照 |
| REG-01 | 旧五种回归 | 原有 golden argv、权限、PTY、session/usage fixtures | 不为新增 Provider 改坏原有行为 |
| CLEAN-01 | 副作用守卫 | 全部 fake/E2E 运行前后对照 PATH/目录/进程 | 无真实模型、账号、用户 DB、全局安装或孤儿进程 |

## Fixture 设计约束

子进程模式支持：normal、chunked、permission-before-final、malformed、oversized、ignore-cancel、abrupt-eof、replay、unsupported-version、cursor-question、cursor-plan。每个模式可以采用临时环境变量选择，但不得作为生产参数暴露。

假工具把“执行”写入临时审计哨兵，拒绝测试断言哨兵不存在；协议桩还需按实际 schema 发报文，不能只 mock 前端最终状态。原始实际 CLI 录制若获授权，先脱敏并记录版本、平台、模式和 recorded 来源。

网络默认禁用，HOME/用户数据目录/PATH/凭据目录指向隔离临时环境。保持操作系统必需变量，但移除无关 Provider secrets。部署测试遵循原项目 desktop side-effect guard 与真实原生持久化要求。
