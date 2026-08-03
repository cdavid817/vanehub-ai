import catalogDocument from "./onepiece-provider-catalog.json";
import type {
  OnePieceProviderEndpoint,
  OnePieceProviderPreset,
  ProviderEndpointType,
} from "../types/agent";

export interface SharedProviderCatalogDocument {
  catalogVersion: number;
  sourceRevisions: { cherryStudio: string; ccSwitch: string };
  providers: Array<Omit<OnePieceProviderPreset, "catalogVersion" | "endpoints" | "interfaceFormat" | "baseUrl" | "modelDiscovery"> & {
    endpoints: Partial<Record<ProviderEndpointType, Omit<OnePieceProviderEndpoint, "type">>>;
  }>;
}

export function getSharedProviderCatalog(): SharedProviderCatalogDocument {
  return catalogDocument as unknown as SharedProviderCatalogDocument;
}

export function getOnePieceProviderPresets(): OnePieceProviderPreset[] {
  const catalog = getSharedProviderCatalog();
  return catalog.providers.map((provider) => {
    const endpoints = Object.entries(provider.endpoints).map(([type, endpoint]) => ({
      ...endpoint,
      type: type as ProviderEndpointType,
      modelDiscovery: { ...endpoint.modelDiscovery },
    }));
    const defaultEndpoint = endpoints.find((endpoint) => endpoint.type === provider.defaultEndpointType);
    if (!defaultEndpoint) throw new Error(`Provider ${provider.id} has no default endpoint`);
    return {
    ...provider,
    catalogVersion: catalog.catalogVersion,
    fallbackModels: [...provider.fallbackModels],
    endpoints,
    interfaceFormat: defaultEndpoint.interfaceFormat,
    baseUrl: defaultEndpoint.baseUrl,
    modelDiscovery: { strategy: defaultEndpoint.modelDiscovery.strategy },
  };
  });
}

export function resolveOnePieceProviderPreset(providerId: string, endpointType?: ProviderEndpointType): OnePieceProviderPreset | undefined {
  const provider = getOnePieceProviderPresets().find((candidate) => candidate.id === providerId);
  if (!provider || !endpointType) return provider;
  const endpoint = provider.endpoints.find((candidate) => candidate.type === endpointType);
  if (!endpoint) return undefined;
  return {
    ...provider,
    defaultEndpointType: endpoint.type,
    interfaceFormat: endpoint.interfaceFormat,
    baseUrl: endpoint.baseUrl,
    modelDiscovery: { strategy: endpoint.modelDiscovery.strategy },
  };
}
