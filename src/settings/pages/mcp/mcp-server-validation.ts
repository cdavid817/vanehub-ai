import { z } from "zod";
import type { McpScope, McpServerConfig, McpTransportType } from "../../../types/mcp";

export type McpServerFormMessages = {
  name: string;
  commandRequired: string;
  urlRequired: string;
  jsonObject: string;
  argsArray: string;
};

const defaultMessages: McpServerFormMessages = {
  name: "Name must be kebab-case lowercase letters, numbers, and hyphens",
  commandRequired: "stdio MCP server requires Command",
  urlRequired: "URL MCP server requires URL",
  jsonObject: "Must be a JSON object",
  argsArray: "args JSON must be an array",
};

function mcpServerFormSchema(messages: McpServerFormMessages) {
  return z
    .object({
    name: z.string().trim().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/, messages.name),
    transportType: z.enum(["stdio", "sse", "streamable_http"]),
    scope: z.enum(["user", "project"]),
    command: z.string(),
    args: z.string(),
    env: z.string(),
    url: z.string(),
    headers: z.string(),
    description: z.string(),
    active: z.boolean(),
  })
  .superRefine((value, ctx) => {
    if (value.transportType === "stdio" && !value.command.trim()) {
      ctx.addIssue({
        code: "custom",
        message: messages.commandRequired,
        path: ["command"],
      });
    }

    if (value.transportType !== "stdio" && !value.url.trim()) {
      ctx.addIssue({
        code: "custom",
        message: messages.urlRequired,
        path: ["url"],
      });
    }
  });
}

export type McpServerFormValues = {
  name: string;
  transportType: McpTransportType;
  scope: McpScope;
  command: string;
  args: string;
  env: string;
  url: string;
  headers: string;
  description: string;
  active: boolean;
};

export type McpServerFormErrors = Partial<Record<keyof McpServerFormValues | "form", string>>;

function parseRecord(
  value: string,
  label: keyof McpServerFormValues,
  messages: McpServerFormMessages,
): { value?: Record<string, string>; error?: McpServerFormErrors } {
  try {
    const parsed: unknown = value.trim() ? JSON.parse(value) : {};
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return { error: { [label]: messages.jsonObject } };
    }
    return {
      value: Object.fromEntries(Object.entries(parsed as Record<string, unknown>).map(([key, item]) => [key, String(item)])),
    };
  } catch (err) {
    return { error: { [label]: err instanceof Error ? err.message : String(err) } };
  }
}

function parseArgs(value: string, messages: McpServerFormMessages): { value?: string[]; error?: McpServerFormErrors } {
  const trimmed = value.trim();
  if (!trimmed) return { value: [] };
  if (trimmed.startsWith("[")) {
    try {
      const parsed: unknown = JSON.parse(trimmed);
      if (!Array.isArray(parsed)) return { error: { args: messages.argsArray } };
      return { value: parsed.map(String) };
    } catch (err) {
      return { error: { args: err instanceof Error ? err.message : String(err) } };
    }
  }
  return { value: trimmed.split(/\r?\n/).map((item) => item.trim()).filter(Boolean) };
}

export function validateMcpServerForm(
  values: McpServerFormValues,
  messages: McpServerFormMessages = defaultMessages,
): { success: true; config: McpServerConfig } | { success: false; errors: McpServerFormErrors } {
  const parsed = mcpServerFormSchema(messages).safeParse(values);
  if (!parsed.success) {
    return {
      success: false,
      errors: Object.fromEntries(
        parsed.error.issues.map((issue) => [String(issue.path[0] ?? "form"), issue.message]),
      ) as McpServerFormErrors,
    };
  }

  const args = parseArgs(values.args, messages);
  if (args.error) return { success: false, errors: args.error };

  const env = parseRecord(values.env, "env", messages);
  if (env.error) return { success: false, errors: env.error };

  const headers = parseRecord(values.headers, "headers", messages);
  if (headers.error) return { success: false, errors: headers.error };

  return {
    success: true,
    config: {
      name: values.name.trim(),
      transportType: values.transportType,
      command: values.transportType === "stdio" ? values.command.trim() : null,
      args: values.transportType === "stdio" ? args.value ?? [] : null,
      env: values.transportType === "stdio" ? env.value ?? {} : null,
      url: values.transportType !== "stdio" ? values.url.trim() : null,
      headers: values.transportType !== "stdio" ? headers.value ?? {} : null,
      description: values.description.trim() || null,
      active: values.active,
      scope: values.scope,
    },
  };
}
