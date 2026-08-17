# IM connectors

The native side owns IM connector configuration, credentials, routing, and inbound delivery. Remote-workspace and IM workflow is covered in the user guide; this chapter covers the native design.

## Five built-in connectors

Five independently configurable built-in connectors with stable ids: `feishu`, `telegram`, `dingtalk`, `wecom`, and `weixin`. The connector descriptor list returns all five with localized display metadata, configuration fields, capabilities, and an experimental flag. Personal WeChat (`weixin`) is marked experimental; the other four are not.

## First-version direct-message scope

Each connector accepts text **direct messages** only. Group messages and non-text content are excluded from Agent execution in the first version: a group message is acknowledged or consumed without creating a VaneHub message or Agent generation. A valid text direct message is normalized from its platform event and submitted to the shared inbound router.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the spec.

- [openspec/specs/im-connector-management](../../../openspec/specs/im-connector-management/spec.md)

IM connectors live in the `communications` bounded context; see [Native bounded contexts](native-contexts.md).
