import type {
  AssignSessionCategoryInput,
  CreateSessionCategoryInput,
  RenameSessionCategoryInput,
  SessionCategory,
} from "../types/agent";
import type { SessionCategoryService } from "./session-organization-service";
import { nowIso } from "./web-mock-clock";
import { listWebSessions, replaceWebSessions, updateWebSession } from "./web-session-state";

let sessionCategories: SessionCategory[] = [];
let nextSessionCategoryId = 1;

function findCategory(categoryId: string) {
  const category = sessionCategories.find((candidate) => candidate.id === categoryId);
  if (!category) {
    throw new Error(`Category not found: ${categoryId}`);
  }
  return category;
}

function validateCategoryName(name: string, exceptId?: string) {
  const trimmed = name.trim();
  if (!trimmed) throw new Error("Category name cannot be empty.");
  const duplicate = sessionCategories.some((category) => category.name === trimmed && category.id !== exceptId);
  if (duplicate) throw new Error("Category name already exists.");
  return trimmed;
}

export const webSessionCategoryClient: SessionCategoryService = {
  async listSessionCategories() {
    return [...sessionCategories].sort((left, right) => left.sortOrder - right.sortOrder || left.name.localeCompare(right.name));
  },

  async createSessionCategory(input: CreateSessionCategoryInput) {
    const timestamp = nowIso();
    const category: SessionCategory = {
      id: `web-category-${nextSessionCategoryId++}`,
      name: validateCategoryName(input.name),
      sortOrder: sessionCategories.length,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    sessionCategories = [...sessionCategories, category];
    return category;
  },

  async renameSessionCategory(input: RenameSessionCategoryInput) {
    const category = findCategory(input.categoryId);
    const timestamp = nowIso();
    const updated = { ...category, name: validateCategoryName(input.name, input.categoryId), updatedAt: timestamp };
    sessionCategories = sessionCategories.map((candidate) => (candidate.id === input.categoryId ? updated : candidate));
    return updated;
  },

  async deleteSessionCategory(categoryId: string) {
    findCategory(categoryId);
    sessionCategories = sessionCategories.filter((category) => category.id !== categoryId);
    replaceWebSessions(listWebSessions().map((session) => (session.categoryId === categoryId ? { ...session, categoryId: null, updatedAt: nowIso() } : session)));
  },

  async assignSessionCategory(input: AssignSessionCategoryInput) {
    if (input.categoryId) findCategory(input.categoryId);
    return updateWebSession(input.sessionId, { categoryId: input.categoryId });
  },
};
