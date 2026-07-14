export const providers = [
  {
    id: "zhipu",
    name: "智谱 AI",
    vendor: "Zhipu AI",
    status: "运行中",
    models: ["GLM-4.5", "GLM-4"],
    key: "sk-***************3f7a",
    endpoint: "https://open.bigmodel.cn/api/paas/v4",
    latency: "128ms",
    successRate: "99.2%",
  },
  {
    id: "anthropic",
    name: "Anthropic",
    vendor: "Anthropic",
    status: "空闲",
    models: ["Claude-3.5", "Claude-3"],
    key: "sk-ant-**********7c2d",
    endpoint: "https://api.anthropic.com/v1",
    latency: "342ms",
    successRate: "97.8%",
  },
  {
    id: "openai",
    name: "OpenAI",
    vendor: "OpenAI",
    status: "已禁用",
    models: ["GPT-4o", "GPT-4"],
    key: "未配置 API Key",
    endpoint: "https://api.openai.com/v1",
    latency: "-",
    successRate: "-",
  },
];

export const sdkPackages = [
  { name: "@anthropic/sdk", description: "Anthropic 官方 SDK", current: "v0.28.0", latest: "v0.28.0", status: "最新", size: "32MB", source: "npm" },
  { name: "openai", description: "OpenAI 官方 SDK", current: "v4.52.0", latest: "v4.67.0", status: "可更新", size: "45MB", source: "npm" },
  { name: "@zhipu/sdk", description: "智谱 AI 官方 SDK", current: "v2.1.4", latest: "v2.1.4", status: "最新", size: "18MB", source: "npm" },
  { name: "@mcp/sdk", description: "MCP 协议 SDK", current: "v1.0.3", latest: "v1.2.0", status: "可更新", size: "12MB", source: "npm" },
  { name: "@tauri-apps/api", description: "Tauri 桌面框架 API", current: "v2.0.2", latest: "v2.0.2", status: "最新", size: "21MB", source: "npm" },
];

export const mcpServers = [
  { id: "codehub", name: "CodeHub MCP", status: "运行中", tools: 12, latency: "42ms", scope: "代码仓库" },
  { id: "filesystem", name: "文件系统 MCP", status: "运行中", tools: 8, latency: "18ms", scope: "本地文件" },
  { id: "browser", name: "Browser MCP", status: "空闲", tools: 6, latency: "86ms", scope: "浏览器自动化" },
];

export const skills = [
  { id: "code-review", name: "代码审查", category: "安全", trigger: "/review", agent: "代码审查员", runs: 32, success: "96.8%" },
  { id: "security-scan", name: "安全扫描", category: "扫描", trigger: "/scan", agent: "代码审查员", runs: 18, success: "100%" },
  { id: "quality-score", name: "质量评分", category: "分析", trigger: "/score", agent: "代码审查员", runs: 15, success: "93.3%" },
  { id: "unit-test", name: "单元测试生成", category: "测试", trigger: "/test", agent: "测试工程师", runs: 28, success: "89.2%" },
  { id: "api-doc", name: "API文档生成", category: "文档", trigger: "/apidoc", agent: "文档生成器", runs: 22, success: "95.4%" },
  { id: "readme", name: "README生成", category: "项目", trigger: "/readme", agent: "文档生成器", runs: 14, success: "92.8%" },
  { id: "chart", name: "数据可视化", category: "图表", trigger: "/chart", agent: "数据分析师", runs: 5, success: "80%" },
  { id: "predict", name: "趋势预测", category: "预测", trigger: "/predict", agent: "数据分析师", runs: 3, success: "66.7%" },
];
