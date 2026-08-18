import type {
  AssignSessionCategoryInput,
  CreateSessionCategoryInput,
  KnownProject,
  KnownRemoteWorkspace,
  ProjectInspection,
  RenameSessionCategoryInput,
  Session,
  SessionCategory,
} from "../types/agent";
import type { ExpertRole, SaveExpertRoleInput } from "../types/expert-role";

export interface SessionCategoryService {
  listSessionCategories(): Promise<SessionCategory[]>;
  createSessionCategory(input: CreateSessionCategoryInput): Promise<SessionCategory>;
  renameSessionCategory(input: RenameSessionCategoryInput): Promise<SessionCategory>;
  deleteSessionCategory(categoryId: string): Promise<void>;
  assignSessionCategory(input: AssignSessionCategoryInput): Promise<Session>;
}

export interface ExpertRoleService {
  listExpertRoles(): Promise<ExpertRole[]>;
  saveExpertRole(input: SaveExpertRoleInput): Promise<ExpertRole>;
  deleteExpertRole(roleId: string): Promise<void>;
}

export interface KnownWorkspaceService {
  listKnownProjects(): Promise<KnownProject[]>;
  listKnownRemoteWorkspaces(): Promise<KnownRemoteWorkspace[]>;
  inspectProject(path: string): Promise<ProjectInspection>;
  selectProjectDirectory(): Promise<string | null>;
  selectWorkspaceDirectory(): Promise<string | null>;
}
