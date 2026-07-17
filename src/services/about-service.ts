import { ServiceError } from "./service-error";

export const aboutRepositoryUrl = "https://github.com/cdavid817/vanehub-ai";
export const aboutReleasesUrl = `${aboutRepositoryUrl}/releases`;
export const aboutCurrentVersion = "0.1.0";
export const aboutBuildChannel = "Preview";

const latestReleaseApiUrl = "https://api.github.com/repos/cdavid817/vanehub-ai/releases/latest";

export interface AboutUpdateInfo {
  checkedAt: string;
  currentVersion: string;
  latestVersion: string;
  releaseName: string;
  releaseNotes: string;
  releaseUrl: string;
  updateAvailable: boolean;
}

interface GitHubReleasePayload {
  body?: string;
  html_url?: string;
  name?: string;
  tag_name?: string;
}

function normalizeVersion(version: string) {
  return version.trim().replace(/^v/i, "");
}

export function compareVersions(left: string, right: string) {
  const leftParts = normalizeVersion(left).split(/[.-]/);
  const rightParts = normalizeVersion(right).split(/[.-]/);
  const maxLength = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < maxLength; index += 1) {
    const leftPart = Number.parseInt(leftParts[index] ?? "0", 10);
    const rightPart = Number.parseInt(rightParts[index] ?? "0", 10);
    const safeLeft = Number.isNaN(leftPart) ? 0 : leftPart;
    const safeRight = Number.isNaN(rightPart) ? 0 : rightPart;

    if (safeLeft > safeRight) return 1;
    if (safeLeft < safeRight) return -1;
  }

  return 0;
}

function isReleasePayload(value: unknown): value is GitHubReleasePayload {
  return typeof value === "object" && value !== null && "tag_name" in value;
}

export async function checkAboutUpdates(fetchImpl: typeof fetch = fetch): Promise<AboutUpdateInfo> {
  const response = await fetchImpl(latestReleaseApiUrl, {
    headers: {
      Accept: "application/vnd.github+json",
    },
  });

  if (!response.ok) {
    throw new ServiceError("runtime", `GitHub release check failed with HTTP ${response.status}`);
  }

  const payload: unknown = await response.json();
  if (!isReleasePayload(payload) || typeof payload.tag_name !== "string") {
    throw new ServiceError("runtime", "GitHub release response did not include a tag name");
  }

  const latestVersion = normalizeVersion(payload.tag_name);
  return {
    checkedAt: new Date().toISOString(),
    currentVersion: aboutCurrentVersion,
    latestVersion,
    releaseName: typeof payload.name === "string" && payload.name.length > 0 ? payload.name : payload.tag_name,
    releaseNotes: typeof payload.body === "string" ? payload.body : "",
    releaseUrl: typeof payload.html_url === "string" && payload.html_url.length > 0 ? payload.html_url : aboutReleasesUrl,
    updateAvailable: compareVersions(latestVersion, aboutCurrentVersion) > 0,
  };
}
