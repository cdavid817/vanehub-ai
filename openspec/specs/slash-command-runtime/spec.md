# slash-command-runtime Specification

## Purpose
定义应用级斜杠命令的行为契约：哪些输入被识别为命令、命令在哪些会话中可用、执行后用户看到什么、失败时如何反馈。命令由前端执行并作用于已有的会话与界面能力，不构成发送给模型的消息。
## Requirements
### Requirement: Command-shaped input recognition

系统 SHALL 仅在输入满足全部下列条件时将其识别为命令：输入去除首尾空白后为单行、以单个 `/` 开头、`/` 之后紧跟一个以字母开头且仅含字母、数字与连字符的名称、名称之后为空白或输入结束。不满足者 SHALL 作为普通消息处理。

命令名 SHALL 大小写不敏感。名称之后的部分 SHALL 按连续空白切分为参数序列。

#### Scenario: A bare command is recognised

- **WHEN** 用户提交 `/help`
- **THEN** 系统 SHALL 将其识别为名称为 `help`、参数为空的命令

#### Scenario: Arguments are split on whitespace

- **WHEN** 用户提交 `/mode   plan`
- **THEN** 系统 SHALL 将其识别为名称为 `mode`、参数为 `["plan"]` 的命令

#### Scenario: A filesystem path is not a command

- **WHEN** 用户提交 `/usr/bin/env`
- **THEN** 系统 SHALL 将其作为普通消息处理

#### Scenario: Multi-line input is not a command

- **WHEN** 用户提交首行为 `/help` 但包含换行的多行文本
- **THEN** 系统 SHALL 将其作为普通消息处理

### Requirement: Literal slash escape

系统 SHALL 将以 `//` 开头的输入识别为字面文本转义，并 SHALL 去掉其中一个 `/` 后作为普通消息发送。

该转义存在的原因是未知命令不会被转发给模型，若无转义，真正以 `/` 开头的散文将无法发送。

#### Scenario: Escaped text reaches the model

- **WHEN** 用户提交 `//help`
- **THEN** 系统 SHALL 发送内容为 `/help` 的普通消息
- **AND** 系统 SHALL NOT 执行任何命令

### Requirement: Session-scoped command availability

系统 SHALL 通过单一的会话可用性判定决定斜杠命令是否在当前会话启用。第一版中该判定 SHALL 仅对内置原生 Agent 的会话为真，且 SHALL 依据稳定的 agent 标识而非显示名称。

在未启用的会话中，命令形态的输入 SHALL 作为普通消息处理。

每条命令 SHALL 另行声明自身的适用条件，该条件 SHALL 为其输入参数的纯函数，SHALL NOT 依赖模块级可变状态。

#### Scenario: Commands are inert in an ineligible session

- **WHEN** 当前会话不是内置原生 Agent 会话，用户提交 `/mode execute`
- **THEN** 系统 SHALL 将该输入作为普通消息发送
- **AND** 系统 SHALL NOT 改变执行模式

#### Scenario: A command may require more than the session

- **WHEN** 一条命令的适用条件依赖会话行以外的事实
- **THEN** 该事实 SHALL 作为显式参数传入适用性判定

### Requirement: Command dispatch does not reach the model

系统 SHALL 在提交时判定输入是否被命令层接管，且该判定 SHALL 同步完成，以决定是否放行给消息发送路径。被接管的输入 SHALL NOT 发送给模型，且输入框 SHALL 被清空。

命令自身的执行 MAY 为异步，其结果 SHALL 在完成后呈现。

#### Scenario: A known command is consumed

- **WHEN** 在启用斜杠命令的会话中提交 `/mode execute`
- **THEN** 系统 SHALL 应用该执行模式
- **AND** 系统 SHALL NOT 发送任何消息
- **AND** 输入框 SHALL 被清空

### Requirement: Unknown commands are reported, not forwarded

系统 SHALL 对启用会话中未匹配到任何可用命令的命令形态输入给出错误反馈，且 SHALL NOT 将其发送给模型。反馈 SHALL 指引用户查看命令列表。

静默转发被禁止的原因是用户会误以为消息已送达模型。

#### Scenario: An unknown name produces an error

- **WHEN** 用户提交 `/definitelynotacommand`
- **THEN** 系统 SHALL 呈现一条错误反馈
- **AND** 系统 SHALL NOT 发送任何消息

### Requirement: Invalid arguments are rejected without side effects

系统 SHALL 在命令参数不合法时呈现错误反馈并列出该命令的合法取值，且 SHALL NOT 产生该命令的任何副作用。

#### Scenario: An unsupported value changes nothing

- **WHEN** 用户提交 `/mode nonsense`
- **THEN** 系统 SHALL 呈现列出合法取值的错误反馈
- **AND** 会话的执行模式 SHALL 保持不变

### Requirement: Command output is presented outside the message list

系统 SHALL 在聊天消息列表之外呈现命令输出，且该输出 SHALL 可由用户关闭。命令输出 SHALL NOT 作为消息持久化，SHALL NOT 出现在会话导出中。

该要求的原因是消息列表在每轮发送后会从后端重新拉取，本地注入的条目无法存活。

#### Scenario: Output survives an unrelated refetch

- **WHEN** 命令输出已呈现，随后消息列表因其他原因重新拉取
- **THEN** 命令输出 SHALL 仍然可见

#### Scenario: Output is dismissible

- **WHEN** 用户关闭命令输出
- **THEN** 该输出 SHALL 不再显示

### Requirement: Command discovery through completion and listing

当输入为 `/` 或 `/` 加一个部分名称时，系统 SHALL 呈现当前会话中可用命令的补全候选，并 SHALL 按名称前缀过滤。补全的呈现 SHALL NOT 执行任何命令。

系统 SHALL 提供一条列出当前会话全部可用命令及其用途的命令。列表 SHALL 反映各命令自身的适用条件。

#### Scenario: Typing a prefix offers candidates

- **WHEN** 输入框内容为 `/st`
- **THEN** 系统 SHALL 呈现名称以 `st` 开头的可用命令

#### Scenario: Typing never executes

- **WHEN** 用户逐字符键入 `/mode execute` 而未提交
- **THEN** 系统 SHALL NOT 应用任何执行模式变更

#### Scenario: Listing excludes inapplicable commands

- **WHEN** 某命令的适用条件在当前会话为假，用户请求命令列表
- **THEN** 该命令 SHALL NOT 出现在列表中

### Requirement: Command failures are surfaced and logged

命令执行失败时，系统 SHALL 向用户呈现错误反馈，并 SHALL 通过前端服务边界上报该失败事件，SHALL NOT 由 React 组件直接写入本地日志文件。

#### Scenario: A failing command reports rather than throws

- **WHEN** 一条命令在执行中抛出异常
- **THEN** 系统 SHALL 呈现错误反馈
- **AND** 系统 SHALL 通过服务边界上报该失败

### Requirement: Localized command surfaces

系统 SHALL 为命令的描述、输出与错误反馈提供项目全部受支持语言的文案。命令名称本身 SHALL 保持不翻译，以保证跨语言的可输入性。

#### Scenario: Every supported locale carries the copy

- **WHEN** 应用以任一受支持语言运行
- **THEN** 命令列表、命令输出与错误反馈 SHALL 以该语言呈现
- **AND** 命令名称 SHALL 保持一致的英文形式

