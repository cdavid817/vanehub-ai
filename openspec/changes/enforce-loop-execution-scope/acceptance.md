# 验收矩阵

本文件是实施验收要求，所有用例当前均为 **NOT RUN（未实施）**。OpenSpec 语法通过不能替代运行时验证。每项实施记录至少包含：测试文件/用例名、OS/文件系统、执行器和版本证据、实际命令、结果及必要的脱敏证据引用。规范简称：S=loop-execution-scope，R=loop-engineering-runtime，P=permissions-core，A=permissions-approval，U=loop-management-ui，C=cli-agent-permission-launch-flags。

## 路径、能力和权限

| ID | 输入/触发 | 必须观察到的结果 | 规范 / 任务 |
|---|---|---|---|
| SC-01 | allowed=`src`，写 `src/app.ts`；同样写 `src-old/a` | 前者在策略 Allow 后真实写入，后者零副作用拒绝 | S Canonical workspace mutation scope / 2.1,4.6 |
| SC-02 | allowed=`.`；protected=`src/generated`；trusted/yolo/Global Allow/Once approve | 对保护项仍拒绝；不能通过批准扩权 | S Canonical workspace mutation scope，P Loop scope admission precedes permission resolution / 4.1,5.5 |
| SC-03 | 空允许范围、`../`、absolute、drive/UNC/device/ADS、控制符/glob；规范化重复项 | 非法项拒绝，合法规范化一致；空范围不转全项目；非字符串前缀比较 | S Canonical workspace mutation scope / 2.1 |
| SC-04 | 超过配置容量；提示词展示摘要超过 32 项 | 保存明确报容量错误或完整保留合法配置；执行范围从不随 prompt 截断 | S Canonical workspace mutation scope / 2.1 |
| SC-05 | rename 允许→保护/范围外，或范围外→允许；递归删除父目录含保护项 | 任一受影响端/后代越界就拒绝，目标 sentinel 不变 | S Resource checks cover every affected target / 2.2 |
| SC-06 | symlink/junction 指向根外或保护路径；路径校验后替换祖先 | 拒绝或由安全句柄约束，不能改写根外 sentinel | S Race-resistant resource execution / 2.3 |
| SC-07 | hardlink 指向范围外共享 inode；大小写不同卷；Unicode/反斜线输入 | 不原地改范围外别名；按卷/版本语法判断，不能全局 lower-case | S Race-resistant resource execution / 2.3 |
| SC-08 | allowed=`.` 后操作 `.git`、Git common-dir、证据目录 | 拒绝；宿主 worktree 准备仍可用独立窄化能力完成 | S Canonical workspace mutation scope / 2.2,3.2 |
| SC-09 | 同一权限配置，readiness 成功后改变 Agent/definition/active run/能力 | start 使用当前值拒绝且不创建 queued/worktree；准备后身份竞态则记录失败/暂停且无 Agent 副作用 | R Manual bounded Loop start / 2.4,3.2 |
| SC-10 | 仅设置 cwd、ACP、hook 或 CLI sandbox 参数，无覆盖证明 | 严格 start 拒绝，不显示完整强制约束 | S Execution capability evidence，C Loop launch posture does not imply confinement / 2.5 |
| SC-11 | `python -c`/`node -e`、npm/cargo 脚本、后台子进程写根外 | 严格模式启动/调用前拒绝或真实隔离；不能只检查程序名 | S Closed side-effect surfaces，R Guarded deterministic verification / 4.3 |
| SC-12 | Verifier 尝试 write、shell/MCP mutation、改 run state | 两模式均在副作用前拒绝；不可信只读组合无法启动 | R Independent Worker and Verifier roles / 4.4 |
| SC-13 | native 工具→Skill→MCP/子调用丢失 scope；另一个 run 的 token | 拒绝；MCP evaluator 仍保持 Ask floor，不能跨上下文授权 | P Loop scope admission precedes permission resolution / 3.4,5.1 |
| SC-14 | 已运行 scope 中策略收紧/撤权，或能力版本改变 | 托管请求重查；无法动态约束的 owned 进程取消并暂停；不改写已启动参数 | S Execution capability evidence，C Loop capability invalidation is actionable / 4.5 |
| SC-15 | 定义选择 audit 但未确认、确认过期、调度启动或自动降级 | 拒绝；有效逐次人工确认才进入明确标注的审计模式 | S Explicit requested enforcement mode，U Visible enforcement assessment / 2.5,7.5 |
| SC-16 | native 托管 Worker + 必需 native-check/patch-whitespace + 只读 Verifier + accept | 真实 start→preparing→Worker→verification→Verifier→awaiting→accept 全程通过；未覆盖通道关闭，范围内落盘、范围外无副作用 | R In-process deterministic verification supports a complete strict Loop / 4.7,4.8 |
| SC-17 | audit receipt 跨 start/resume/continue、跨 run、超时/重启/时钟回退、scope变化、重复提交 | 绑定不匹配拒绝；成功事务一次消费；同幂等键只返回已有 operation；readiness不依赖已有receipt | S Audit acknowledgements are operation bound / 2.6 |
| SC-18 | builtin未知名称/args、文本新增行尾随空白、扫描失败、旧process记录 | 未知参数拒绝；空白失败；不完整为unverifiable；旧process不升级受控、用户必需检查不被替换 | R In-process deterministic verification supports a complete strict Loop / 4.7 |

## ACP 读取与批准

测试使用临时文件内的随机 sentinel。除 Allow 或匹配有效批准之外，响应、事件、日志和缓存都不得出现该 sentinel；不仅断言 Effect 枚举。

| ID | 输入/触发 | 必须观察到的结果 | 规范 / 任务 |
|---|---|---|---|
| AC-01 | 正常会话及 Loop 的 file.read Allow | 安全解析后按 line/limit 返回一次正确内容 | P ACP file reads honor all permission effects / 5.2 |
| AC-02 | file.read Deny | 不读取内容、不返回内容、不创建 pending | P ACP file reads honor all permission effects / 5.2 |
| AC-03 | 正常 Ask + 交互通道；批准 Once | 批准前无内容；持久提交后健康复核，返回一次；不无限再 Ask | A Deferred ACP file reads reuse durable approval delivery / 5.3,5.5 |
| AC-04 | Ask 无交互通道、超时、取消、generation 结束、重启 | 拒绝/取消，零内容；旧批准不复活 | A Deferred ACP file reads reuse durable approval delivery / 5.6 |
| AC-05 | principal/grant/policy/audit DB 失败或批准提交失败 | 无内容；故障 Ask 不作为 Allow；重试需完整健康评估 | P Evaluation failure fails closed / 5.1,5.6 |
| AC-06 | 等待批准时目标/祖先替换、run/scope/generation 不匹配、策略改为 Deny | 旧批准拒绝，不能读取新目标；需新请求 | A Loop approvals bind immutable scope and live ownership / 5.4,5.5 |
| AC-07 | 两次并发 resolve、重复 delivery、迟到 Allow | 原有 single-winner 语义保持，最多一次内容交付/副作用，不能激活错误 grant | A Deferred ACP file reads reuse durable approval delivery / 5.4,5.6 |
| AC-08 | 非法 line/limit、超大 payload | 排队/读取前明确拒绝；UI/日志只含有界脱敏资源描述 | P ACP file reads honor all permission effects / 5.2,7.7 |

## 产物、生命周期和客户端

| ID | 输入/触发 | 必须观察到的结果 | 规范 / 任务 |
|---|---|---|---|
| EV-01 | 审计模式未托管 Worker/verification 通道创建 ignored、untracked、hidden 保护文件；Verifier阶段用明确测试故障注入/外部篡改产生变化 | 完整扫描捕获包括二进制/权限/type/link变化；已知违规Failed；正常Verifier写请求仍须按SC-12前置拒绝 | S Complete workspace artifact evidence / 6.1,6.3 |
| EV-02 | 先记录越界后恢复原文件；或越界只发生于审计模式不可见通道后恢复 | 已记录违规保持；未观察瞬态不得声称从未发生，展示审计边界 | S Sticky violations and bounded claims / 6.3,7.5 |
| EV-03 | 无法读目录、扫描超限/取消、文件持续变化、writer 未回收 | Paused/unverifiable，不能把部分扫描当通过 | S Complete workspace artifact evidence / 6.2,6.3 |
| EV-04 | commandid 相同但命令、输入、scope、policy 或 executor fingerprint 改变 | 不复用旧 verification 为当前有效证据 | R Guarded deterministic verification / 6.4 |
| EV-05 | checks pass + Verifier pass，但 scope violation/unknown | 不进入 awaiting-acceptance；已有等待态不能成功 accept | R Native decision and stop policy，Human acceptance gate / 6.5,6.6 |
| EV-06 | awaiting 后改树；并发 accept/continue；后台写入；重复 accept | 原生证据门禁/CAS 拒绝陈旧提交或收敛为同一结果，成功绑定封存快照 | S Sealed acceptance evidence / 6.6 |
| EV-07 | 原生直接调用旧 accept/resume 路径，不经 UI | 不能绕过 scope/证据门禁，不提前清 blocker | R Human acceptance gate，Pause, cancellation, and restart recovery / 6.6,7.1 |
| EV-08 | 升级旧定义和各生命周期旧 run；绑定缺失、版本未知、损坏 | 旧定义可编辑但不静默运行；非终态暂停；终态历史不改成功证明 | R Versioned scope migration preserves historical truth / 3.5,7.2 |
| EV-09 | 重启后 owned child outcome 不确定或 capability drift | 消费 shared recovery；不重放副作用，不自动降级；audit 恢复重新确认 | R Pause, cancellation, and restart recovery / 7.2 |
| EV-10 | 严格模式尝试修改宿主证据；audit模式展示信任前提并注入证据损坏 | 严格请求被边界阻断；损坏证据unverifiable；审计模式不宣称抵抗同UID主动篡改 | S Evidence integrity follows the execution trust boundary / 6.8 |
| UI-01 | readiness passed 且 audit；严格模式缺能力；查看历史 run | 始终显示 coverage/限制/冻结模式，不用 passed 暗示隔离；可执行修复原因 | U Visible enforcement assessment / 7.5 |
| UI-02 | create/edit/duplicate/enable-disable/save-load roundtrip | 模式/版本/完整范围无丢失，复制仍遵守原有 disabled 副本约束 | U Scope configuration survives management actions / 7.4 |
| UI-03 | Tauri/Web 两 adapter 走同一操作与错误；missed event pull | 契约一致；mock 明确模拟且不作真实平台能力证明；pending 能恢复 | R Loop frontend service parity，U Visible enforcement assessment / 7.3,7.6 |
| UI-04 | 简中/英文、两主题、窄屏、键盘；accept 慢扫描 | 无硬编码漏译/裁切；显示 mutation pending，重复提交被限制 | U Scope failures remain actionable / 7.6 |
| OB-01 | 所有拒绝/故障/批准交付与 canonical Run 投影 | 结果一致；有界脱敏审计，零文件内容、无新 feature log | S Scope diagnostics preserve privacy / 6.7,7.7 |

## 平台声明与最小交付

| 环境 | 必须验证的内容 | 未执行时的处理 |
|---|---|---|
| Linux + 实际文件系统 | native 安全正反例、symlink/hardlink、子进程和完整扫描 | NOT RUN，不能证明 native enforcement |
| Windows + 实际文件系统 | drive/UNC/device/ADS、junction/reparse、大小写/句柄语义 | 不支持能力必须在 runtime assessment 中阻断严格组合 |
| macOS + 实际文件系统 | 大小写卷差异、链接/安全打开、扫描元数据 | 同上，不用 Linux 测试结果代替 |
| Web/mock | 服务状态、模式确认、错误/批准/恢复 UI | 始终 simulated，不替代 native 验证 |
| 真实 CLI / 测试执行器 | 版本绑定与未覆盖通道阻断、audit 产物门禁 | 测试执行器只能证明接入流程；真实 provider 能力需其自身证明 |

至少完成 SC-16 的一个真实 native 严格 Loop 全生命周期正路径及越界零副作用反路径，以及受控测试 CLI 的审计模式门禁；模型输出可用固定 provider fixture，worktree/工具/持久化/接受门禁必须真实运行。全部入口都有能力评估且未证明者 fail closed。不得通过全部禁用、只测单个工具、全部 mock、全部 prompt 提示或单纯 Git diff 来宣布本项 P0 已完成。若某平台仍未实现，交付说明必须列为明确未完成能力，并维持运行时阻断。
