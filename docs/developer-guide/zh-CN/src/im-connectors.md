# IM connector

native 侧负责 IM connector 的配置、凭证、路由与入站投递。远程工作空间与 IM 工作流在用户指南中介绍;本章介绍 native 侧设计。

## 五个内置 connector

五个可独立配置的内置 connector,具有稳定 id:`feishu`、`telegram`、`dingtalk`、`wecom` 与 `weixin`。connector 描述符列表返回全部五个 connector,并附带本地化的展示元数据、配置字段、能力以及实验性标志。个人微信(`weixin`)被标记为实验性;其余四个不是。

## 第一版本的直接消息范围

每个 connector 仅接受文本**直接消息(direct message)**。群组消息与非文本内容在第一版本中被排除在 Agent 执行之外:一条群组消息会被确认或消费,但不会创建 VaneHub 消息或 Agent 生成。一条有效的文本直接消息会从其平台事件中归一化,并提交至共享入站路由器。

## 设计所在

本章用于为贡献者定向。权威需求位于 spec 中。

- [openspec/specs/im-connector-management](../../../../openspec/specs/im-connector-management/spec.md)

IM connector 位于 `communications` 限界上下文;参见 [Native bounded contexts](native-contexts.md)。
