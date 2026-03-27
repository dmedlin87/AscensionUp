export type PathVerification = "verified" | "unverified" | "invalid";
export type CatalogStatus = "live" | "cached" | "unavailable";
export type AddonStatus = "notInstalled" | "installed" | "updateAvailable" | "error";

export interface ErrorPayload {
  code: string;
  message: string;
  details?: string | null;
}

export interface CommandEnvelope<T> {
  data?: T | null;
  error?: ErrorPayload | null;
}

export interface CandidateAddonPath {
  path: string;
  exists: boolean;
  label: string;
}

export interface PathInspection {
  normalizedGamePath: string;
  gameExecutablePath?: string | null;
  verification: PathVerification;
  candidateAddonPaths: CandidateAddonPath[];
  proposedAddonPath?: string | null;
  message: string;
  ascensionHints: string[];
}

export interface AddonRow {
  addonId: string;
  displayName: string;
  description?: string | null;
  repoAttribution: string;
  repoUrl: string;
  managedFolders: string[];
  installedVersion?: string | null;
  latestVersion?: string | null;
  latestPublishedAt?: string | null;
  lastInstalledAt?: string | null;
  releaseNotes?: string | null;
  status: AddonStatus;
  errorMessage?: string | null;
  disabledReason?: string | null;
  canInstall: boolean;
  canUpdate: boolean;
  canUninstall: boolean;
  canRollback: boolean;
  iconUrl?: string | null;
}

export interface AppSnapshot {
  installerVersion: string;
  selectedTarget: string;
  gamePath?: string | null;
  gameExecutablePath?: string | null;
  addonPath?: string | null;
  pathVerification: PathVerification;
  pathMessage?: string | null;
  needsSetup: boolean;
  catalogStatus: CatalogStatus;
  catalogMessage?: string | null;
  catalogUrl: string;
  lastCatalogRefreshAt?: string | null;
  addonRows: AddonRow[];
  logDirectory: string;
  gameRunning: boolean;
  installerReleasePageUrl: string;
}

export interface OperationResult {
  snapshot: AppSnapshot;
  notice?: string | null;
}

export interface InstallerUpdateStatus {
  currentVersion: string;
  latestVersion?: string | null;
  downloadUrl?: string | null;
  releasePageUrl: string;
  publishedAt?: string | null;
  available: boolean;
  message?: string | null;
}
