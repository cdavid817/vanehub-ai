## 1. 前置与登记

- [ ] 1.1 查询 `schema_migrations` 表与其他活跃分支已占用的迁移版本号，选定本变更使用的号并记录在实施说明中
- [ ] 1.2 在 `openspec/project.md` 的「### Bounded contexts」表中新增 `goals` 一行，说明其所有权范围
- [ ] 1.3 建立 `src-tauri/src/contexts/goals/` 的骨架（`mod.rs`、`api.rs`、`domain/`、`application/`、`infrastructure/`），并在上级 `contexts/mod.rs` 注册
- [ ] 1.4 运行 `npm run docs:check` 确认目录与 Bounded contexts 表已一一对应

## 2. 领域层

- [ ] 2.1 在 `domain/goal.rs` 定义 Goal 聚合与四个持久化状态（草稿、进行中、已达成、已放弃），标题为必填并做去空白校验
- [ ] 2.2 实现状态迁移规则：草稿→进行中、待验收前提下进行中→已达成、已达成→进行中、任意→已放弃、已放弃→进行中
- [ ] 2.3 定义 `GoalDomainError`，照 `PlanDomainError` 的既有写法给出非法迁移与校验失败的变体
- [ ] 2.4 在 `domain/link.rs` 定义 `GoalLink` 与 `GoalLinkTarget` 枚举（Plan、Loop、WorkItem、Session）
- [ ] 2.5 为状态迁移写表驱动测试，覆盖全部合法迁移与全部非法迁移

## 3. 应用层

- [ ] 3.1 在 `application/ports.rs` 定义 `GoalRepository` trait
- [ ] 3.2 在 `application/ports.rs` 定义 `LinkProgressProbe` trait，返回终态、活跃、不可解析三态
- [ ] 3.3 在 `application/progress.rs` 实现纯函数聚合：可推导子项全部终态且数量大于零时派生待验收，否则返回持久化状态
- [ ] 3.4 在聚合中排除 Session 链接，并把不可解析子项剔出分母
- [ ] 3.5 在 `application/goal_service.rs` 实现目标 CRUD、关联与解除关联、验收与重开；关联唯一性冲突返回可读错误
- [ ] 3.6 用假 probe 为 `progress.rs` 写测试，覆盖：无子项、部分终态、全终态、probe 失败、仅 Session 链接、循环停在待验收、计划停在失败、计划停在待验收

## 4. 基础设施层

- [ ] 4.1 编写数据库迁移，建 `goals` 与 `goal_links` 两表，`goal_links` 对 `(goal_id, target_kind, target_id)` 建唯一索引
- [ ] 4.2 在 `infrastructure/goal_repository.rs` 实现 `GoalRepository`
- [ ] 4.3 在 `infrastructure/progress_probes.rs` 实现计划 probe：按最新一次运行判定，已完成与已取消为终态，失败与待验收为活跃，无运行记录为活跃，已归档为终态
- [ ] 4.4 实现循环 probe：复用既有的循环终态判定（已成功、已失败、已取消），待验收为活跃
- [ ] 4.5 实现看板项 probe：完成阶段或已归档为终态
- [ ] 4.6 实现会话 probe：仅用于展示，恒不参与推导
- [ ] 4.7 让每个 probe 的查询失败降级为不可解析而非向上抛错
- [ ] 4.8 在 `api.rs` 暴露 facade，并在 bootstrap 组合边界注入四个 probe 实现

## 5. Tauri commands

- [ ] 5.1 在 `src-tauri/src/commands/goals/` 逐文件实现 `create_goal`、`update_goal`、`delete_goal`、`list_goals`、`get_goal`
- [ ] 5.2 逐文件实现 `link_goal_target`、`unlink_goal_target`
- [ ] 5.3 逐文件实现 `accept_goal`、`reopen_goal`、`abandon_goal`
- [ ] 5.4 确认所有 command 的错误一律转换为 `Result<T, String>`，并注册到 command 清单
- [ ] 5.5 操作日志通过统一日志服务写入，不新建 feature-local 日志文件

## 6. 前端服务边界

- [ ] 6.1 在 `src/contracts/goal.ts` 定义目标、关联、派生进度的类型契约
- [ ] 6.2 在 `src/services/goal-service.ts` 定义服务接口
- [ ] 6.3 实现 `src/services/tauri-goal-client.ts`
- [ ] 6.4 实现 `src/services/web-goal-client.ts`，含 mock 数据与同等的推导语义
- [ ] 6.5 编写两个实现的行为对齐契约测试，参照既有的 `*-adapter-parity.test.ts`

## 7. 前端界面

- [ ] 7.1 在 `src/goal-center/` 实现目标列表与创建、编辑表单
- [ ] 7.2 实现目标详情：子项分组展示、派生状态呈现、进度概览
- [ ] 7.3 明确呈现不可解析子项与阻塞原因，不使用无说明的静态进度条
- [ ] 7.4 实现关联与解除关联的交互，含会话的显式挂载入口
- [ ] 7.5 实现验收与重开操作，非待验收时禁用验收入口
- [ ] 7.6 接入主布局导航，补齐多语言文案
- [ ] 7.7 编写组件测试，确认每个新文件不超过 300 行

## 8. 校验

- [ ] 8.1 运行 `openspec validate add-goal-system --strict`
- [ ] 8.2 运行 `npm run lint:ci`、`npm run test`、`npm run build`
- [ ] 8.3 运行 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` 与 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 8.4 运行 `cargo test --manifest-path src-tauri/Cargo.toml` 与 `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] 8.5 运行 `npm run docs:check` 与 `openspec validate --specs --strict`
- [ ] 8.6 界面行为有变更，运行 `npx playwright test`
