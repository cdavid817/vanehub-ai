## 1. 基线和合约（先于实现）

- [x] 1.1 核对当前 HEAD、AGENTS.md、六个相关主规范及活动 ACP 变更；记录与本包基线的差异，不覆盖已有变更。
- [x] 1.2 在实现前运行 `npx --yes @fission-ai/openspec@1.8.0 validate enforce-loop-execution-scope --strict`，确认需求和范围已评审。
- [x] 1.3 建立 side-effect inventory：native 工具、ACP read/write/terminal、Claude hook、每种 CLI transport、verification、MCP/Skill 及后代进程；逐项记录能力证据和明确不支持项。
- [x] 1.4 定义版本化 scope、requestedMode、assessment、evidence、reason code 与受信任 ownership ref 的 Rust/TS 合约，包含模拟标记与 additive DTO 映射。

## 2. 路径与权威准入

- [x] 2.1 实现一个可复用的纯路径配置校验器：字面组件匹配、非空允许范围、显式 `.`、保护优先、保留资源、完整配置容量错误；补齐路径矩阵单测。
- [x] 2.2 实现平台资源解析与安全交付 port/adapter，覆盖创建、覆写、rename 两端、递归删除、模式/类型/链接；不得依靠 canonicalize 后字符串 open。
- [x] 2.3 对 symlink/junction/hardlink/TOCTOU 编写真文件 sentinel 用例；不支持平台返回能力不足，不降级为字符串检查。
- [x] 2.4 把保存语法、readiness 和 start 统一到同一原生校验服务；start 复查当前版本、active run、能力和审计模式确认，不信任 UI 结果。
- [x] 2.5 定义完整/部分/仅产物/未知覆盖评估；实现默认严格模式阻断和逐次手动审计模式准入，保留 Verifier 只读硬约束。
- [x] 2.6 实现 admission challenge、逐操作一次 audit receipt、5 分钟 TTL、原子消费和幂等键；接通 start/resume/continue，旧调用不得默许审计模式。

## 3. 持久化与可信范围

- [x] 3.1 用下一可用 migration id 添加范围版本、模式、绑定/证据和必要索引，使用现有事务/CAS 机制；迁移回归覆盖旧 JSON/缺字段。
- [x] 3.2 queued 事务冻结定义、模式及初始能力评估；preparing 后持久化实际 worktree 身份、base commit、baseline 和角色归属，成功前不启动 Agent/命令。
- [x] 3.3 为 `LoopRoleSessionRequest`、`RunLoopVerificationRequest`、session gateway 和权限上下文接通 backend scope ref，保持 stable Agent principal 不变。
- [x] 3.4 对嵌套工具、MCP、Skill 和进程后代继承交集上限；上下文缺失、替换或跨 run 重用时拒绝。
- [x] 3.5 实现旧定义 `legacy-unverified`、旧非终态 `scope-binding-missing` 及历史终态无新证明展示，不能自动扩大空范围或补写历史授权。

## 4. 托管操作与进程

- [x] 4.1 在所有 native 托管文件副作用前接入 scope admission，再进入现有 permission evaluator；覆盖 trusted/yolo/记忆 grant 不得越界。
- [x] 4.2 为 ACP write/read/terminal 及 Claude 已映射 hook 接入可信运行归属和范围准入；映射不完整不能宣称完整 CLI 覆盖。
- [x] 4.3 统一评估验证命令及脚本的所有副作用通道；严格模式拒绝未覆盖的 interpreter、构建脚本、shell/MCP；有能力的执行器约束后代进程。
- [x] 4.4 让 Verifier 所有变更通道执行前拒绝，包含验证阶段后续使用的 CLI/工具；不能证明只读的组合不可选用于实际执行。
- [x] 4.5 对 capability witness 失效和权限收紧实现原生撤销/取消/暂停；未来 CLI launch 参数规则保持不变，不假称动态改变已启动 CLI。
- [x] 4.6 用真实 native 工具完成允许路径写入的正例及越界/保护写入无副作用反例；不得以禁用全部 Loop 代替实现。
- [x] 4.7 增加 versioned verification kind，旧记录映射 process；实现无子进程 native-check/patch-whitespace、安全只读快照和明确检查范围，拒绝未知参数，不能替换用户必需 process 检查。
- [x] 4.8 完整运行严格 native Loop：托管 Worker 实际写文件、必需内置 verification、只读 Verifier、阶段证据、awaiting 和 accept；关闭未覆盖通道，验证越界请求无副作用。

## 5. ACP 读取与批准

- [x] 5.1 在现有单一 evaluator 中提供三态与健康状态，故障 Ask 与正常 Ask 可区分；保留 MCP Ask floor 和 existing grant 规则。
- [x] 5.2 修复 `handle_read`：Allow 安全读取、Deny 无内容且无批准、健康 Ask 延迟；无人可交互时拒绝，批准前不读取内容。
- [x] 5.3 给 InteractionKind 增加 FileRead，接通所有 selector/display/decision_fits/cancelled_reply/tool_call_id/交付/ack 分支；输入范围先验证。
- [x] 5.4 在原有 pending store 和 resolution 事务中绑定 owner、scope、resource identity；复用单赢家、commit-before-effect、过期和重启策略。
- [x] 5.5 交付前重查当前策略健康状态、Deny、取消和路径身份；Once 批准在健康 Ask 下只生效一次，不能重复 Ask 或跨上下文复用。
- [x] 5.6 补齐 ACP Allow/Deny/Ask、存储失败、取消/超时/重启、重复决议、路径替换和跨 run 测试；断言没有提前返回 sentinel 内容。

## 6. 阶段证据与接受门禁

- [x] 6.1 实现 Agent 不可写的 baseline/current manifest 存储及异步完整扫描，覆盖 ignored/untracked/hidden/binary/type/mode/link/delete，不复用 bounded UI diff 为证明。
- [x] 6.2 扫描遇到预算、访问、取消、变化或平台能力缺口时产生 unverifiable；处理 run 专属临时目录，禁止全局缓存豁免。
- [x] 6.3 Worker、验证命令、Verifier 阶段结束回收后代写入者，再记录范围证据；已观察违规不可被后续回滚清除。
- [x] 6.4 扩展 verification 证据复用键为命令内容、输入指纹、scope、policy 和 executor witness；旧 id 命中不得绕过检查。
- [x] 6.5 把完整无违规范围证据接入 native decision 和 awaiting-acceptance；已知违规 Failed，证据不足 Paused，保留工作区和诊断。
- [x] 6.6 将所有 accept 入口接入异步 operation，按 writer lease→回收→实际 artifact 内容复制/完整 manifest→验证指纹一致性→CAS 封存；外部复制竞态或并发请求不能成功接受陈旧证据。
- [x] 6.7 同步 canonical Run 结果，保证 Loop 与通用运行投影不会出现失败/成功矛盾；禁止自动回滚或删除越界产物。
- [x] 6.8 验证严格执行边界确实保护证据存储；审计模式在确认/展示中明确合作 CLI 与可信宿主控制面的前提，异常证据阻断，不能以换目录或 hash 宣称抗同 UID 篡改。

## 7. 恢复与客户端

- [x] 7.1 扩展领域状态不变量支持新增 Paused 原因；resume/continue 在清除 blocker 前重新检查上限、证据与能力，禁止自动降级或不确定副作用重放。
- [x] 7.2 接入现有 shared recovery projection；覆盖有效绑定恢复、旧绑定缺失、证据损坏及审计模式重新人工确认。
- [x] 7.3 同步 loop service、Tauri/Web adapters、命令 DTO/mapper 和 Web 状态/调度模拟；React 组件禁止直接 invoke。
- [x] 7.4 更新表单、review、overview 和 clone/enable 转换，明确路径语法和两种模式并保留全部字段 roundtrip。
- [x] 7.5 preflight 常显真实覆盖程度和限制；run header/detail/timeline 显示冻结评估、证据及恢复动作，不能用绿色 readiness 暗示隔离。
- [x] 7.6 补齐简中/英文、两主题、窄屏、可访问性与 mutation pending；Web mock 在关键位置标识模拟并演示拒绝、Ask、失效证据。
- [x] 7.7 复用 unified logs 和 operations，存储前脱敏；核实没有文件内容、新 feature log 或无界工具 payload 落盘。

## 8. 验收与质量门禁

- [x] 8.1 按 acceptance.md 逐项记录测试、平台和结果；native sentinel 的正反例、严格模式真实阻断、审计模式门禁是必需证据。
- [x] 8.2 运行相关 Loop/permissions/ACP/adapter 单元与集成测试、桌面 Loop E2E 及 Web E2E；依据当前 package scripts 选择执行入口，不能凭空发明命令。
- [x] 8.3 运行 `npm run lint:ci`、`npm run test`、`npm run build`。
- [x] 8.4 运行 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`。
- [x] 8.5 运行 `npm run native:panic:check`、`cargo test --workspace`；涉及的 architecture/contracts/i18n 检查按当前 AGENTS.md 和脚本执行。
- [x] 8.6 运行 `npx --yes @fission-ai/openspec@1.8.0 validate enforce-loop-execution-scope --strict` 与 `npx --yes @fission-ai/openspec@1.8.0 validate --specs --strict`；审查 delta 合并预览不丢失既有 scenarios。
- [x] 8.7 复核 Windows/macOS/Linux 的真实能力矩阵；未执行/不支持的平台明确标注并在运行时阻断相应严格能力，不能记为 PASS。
- [x] 8.8 汇总实现、实际命令结果、未通过门禁、兼容性变化和剩余平台限制；只有完成并验证的任务才能勾选。不自动归档、merge、push 或发布。
