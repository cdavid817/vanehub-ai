# goal-management Specification

## Purpose
目标能力让用户把分散在循环与工作看板中的执行体归拢到一个可追踪的顶层目标之下，用一处视图回答「这件事推进到哪一步了」，并以人工验收作为目标达成的唯一权威判定。
## Requirements
### Requirement: 目标的持久化状态

系统 SHALL 为每个目标持久化以下四种状态之一：草稿、进行中、已达成、已放弃。系统 MUST NOT 持久化「待验收」状态。

目标 SHALL 至少携带标题；描述与验收说明为可选。验收说明是供人阅读的判断依据，系统 MUST NOT 用它做任何机器判定。

#### Scenario: 新建目标进入草稿

- **WHEN** 用户创建一个目标
- **THEN** 该目标状态为草稿

#### Scenario: 标题缺失被拒绝

- **WHEN** 用户创建目标时未提供标题或标题仅含空白字符
- **THEN** 系统拒绝创建并返回可读的校验错误

#### Scenario: 启用目标

- **WHEN** 用户启用一个草稿状态的目标
- **THEN** 该目标状态变为进行中

#### Scenario: 放弃与重新启用

- **WHEN** 用户放弃任意状态的目标
- **THEN** 该目标状态变为已放弃
- **WHEN** 用户重新启用一个已放弃的目标
- **THEN** 该目标状态变为进行中

### Requirement: 待验收状态按需推导

系统 SHALL 在每次读取目标时，依据其关联子项的当前状态推导该目标是否待验收，并 MUST NOT 将推导结果写入持久化存储。

当且仅当目标持久化状态为进行中、其可推导子项数量大于零、且全部可推导子项均处于终态时，系统 SHALL 将该目标呈现为待验收。

#### Scenario: 全部子项达终态后呈现待验收

- **WHEN** 某进行中目标关联的全部可推导子项都进入终态
- **THEN** 下一次读取该目标时其呈现状态为待验收

#### Scenario: 子项被重开后自动回退

- **WHEN** 某待验收目标的一个子项重新回到非终态
- **THEN** 下一次读取该目标时其呈现状态回到进行中，且无需任何人工操作

#### Scenario: 无可推导子项不呈现待验收

- **WHEN** 某进行中目标没有任何可推导子项
- **THEN** 该目标呈现状态为进行中，而非待验收

### Requirement: 目标与执行体的关联

系统 SHALL 允许把循环、工作看板项、会话与规范化 Run 关联到目标。同一目标与同一个对象之间 MUST 至多存在一条关联。系统 MUST 拒绝新建 Plan 关联；升级前已有的 Plan 关联只作为不可解析的历史记录展示。

系统 SHALL 允许解除任意关联。解除关联 MUST NOT 影响被关联对象自身的状态或数据。

建立关联 MUST NOT 修改被关联对象。执行体既有的目标文本字段保持原样，关联关系仅由目标一侧持有。

#### Scenario: 建立关联

- **WHEN** 用户把一个循环关联到某目标
- **THEN** 该循环出现在目标的子项列表中，且循环自身的数据未被修改

#### Scenario: 新建 Plan 关联被拒绝

- **WHEN** 用户尝试把历史 Plan 对象关联到目标
- **THEN** 系统拒绝该操作并返回可读错误

#### Scenario: 重复关联被拒绝

- **WHEN** 用户把一个已关联到某目标的对象再次关联到同一目标
- **THEN** 系统拒绝该操作并返回可读错误，且不产生重复记录

#### Scenario: 解除关联

- **WHEN** 用户解除某个关联
- **THEN** 该子项从目标的子项列表中消失，被关联对象本身保持不变

### Requirement: 循环子项的终态判定

系统 SHALL 在循环运行处于已成功、已失败或已取消时将其视为终态。循环运行停留在待验收时 MUST NOT 视为终态。

#### Scenario: 失败的循环算终态

- **WHEN** 某目标关联的循环运行处于已失败
- **THEN** 该子项计为终态

#### Scenario: 停在待验收的循环不算终态

- **WHEN** 某目标关联的循环运行停留在待验收
- **THEN** 该子项计为非终态，目标不进入待验收

### Requirement: 工作看板子项的终态判定

系统 SHALL 在看板项处于完成阶段时将其视为终态。已归档的看板项同样 SHALL 视为终态。

#### Scenario: 完成阶段的看板项算终态

- **WHEN** 某目标关联的看板项被移动到完成阶段
- **THEN** 该子项计为终态

#### Scenario: 归档的看板项算终态

- **WHEN** 某目标关联的看板项被归档
- **THEN** 该子项计为终态

### Requirement: 会话关联不参与达成推导

会话关联 MUST NOT 参与达成推导，无论会话处于何种状态。会话没有「完成」语义。

会话与目标的关联 SHALL 仅能由用户显式建立。系统 MUST NOT 在会话创建时自动把它挂载到任何目标。

#### Scenario: 仅关联会话的目标不进入待验收

- **WHEN** 某进行中目标只关联了会话，没有关联任何循环或看板项
- **THEN** 该目标呈现状态为进行中，而非待验收

#### Scenario: 会话不自动挂载

- **WHEN** 用户在存在进行中目标的情况下创建一个新会话
- **THEN** 该会话不会被自动关联到任何目标

### Requirement: 不可解析关联的降级

当关联指向的对象已被删除，或查询其状态失败时，系统 SHALL 把该子项标记为不可解析，MUST NOT 计入达成推导的分母，且 MUST NOT 导致整个目标查询失败。

系统 SHALL 在目标详情中明确呈现不可解析的子项，使阻塞原因对用户可见。

#### Scenario: 被删除的子项不阻塞目标

- **WHEN** 某目标关联的循环已被删除，其余子项均达终态
- **THEN** 该目标呈现状态为待验收，且被删除的子项被标记为不可解析

#### Scenario: 单个查询失败不影响其余子项

- **WHEN** 读取某目标时其中一个子项的状态查询失败
- **THEN** 系统返回该目标，失败的子项标记为不可解析，其余子项状态正常呈现

### Requirement: 人工验收与重开

目标达成 SHALL 只能由人工确认。系统 MUST NOT 自动把任何目标置为已达成。

系统 SHALL 仅在目标呈现状态为待验收时接受验收操作。系统 SHALL 允许把已达成的目标重开为进行中。

#### Scenario: 待验收时验收成功

- **WHEN** 用户对一个呈现状态为待验收的目标执行验收
- **THEN** 该目标持久化状态变为已达成

#### Scenario: 非待验收时验收被拒绝

- **WHEN** 用户对一个仍有子项处于非终态的目标执行验收
- **THEN** 系统拒绝该操作并返回可读错误，目标状态不变

#### Scenario: 重开已达成的目标

- **WHEN** 用户重开一个已达成的目标
- **THEN** 该目标持久化状态变为进行中

### Requirement: 运行时行为一致

目标能力 SHALL 在桌面运行时与 Web 运行时均可用，两个运行时对外暴露同一套服务接口且行为一致。

界面 MUST 通过前端服务边界访问目标能力，MUST NOT 直接调用原生命令。

#### Scenario: 两个运行时接口一致

- **WHEN** 同一段界面代码分别运行在桌面运行时与 Web 运行时
- **THEN** 目标的创建、关联、推导与验收行为一致，界面无需分支处理

### Requirement: Goals link to canonical Runs
Goals SHALL support stable links to canonical Runs for progress evidence while retaining existing manual acceptance and derived completion rules; Session links SHALL remain non-contributing.

#### Scenario: Linked Run completes
- **WHEN** a canonical Run linked to a Goal completes
- **THEN** it is available as execution evidence but does not bypass the Goal's existing acceptance rules

### Requirement: Searchable goal execution-target picker
Goal relationship creation SHALL use searchable typed pickers for supported Session, Run, Loop, Work Item, and other canonical execution targets instead of requiring ordinary users to enter raw target ids.

#### Scenario: Search a target
- **WHEN** the user adds a relationship
- **THEN** the picker SHALL query bounded safe summaries from the owning service and identify target type, title, status, project, and stable identity

#### Scenario: Choose a target
- **WHEN** the user selects a valid result
- **THEN** the UI SHALL submit the stable target type and id through the existing goal service contract
- **AND** display labels SHALL not be used as identity

#### Scenario: Paste an advanced id
- **WHEN** an explicitly enabled diagnostic path accepts a raw id
- **THEN** the id SHALL be validated and resolved before submission
- **AND** unresolved ids SHALL not create a relationship

### Requirement: Nonblocking goal mutations
Goal create, edit, lifecycle, relationship, acceptance, and archive mutations SHALL preserve the loaded goal list and detail and disable only conflicting target actions.

#### Scenario: Update a goal
- **WHEN** a goal mutation is pending
- **THEN** the selected goal SHALL show local pending state while unrelated goals and navigation remain operable

#### Scenario: Mutation fails
- **WHEN** the service rejects the change
- **THEN** the prior canonical detail and the user's recoverable input SHALL remain visible with a local error

#### Scenario: Mutation changes selection
- **WHEN** the selected goal is archived or deleted according to existing semantics
- **THEN** the route SHALL choose a deterministic remaining goal or a clear empty state
- **AND** the list SHALL not flash blank

### Requirement: Goal master-detail presentation
The Goal Center SHALL use the shared master-detail layout with a scannable goal list, a bounded identity and progress summary, related execution sections, and grouped state-aware actions.

#### Scenario: Select a goal
- **WHEN** a goal row is activated
- **THEN** the route and detail SHALL identify the stable selected goal
- **AND** list filters and scroll position SHALL remain preserved

#### Scenario: Render goal progress
- **WHEN** status, progress, milestones, or acceptance state exists
- **THEN** the detail SHALL distinguish derived status from user actions and SHALL not rely on color alone

#### Scenario: Create or edit
- **WHEN** the user starts goal creation or editing
- **THEN** a shared editor sheet SHALL open instead of expanding a form in the page header

### Requirement: State-aware goal lifecycle actions
The Goal Center SHALL present one state-appropriate primary action and place permitted secondary or destructive lifecycle actions in a grouped menu with explicit consequences.

#### Scenario: Goal needs manual acceptance
- **WHEN** the domain reports that acceptance is required
- **THEN** the primary action MAY open the authoritative acceptance flow
- **AND** the UI SHALL not mark the goal complete from inferred child state

#### Scenario: Action is incompatible
- **WHEN** a lifecycle transition is not permitted in the current version or state
- **THEN** the action SHALL be absent or disabled with an accessible reason

#### Scenario: Confirm destructive action
- **WHEN** archive, abandon, delete, or another destructive operation is chosen
- **THEN** the confirmation SHALL state the effect on linked objects and SHALL not claim linked Runs or Sessions will be deleted unless true

### Requirement: Goal relationship overview
Goal detail SHALL summarize related milestones, Work Items, Sessions, Runs, Loops, and acceptance evidence as bounded grouped links or a readable relationship view.

#### Scenario: Inspect related execution
- **WHEN** linked targets exist
- **THEN** the detail SHALL show target type, safe title, current state, and EvidenceLink to the owning surface

#### Scenario: Target is missing or restricted
- **WHEN** a stored relationship cannot be resolved or viewed
- **THEN** the relationship SHALL remain identifiable as unavailable or restricted
- **AND** it SHALL not be silently removed by rendering

#### Scenario: Use compact width
- **WHEN** the relationship view cannot fit a graph or multi-column layout
- **THEN** it SHALL fall back to a grouped list with equivalent information and navigation

### Requirement: Responsive Goal Center navigation
Goal list, detail, filters, creation, and lifecycle actions SHALL remain complete in wide master-detail and compact list-then-detail compositions.

#### Scenario: Open detail on compact width
- **WHEN** a compact user selects a goal
- **THEN** the detail SHALL replace or overlay the list with a clear Back action
- **AND** returning SHALL restore the prior list query and anchor

#### Scenario: Use keyboard
- **WHEN** a user navigates goal rows, relationship links, and actions
- **THEN** focus order, selected state, and action availability SHALL remain clear and stable

### Requirement: Responsive goal-center presentation
The goal center SHALL present goal progress, status, linked work, and primary actions with responsive layout, readable hierarchy, and accessible empty, loading, and error states.

#### Scenario: Goal center at narrow width
- **WHEN** the goal center is viewed at a narrow supported desktop width
- **THEN** goal information and primary actions remain readable and operable without overlap or horizontal clipping

