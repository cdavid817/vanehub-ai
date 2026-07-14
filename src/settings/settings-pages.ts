import {
  Bot,
  Boxes,
  Code2,
  Database,
  Puzzle,
  Settings,
  type LucideIcon,
} from "lucide-react";
import { AgentsPage } from "./pages/agents-page";
import { BasicSettingsPage } from "./pages/basic-settings-page";
import { McpPage } from "./pages/mcp-page";
import { ProvidersPage } from "./pages/providers-page";
import { SdkPage } from "./pages/sdk-page";
import { SkillsPage } from "./pages/skills-page";

export type SettingsPageId = "basic" | "providers" | "sdk" | "mcp" | "agents" | "skills";

export interface SettingsPageContext {
  searchTerm: string;
}

export interface SettingsPageDefinition {
  id: SettingsPageId;
  label: string;
  crumb: string;
  icon: LucideIcon;
  badge?: number;
  searchPlaceholder: string;
  component: (props: SettingsPageContext) => JSX.Element;
}

export const settingsPages: SettingsPageDefinition[] = [
  {
    id: "basic",
    label: "基础配置",
    crumb: "基础配置",
    icon: Settings,
    searchPlaceholder: "搜索设置项...",
    component: BasicSettingsPage,
  },
  {
    id: "providers",
    label: "供应商管理",
    crumb: "供应商管理",
    icon: Database,
    badge: 3,
    searchPlaceholder: "搜索供应商...",
    component: ProvidersPage,
  },
  {
    id: "sdk",
    label: "SDK 依赖",
    crumb: "SDK 依赖",
    icon: Code2,
    badge: 5,
    searchPlaceholder: "搜索 SDK...",
    component: SdkPage,
  },
  {
    id: "mcp",
    label: "MCP 服务器",
    crumb: "MCP 服务器",
    icon: Boxes,
    badge: 3,
    searchPlaceholder: "搜索 MCP...",
    component: McpPage,
  },
  {
    id: "agents",
    label: "Agents",
    crumb: "Agents",
    icon: Bot,
    badge: 4,
    searchPlaceholder: "搜索 Agent...",
    component: AgentsPage,
  },
  {
    id: "skills",
    label: "Skills",
    crumb: "Skills",
    icon: Puzzle,
    badge: 8,
    searchPlaceholder: "搜索 Skill...",
    component: SkillsPage,
  },
];

export const defaultSettingsPageId: SettingsPageId = "basic";

export function getSettingsPage(id: SettingsPageId) {
  return settingsPages.find((page) => page.id === id) ?? settingsPages[0];
}
