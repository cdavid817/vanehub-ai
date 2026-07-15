import type { WorkspaceSnapshot } from "../types/workspace";

export const emptyWorkspaceSnapshot: WorkspaceSnapshot = {
  conversations: [],
  tools: [],
  agentNodes: [],
  chatMessages: [],
};

export const mockWorkspaceSnapshot: WorkspaceSnapshot = {
  conversations: [
    { title: "智能客服优化方案", status: "进行中", agents: "3 Agents", date: "07-14", active: true },
    { title: "数据分析报告生成", status: "进行中", agents: "2 Agents", date: "07-13" },
    { title: "代码审查自动化", status: "已归档", agents: "4 Agents", date: "07-10", archived: true },
    { title: "产品文档协作", status: "已归档", agents: "2 Agents", date: "07-08", archived: true },
    { title: "营销文案创作", status: "已归档", agents: "3 Agents", date: "07-05", archived: true },
  ],
  tools: [
    { label: "技能", iconName: "shield", tone: "text-purple-400" },
    { label: "MCP 服务器", iconName: "wrench", tone: "text-cyan-400" },
    { label: "插件", iconName: "zap", tone: "text-primary" },
    { label: "看板", iconName: "layers", tone: "text-emerald-400" },
    { label: "规则", iconName: "sliders", tone: "text-amber-400" },
    { label: "连接器", iconName: "users", tone: "text-primary" },
  ],
  agentNodes: [
    {
      id: "reviewer",
      title: "代码审查员",
      description: "代码分析 · 安全检测",
      icon: "A",
      x: "left-[7%] top-[9%]",
      tone: "text-purple-400",
    },
    {
      id: "tester",
      title: "测试工程师",
      description: "单元测试 · 集成测试",
      icon: "T",
      x: "right-[30%] top-[9%]",
      tone: "text-cyan-400",
    },
    {
      id: "docs",
      title: "文档生成器",
      description: "文档编写 · 格式转换",
      icon: "D",
      x: "left-[27%] top-[43%]",
      tone: "text-emerald-400",
    },
  ],
  chatMessages: [
    {
      role: "用户",
      content: "优化智能客服的回答质量，重点关注多轮追问、转人工判断和知识库引用。",
      time: "14:20",
    },
    {
      role: "代码审查员",
      content: "已检查当前客服策略模块，建议把转人工规则拆分为意图识别、置信度阈值和兜底策略三层。",
      time: "14:22",
    },
    {
      role: "测试工程师",
      content: "我会补充多轮追问和低置信度场景的回归用例，覆盖 FAQ、订单、退款三类流程。",
      time: "14:24",
    },
    {
      role: "文档生成器",
      content: "已整理优化方案草案，包括配置项说明、上线步骤和客服运营验证清单。",
      time: "14:27",
    },
  ],
};
