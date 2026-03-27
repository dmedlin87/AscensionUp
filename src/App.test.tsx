import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import type {
  AppSnapshot,
  InstallerUpdateStatus,
  OperationResult,
  PathInspection,
} from "./domain/types";

const apiMocks = vi.hoisted(() => ({
  bootstrapApp: vi.fn<() => Promise<AppSnapshot>>(),
  checkInstallerUpdate: vi.fn<() => Promise<InstallerUpdateStatus | null>>(),
  inspectGamePath: vi.fn<(selectedPath: string) => Promise<PathInspection>>(),
  confirmGamePath: vi.fn<
    (gamePath: string, addonPath: string, gameExecutablePath?: string | null) => Promise<AppSnapshot>
  >(),
  installAddon: vi.fn<
    (addonId: string, allowWhileGameRunning: boolean) => Promise<OperationResult>
  >(),
  updateAddon: vi.fn<
    (addonId: string, allowWhileGameRunning: boolean) => Promise<OperationResult>
  >(),
  updateAllAddons: vi.fn<
    (allowWhileGameRunning: boolean) => Promise<OperationResult>
  >(),
  rollbackAddon: vi.fn<
    (addonId: string, allowWhileGameRunning: boolean) => Promise<OperationResult>
  >(),
  openLogsFolder: vi.fn<() => Promise<boolean>>(),
}));

vi.mock("./app/api", () => ({
  bootstrapApp: apiMocks.bootstrapApp,
  checkInstallerUpdate: apiMocks.checkInstallerUpdate,
  inspectGamePath: apiMocks.inspectGamePath,
  confirmGamePath: apiMocks.confirmGamePath,
  installAddon: apiMocks.installAddon,
  updateAddon: apiMocks.updateAddon,
  updateAllAddons: apiMocks.updateAllAddons,
  rollbackAddon: apiMocks.rollbackAddon,
  openLogsFolder: apiMocks.openLogsFolder,
  CommandError: class CommandError extends Error {
    code: string;
    details?: string | null;

    constructor(code: string, message: string, details?: string | null) {
      super(message);
      this.code = code;
      this.details = details;
    }
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

const configuredSnapshot: AppSnapshot = {
  installerVersion: "1.0.0",
  selectedTarget: "Bronzebeard",
  gamePath: "C:\\Games\\Ascension",
  gameExecutablePath: "C:\\Games\\Ascension\\Ascension.exe",
  addonPath: "C:\\Games\\Ascension\\Resources\\Client\\Interface\\Addons",
  pathVerification: "verified",
  pathMessage: "Found one valid addon directory.",
  needsSetup: false,
  catalogStatus: "live",
  catalogMessage: null,
  catalogUrl: "https://example.test/catalog.json",
  lastCatalogRefreshAt: "2026-03-26T12:00:00Z",
  addonRows: [
    {
      addonId: "priest-helper",
      displayName: "Priest Helper",
      description: "Supports class helper overlays.",
      repoAttribution: "owner/priest-helper",
      repoUrl: "https://github.com/owner/priest-helper",
      managedFolders: ["PriestHelper"],
      installedVersion: "1.0.0",
      latestVersion: "1.1.0",
      latestPublishedAt: "2026-03-25T00:00:00Z",
      lastInstalledAt: "2026-03-20T00:00:00Z",
      releaseNotes: "Improves priest logic.",
      status: "updateAvailable",
      errorMessage: null,
      disabledReason: null,
      canInstall: false,
      canUpdate: true,
      canRollback: true,
      iconUrl: null,
    },
  ],
  logDirectory: "C:\\Users\\dmedl\\AppData\\Local\\AscensionAddonInstaller\\logs",
  gameRunning: false,
  installerReleasePageUrl: "https://github.com/owner/repo/releases/latest",
};

describe("App", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    apiMocks.bootstrapApp.mockResolvedValue(configuredSnapshot);
    apiMocks.checkInstallerUpdate.mockResolvedValue({
      currentVersion: "1.0.0",
      latestVersion: "1.0.1",
      downloadUrl: "https://github.com/owner/repo/releases/latest/download/Ascension.zip",
      releasePageUrl: "https://github.com/owner/repo/releases/latest",
      publishedAt: "2026-03-26T00:00:00Z",
      available: true,
      message: "A newer installer version is available.",
    });
    apiMocks.updateAllAddons.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.installAddon.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.updateAddon.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.rollbackAddon.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.openLogsFolder.mockResolvedValue(true);
  });

  it("renders the configured addon list", async () => {
    render(<App />);

    expect(
      await screen.findByRole("heading", { name: /Priest Helper/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Update Available/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Update All/i })).toBeInTheDocument();
  });

  it("shows setup guidance when the game path is missing", async () => {
    apiMocks.bootstrapApp.mockResolvedValue({
      ...configuredSnapshot,
      needsSetup: true,
      gamePath: null,
      addonPath: null,
      addonRows: [],
      pathVerification: "invalid",
      pathMessage: "Choose an Ascension folder or executable to begin.",
    });

    render(<App />);

    expect(
      await screen.findByRole("heading", {
        name: /Point the installer at the correct Ascension client/i,
      }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Choose Folder/i })).toBeInTheDocument();
  });
});
