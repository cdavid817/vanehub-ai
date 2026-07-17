import type {
  ExtensionEnableRequest,
  ExtensionFrameworkRequest,
  ExtensionInstallPreview,
  ExtensionOverview,
} from "../types/extension";
import type { OperationTask } from "../types/operation";

export interface ExtensionService {
  getOverview(): Promise<ExtensionOverview>;
  refreshHealth(): Promise<ExtensionOverview>;
  getInstallPreview(request: ExtensionFrameworkRequest): Promise<ExtensionInstallPreview>;
  install(request: ExtensionFrameworkRequest): Promise<OperationTask>;
  uninstall(request: ExtensionFrameworkRequest): Promise<OperationTask>;
  setEnabled(request: ExtensionEnableRequest): Promise<OperationTask>;
  start(request: ExtensionFrameworkRequest): Promise<OperationTask>;
  stop(request: ExtensionFrameworkRequest): Promise<OperationTask>;
  selfTest(request: ExtensionFrameworkRequest): Promise<OperationTask>;
}
