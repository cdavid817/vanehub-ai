# Claude Code 实施指令

你现在在 cdavid817/vanehub-ai 仓库中。请实施已确认的 OpenSpec change：

`openspec/changes/add-session-worktree-cleanup/`

目标：删除会话时弹出统一确认框，默认仅删除会话；对于经过核验的普通会话 Git worktree，允许用户显式选择同时安全移除工作目录。第一版始终保留分支，不提供 force 删除。

我已确认此方案，请进入实现，不要只继续输出设计或泛化建议。读取 proposal.md、design.md、tasks.md、specs/、acceptance-tests.md、code-map.md 与 verification.md 后，按任务依赖逐步实施、测试、自审和修复。遇到实现细节与源码不符，先依据当前源码更新 code-map/设计和相关 delta，再继续；不得通过放宽保护条件解决冲突。

## 开始前

读取根 AGENTS.md、CLAUDE.md 指向的约束、openspec/config.yaml、openspec/project.md。记录当前 git status、分支与 HEAD，检查未提交修改和重叠 change。设计基线是 main@d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5，但应在我当前指定/打开的目标 checkout 实施；不要擅自切换 main/dev、重置、覆盖已有修改，也不要根据旧会话记忆假定技术栈或迁移号。

执行 `openspec validate add-session-worktree-cleanup --strict`。对 MODIFIED 需求读取当前主规范的完整原文，保留已有场景，再验证。环境缺少工具时先记录实际阻塞；不要声称已通过，也不要跳过仓库门禁来制造完成状态。

## 不可省略的实现边界

1. 所有单条、右键、搜索、归档列表和批量删除共用确认服务。默认 keep，破坏性选择不记忆；普通目录和远程目录绝不删除。
2. worktree 路径和允许策略由后端可信数据解析；校验 provenance、root/common-dir/admin-dir、Git 登记和实际文件系统身份。不能仅凭 `vanehub/` 或目录名授权，也不能接受前端任意路径。
3. 新 worktree 记录来源，历史资源只有可信创建证据与当前 Git 身份完整对应才允许清理；未知/外部/Loop/子 Agent 的资源保持原策略，不在本功能中强制接管。
4. tracked/staged/conflict/untracked 有变化时阻止清理；ignored 文件单独提示、完整扫描、绑定当前 fingerprint 额外确认。扫描失败/超限/不完整必须 fail-closed。
5. 查所有真实引用，包括 folder/子目录形式引用、归档会话、Loop review、定时任务和运行句柄。共享原 projectPath 不代表同一 worktree；同一真实 worktree 只清理一次。
6. 删除需要持久化 journal、幂等 requestId、session claim 和跨实例工作区使用门禁。先停止并等待全部受管理写入者退出，再最终复查；取消受理不等于退出。
7. 唯一允许的清理是从可信存活锚点执行非 force `git worktree remove`。禁止 recursive delete 兜底，禁止自动 prune/unlock/repair/clean/reset、自动提交/stash/合并，以及删除任何分支。
8. Git 清理失败不能自动降级成仅删除会话；Git 已移除但数据库失败必须保存 finalize_pending，能重启恢复。部分效果和同名新对象不得盲目重放删除。
9. 保持 React → service → Tauri/Web adapter → Rust application/ports 分层；Web/mock 明确 simulated，web-http 无 adapter 仍显式失败。旧内部 deleteSession 仅 keep 且不绕过 claim。
10. 修正批量发起后立即关闭/丢结果和非活动会话删除错误清空活动状态的问题；用逐资源/会话结果展示进度、失败、部分完成和重试。

不要顺手建设全局 worktree 管理中心、自动 GC、删分支功能，不要扩展到远程目录清理。创建流程本次只接入必要来源与门禁，其他创建体验问题单独处理。

## 测试与交付

按 acceptance-tests.md 和 traceability.json 落实自动化，先纯策略与真实临时 Git/DB，再 adapter/组件/Playwright，最后真实桌面 IPC 与当前平台端到端。故障注入必须覆盖 Git 效果与数据库之间的崩溃窗口、receipt 写失败、目录替换、旧 preview、重复请求、共享引用和停止超时。

所有 destructive 测试只能在本次创建的隔离临时仓库/临时数据库运行。不要拿我的实际项目、已有 worktree、生产会话、真实凭据或真实模型做删除测试。

先跑针对性测试，再逐字执行当前根 AGENTS.md 的全部适用门禁，包括本 change 与主 specs 两种严格校验。任何失败要保留证据并修复；环境缺失标 BLOCKED，未跑标 NOT RUN。不要把 mock 或 Linux 通过写成三平台通过。新增生产 TS/TSX 不超过 300 行；不得增加 eslint 豁免、跳过 hooks 或降低安全检查。

完成一组就更新 tasks.md，仅给确有实现与验证证据的任务打勾。所有校验完成后更新 verification.md，输出修改文件、需求到测试映射、真实命令结果、当前平台、剩余风险。可以按小步组织本地提交，commit message 用英文 Conventional Commits；没有额外授权不要 push、合并或 archive。
