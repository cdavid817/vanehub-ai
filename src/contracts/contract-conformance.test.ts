import { describe, expect, it } from "vitest";
import type * as AgentContracts from "./agent";
import type * as McpContracts from "./mcp";
import type * as SdkContracts from "./sdk";
import type * as OperationContracts from "./operation";
import type * as AgentTypes from "../types/agent";
import type * as McpTypes from "../types/mcp";
import type * as SdkTypes from "../types/sdk";
import type * as OperationTypes from "../types/operation";

type Equal<Left, Right> =
  (<Value>() => Value extends Left ? 1 : 2) extends
  (<Value>() => Value extends Right ? 1 : 2)
    ? (<Value>() => Value extends Right ? 1 : 2) extends
      (<Value>() => Value extends Left ? 1 : 2)
      ? true
      : false
    : false;

type Assert<T extends true> = T;

type AgentAssertions = [
  Assert<Equal<AgentContracts.InteractionMode, AgentTypes.InteractionMode>>,
  Assert<Equal<AgentContracts.AvailabilityState, AgentTypes.AvailabilityState>>,
  Assert<Equal<AgentContracts.SessionLifecycleState, AgentTypes.SessionLifecycleState>>,
  Assert<Equal<AgentContracts.LaunchMetadata, AgentTypes.LaunchMetadata>>,
  Assert<Equal<AgentContracts.AgentRegistryEntry, AgentTypes.AgentRegistryEntry>>,
  Assert<Equal<AgentContracts.WorkflowState, AgentTypes.WorkflowState>>,
  Assert<Equal<AgentContracts.ReadinessStatus, AgentTypes.ReadinessStatus>>,
  Assert<Equal<AgentContracts.LaunchResult, AgentTypes.LaunchResult>>,
  Assert<Equal<AgentContracts.SessionDetails, AgentTypes.SessionDetails>>,
];

type McpAssertions = [
  Assert<Equal<McpContracts.McpTransportType, McpTypes.McpTransportType>>,
  Assert<Equal<McpContracts.McpConnectionStatus, McpTypes.McpConnectionStatus>>,
  Assert<Equal<McpContracts.McpScope, McpTypes.McpScope>>,
  Assert<Equal<McpContracts.McpServerConfig, McpTypes.McpServerConfig>>,
  Assert<Equal<McpContracts.PartialMcpServerConfig, McpTypes.PartialMcpServerConfig>>,
  Assert<Equal<McpContracts.McpToolInfo, McpTypes.McpToolInfo>>,
  Assert<Equal<McpContracts.McpServerStatus, McpTypes.McpServerStatus>>,
  Assert<Equal<McpContracts.McpTestResult, McpTypes.McpTestResult>>,
  Assert<Equal<McpContracts.McpImportResult, McpTypes.McpImportResult>>,
  Assert<Equal<McpContracts.McpImportServerEntry, McpTypes.McpImportServerEntry>>,
  Assert<Equal<McpContracts.McpImportExport, McpTypes.McpImportExport>>,
];

type SdkAssertions = [
  Assert<Equal<SdkContracts.SdkId, SdkTypes.SdkId>>,
  Assert<Equal<SdkContracts.SdkInstallStatus, SdkTypes.SdkInstallStatus>>,
  Assert<Equal<SdkContracts.SdkVersionSource, SdkTypes.SdkVersionSource>>,
  Assert<Equal<SdkContracts.SdkOperationType, SdkTypes.SdkOperationType>>,
  Assert<Equal<SdkContracts.SdkDefinition, SdkTypes.SdkDefinition>>,
  Assert<Equal<SdkContracts.SdkStatus, SdkTypes.SdkStatus>>,
  Assert<Equal<SdkContracts.SdkVersionInfo, SdkTypes.SdkVersionInfo>>,
  Assert<Equal<SdkContracts.SdkEnvironmentStatus, SdkTypes.SdkEnvironmentStatus>>,
  Assert<Equal<SdkContracts.SdkOperationLog, SdkTypes.SdkOperationLog>>,
  Assert<Equal<SdkContracts.SdkOperationRequest, SdkTypes.SdkOperationRequest>>,
  Assert<Equal<SdkContracts.SdkOperationResult, SdkTypes.SdkOperationResult>>,
  Assert<Equal<SdkContracts.SdkStatusMap, SdkTypes.SdkStatusMap>>,
  Assert<Equal<SdkContracts.SdkVersionMap, SdkTypes.SdkVersionMap>>,
  Assert<Equal<SdkContracts.SdkUpdateMap, SdkTypes.SdkUpdateMap>>,
  Assert<Equal<SdkContracts.SdkVersionAction, SdkTypes.SdkVersionAction>>,
];

type OperationAssertions = [
  Assert<Equal<OperationContracts.OperationKind, OperationTypes.OperationKind>>,
  Assert<Equal<OperationContracts.OperationStatus, OperationTypes.OperationStatus>>,
  Assert<Equal<OperationContracts.OperationLogEntry, OperationTypes.OperationLogEntry>>,
  Assert<Equal<OperationContracts.OperationTask, OperationTypes.OperationTask>>,
];

void (0 as unknown as AgentAssertions);
void (0 as unknown as McpAssertions);
void (0 as unknown as SdkAssertions);
void (0 as unknown as OperationAssertions);

describe("contract conformance", () => {
  it("compiles when committed contracts match frontend service types", () => {
    expect(true).toBe(true);
  });
});
