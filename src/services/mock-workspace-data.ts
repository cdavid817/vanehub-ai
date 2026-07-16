import type { WorkspaceSnapshot } from "../types/workspace";

export const emptyWorkspaceSnapshot: WorkspaceSnapshot = {
  conversations: [],
  tools: [],
  agentNodes: [],
  chatMessages: [],
};

export const mockWorkspaceSnapshot: WorkspaceSnapshot = {
  conversations: [
    { title: "Customer Support Optimization", status: "Running", agents: "3 Agents", date: "07-14", active: true },
    { title: "Data Analysis Report", status: "Running", agents: "2 Agents", date: "07-13" },
    { title: "Code Review Automation", status: "Archived", agents: "4 Agents", date: "07-10", archived: true },
    { title: "Product Docs Collaboration", status: "Archived", agents: "2 Agents", date: "07-08", archived: true },
    { title: "Marketing Copywriting", status: "Archived", agents: "3 Agents", date: "07-05", archived: true },
  ],
  tools: [
    { label: "Skills", iconName: "shield", tone: "text-purple-400" },
    { label: "MCP Servers", iconName: "wrench", tone: "text-cyan-400" },
    { label: "Plugins", iconName: "zap", tone: "text-primary" },
    { label: "Board", iconName: "layers", tone: "text-emerald-400" },
    { label: "Rules", iconName: "sliders", tone: "text-amber-400" },
    { label: "Connectors", iconName: "users", tone: "text-primary" },
  ],
  agentNodes: [
    {
      id: "reviewer",
      title: "Code Reviewer",
      description: "Code analysis · Security checks",
      icon: "A",
      x: "left-[7%] top-[9%]",
      tone: "text-purple-400",
    },
    {
      id: "tester",
      title: "Test Engineer",
      description: "Unit tests · Integration tests",
      icon: "T",
      x: "right-[30%] top-[9%]",
      tone: "text-cyan-400",
    },
    {
      id: "docs",
      title: "Docs Generator",
      description: "Documentation · Format conversion",
      icon: "D",
      x: "left-[27%] top-[43%]",
      tone: "text-emerald-400",
    },
  ],
  chatMessages: [
    {
      role: "User",
      content: "Improve customer support answer quality, especially follow-up questions, handoff decisions, and knowledge references.",
      time: "14:20",
    },
    {
      role: "Code Reviewer",
      content: "I checked the support policy module and recommend splitting handoff rules into intent detection, confidence thresholds, and fallback policy.",
      time: "14:22",
    },
    {
      role: "Test Engineer",
      content: "I will add regression cases for follow-up and low-confidence scenarios across FAQ, order, and refund flows.",
      time: "14:24",
    },
    {
      role: "Docs Generator",
      content: "I prepared an optimization draft with configuration notes, rollout steps, and validation checklist.",
      time: "14:27",
    },
  ],
};
