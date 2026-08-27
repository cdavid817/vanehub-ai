import type { LazyFeatureLoader } from "../components/lazy-feature";
import type { SettingsPageContext } from "./settings-page-types";

/**
 * Split out of `settings-pages.ts` so the navigation model stays readable: everything here is
 * one lazy first-visit import, and the page list next door is where the product decisions live.
 */
export const loadBasicPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/basic-settings-page")
  .then((module) => ({ default: module.BasicSettingsPage }));
export const loadProvidersPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/cli-management/cli-management-page")
  .then((module) => ({ default: module.CliManagementPage }));
export const loadCliParametersPage: LazyFeatureLoader<SettingsPageContext> = () => import("./cli-parameters/cli-parameters-page")
  .then((module) => ({ default: module.CliParametersPage }));
export const loadExtensionsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/extensions-page")
  .then((module) => ({ default: module.ExtensionsPage }));
export const loadMcpPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/mcp-page")
  .then((module) => ({ default: module.McpPage }));
export const loadAgentConfigurationsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/agent-configurations-page")
  .then((module) => ({ default: module.AgentConfigurationsPage }));
export const loadCodeIntelligencePage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/code-intelligence-page")
  .then((module) => ({ default: module.CodeIntelligencePage }));
export const loadExpertRolesPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/expert-roles-page")
  .then((module) => ({ default: module.ExpertRolesPage }));
export const loadAgentPoliciesPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/agent-policies-page")
  .then((module) => ({ default: module.AgentPoliciesPage }));
export const loadPersonalizationPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/personalization-page")
  .then((module) => ({ default: module.PersonalizationPage }));
export const loadSkillsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/skills-page")
  .then((module) => ({ default: module.SkillsPage }));
export const loadPromptHooksPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/prompt-hooks-page")
  .then((module) => ({ default: module.PromptHooksPage }));
export const loadLocalMediaPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/local-media-page")
  .then((module) => ({ default: module.LocalMediaPage }));
export const loadImPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/im-page")
  .then((module) => ({ default: module.ImPage }));
export const loadSshConnectionsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/ssh-connections-page")
  .then((module) => ({ default: module.SshConnectionsPage }));
export const loadPluginIntegrationsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/plugin-integrations-page")
  .then((module) => ({ default: module.PluginIntegrationsPage }));
export const loadObservabilityPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/observability-settings-page")
  .then((module) => ({ default: module.ObservabilitySettingsPage }));
export const loadUsagePage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/usage-statistics-page")
  .then((module) => ({ default: module.UsageStatisticsPage }));
export const loadDocumentationPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/documentation-page")
  .then((module) => ({ default: module.DocumentationPage }));
export const loadAboutPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/about-page")
  .then((module) => ({ default: module.AboutPage }));
