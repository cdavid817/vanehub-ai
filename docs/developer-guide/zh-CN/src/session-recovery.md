# 会话恢复

受管理的生成是持久的：会话可以在崩溃、中断的 tool-use loop 或结构性不一致之后安全恢复，而不会丢失证据或重复执行工作。恢复状态**独立**于会话的生命周期状态进行追踪。

## 恢复状态与生命周期正交

每一个持久会话都带有一个恢复状态：`clean`、`reconciling`、`action_required` 或 `quarantined`：

- 一个 `failed` 会话，如果恢复状态为 `clean` 且没有活跃的 run，仍然接受新消息。
- 一个 `idle` 会话，如果恢复状态为 `action_required`，则在每一条受管理的提交路径上拒绝新的生成工作，直到某个允许的恢复动作成功为止。
- 一个 `quarantined` 会话——一种稳定的、不冒丢失证据风险就无法对账的结构性不一致——保持可读且可导出，但拒绝任何依赖该不一致状态的生成或变更。

## 持久执行标识与归属

每一个被接受的受管理生成都有一个稳定的 execution run id，与其会话和已持久化的消息相关联。在 provider 或 CLI 执行开始之前，该会话会原子地认领**至多一个**活跃的 execution run。在一个 run 活跃期间发起的竞争认领会被拒绝，且不会启动工作。

## 恢复流程

启动恢复由 `run_startup_with_retry` 驱动,它扫描候选会话、对每个候选做一次原子认领、读取终态证据、决策恢复状态,最后发布恢复结果并写一份不可变报告。整条路径与生成路径解耦:它只清 `active_run`、打标记、写报告,**绝不自动重放**任何工作。

```mermaid
sequenceDiagram
    participant Boot as run_startup_with_retry
    participant Scan as 候选扫描
    participant Claim as claim_recovery_candidate
    participant Evidence as read_terminal_evidence
    participant Decide as decide_recovery
    participant Pub as publish_recovery
    participant Report as 不可变报告

    Boot->>Scan: 扫描需要恢复的会话
    Scan->>Claim: 逐个候选
    Claim->>Claim: 原子 CAS 认领<br/>至多一个活跃 run
    Claim->>Evidence: 读取终态证据
    Evidence->>Decide: 输入证据
    Decide->>Pub: 决策恢复状态
    Pub->>Report: 写不可变报告
```

恢复状态本身是一个独立列 `recovery_status`,下面的状态机枚举了所有可达状态:

```mermaid
stateDiagram-v2
    [*] --> clean : 新建会话
    clean --> reconciling : 启动恢复扫描
    reconciling --> clean : 决策为一致
    reconciling --> action_required : 决策需用户介入
    reconciling --> quarantined : 结构性不一致
    action_required --> clean : acknowledge_recovery 成功
    action_required --> quarantined : 用户选择隔离
    quarantined --> [*] : 仅可读/导出
```

`decide_recovery` 的关键决策如下。注意单一终态会落到 `clean` 并带上对应的终态标签,而无终态、无工具活动的 ManagedApi 情况会保留部分内容并标记为 `InterruptedWithoutToolAmbiguity`。

```mermaid
flowchart TD
    E[read_terminal_evidence 输出] --> D{decide_recovery}
    D -->|消息序列非法| Q1[Quarantined]
    D -->|无 active run| AR1[action_required]
    D -->|run 不一致| AR2[action_required]
    D -->|未完成工具活动| AR3[action_required]
    D -->|单一终态: completed/failed/cancelled| C1[clean + 终态标签]
    D -->|无终态,无工具活动,ManagedApi| AMB[InterruptedWithoutToolAmbiguity<br/>保留部分内容]
```

为什么恢复与生命周期正交:恢复状态存放在自己的 `recovery_status` 列上,不与会话的 `idle`/`active`/`failed` 等生命周期状态混用。一个 `failed` 但 `recovery_status=clean` 的会话仍可接受新消息;一个 `idle` 但 `recovery_status=action_required` 的会话会在每条受管理提交路径上被拒绝,直到恢复动作成功。

为什么只清不重放:恢复动作最多清掉 `active_run`、打上恢复标记、写一份不可变报告,**绝不重试**任何已发起的生成或工具调用。这避免了在不确定的终态下重复执行副作用——一旦证据不足以判断 run 是否完成,正确的做法是把决策权交回用户,而不是猜一个状态继续跑。

用户确认机制:`acknowledge_recovery` 在确认时要求传入的 revision 与当前会话 revision 匹配,防止在恢复扫描后又发生了新变更的情况下确认一个陈旧状态。确认动作本身**不会**清掉不确定的恢复效果——它只把 `recovery_status` 从 `action_required` 推进到 `clean`,把"是否接受这次恢复的最终效果"的判断显式交给用户。

## 关键类型与常量

### SessionRecoveryStatus

恢复状态枚举四值:`clean`/`reconciling`/`action_required`/`quarantined`。恢复状态存放在独立列 `recovery_status` 上,不与会话生命周期状态混用。配套列包括 `recovery_revision`(恢复侧修订号,确认时用作乐观并发检查)、`state_revision`(会话状态修订号)、`history_revision`(消息历史修订号)、`active_execution_run_id`(当前认领的活跃 execution run id)。

### 候选扫描

`recovery_candidates_after` 选出需要恢复的会话,条件为:非 `archived`;`recovery_status NOT IN (action_required, quarantined)`;`active_execution_run_id IS NOT NULL` 或生命周期 ∈ `starting`/`running` 或 `recovery_status='reconciling'`(三选一即可)。

### 原子 claim

`claim_recovery_candidate` 做乐观并发认领:以条件 UPDATE 抢占一个候选,条件不满足时返回 `None`,调用方视为 stale 并跳过——保证同一候选不会被两个恢复 worker 同时认领。

### 终态证据

`SessionTerminalEvidence` 承载终态证据,带两条上限:消息上限 256 条,操作上限 32 条。`ExecutionEvidenceFidelity` 标识证据的可见度,取值 `ManagedApi`(API 直连,可见度高)/`ManagedCliOpaque`(CLI 包装但受管)/`InteractiveOpaque`(交互式、不透明)。

### decide_recovery 决策顺序

`decide_recovery` 按以下顺序判定(先命中先返回):存储瞬时态不一致 → `RetryLater`;存在 live handle → `RetryLater`;消息序列非法 → `Quarantined`;无 active run → `action_required`;run 不一致 → `action_required`;无 assistant 消息 → `action_required`,多于一条 assistant 消息 → `Quarantined`;存在未完成的工具活动 → `action_required`;单一终态 → `clean` + 终态标签(`completed`/`failed`/`cancelled`);多个冲突终态 → `action_required`;无终态、无工具活动、`ManagedApi` → `InterruptedWithoutToolAmbiguity`,保留部分内容;CLI opaque(`ManagedCliOpaque`/`InteractiveOpaque`)→ `action_required`。

### acknowledge_recovery

确认时要求传入的 `expected_recovery_revision` 与当前 `recovery_revision` 匹配,否则返回 `RecoveryRevisionConflict`/`RecoveryActionNotAllowed`。确认动作不清不确定的恢复效果,也不重试任何工作。

### canonical Run 恢复

`reconcile_startup` 对 canonical Run 做启动期恢复:若存在 live lease 则跳过(仍在运行,无需恢复);否则聚合其下各子会话的决策结果。

### 幂等

`run_until_drained` 逐批游标推进,游标 `after_session_id` 是内存中的循环变量(每批处理完更新),不在批间持久化;若扫描期间存储暂时不可用,该批候选会被标记为 `deferred` 而非丢弃。`run_startup_with_retry` 以 `RecoveryTrigger::Startup` 跑首轮,仅当首轮 `deferred > 0`(因存储暂时不可用而延迟)时才以 `RecoveryTrigger::ExplicitRetry` 跑第二轮,保证幂等。

## 设计所在之处

本章用于引导贡献者。权威需求——恢复状态、持久执行标识与归属，以及允许的恢复动作——位于 spec 中。

- [openspec/specs/session-recovery](../../../../openspec/specs/session-recovery/spec.md)

会话持久性位于 `sessions` 限界上下文中；见 [Native bounded contexts](native-contexts.md)。
