import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App';
import type {
  AppSnapshot,
  InstallerUpdateStatus,
  OperationResult,
  PathInspection,
} from './domain/types';

const apiMocks = vi.hoisted(() => ({
  bootstrapApp: vi.fn<() => Promise<AppSnapshot>>(),
  checkInstallerUpdate: vi.fn<() => Promise<InstallerUpdateStatus | null>>(),
  inspectGamePath: vi.fn<(selectedPath: string) => Promise<PathInspection>>(),
  confirmGamePath: vi.fn<
    (gamePath: string, addonPath: string, gameExecutablePath?: string | null) => Promise<AppSnapshot>
  >(),
  refreshCatalog: vi.fn<() => Promise<AppSnapshot>>(),
  installAddon: vi.fn<(addonId: string) => Promise<OperationResult>>(),
  updateAddon: vi.fn<(addonId: string) => Promise<OperationResult>>(),
  updateAllAddons: vi.fn<() => Promise<OperationResult>>(),
  uninstallAddon: vi.fn<(addonId: string) => Promise<OperationResult>>(),
  rollbackAddon: vi.fn<(addonId: string) => Promise<OperationResult>>(),
  openLogsFolder: vi.fn<() => Promise<boolean>>(),
  dialogOpen: vi.fn(),
}));

vi.mock('./app/api', () => ({
  bootstrapApp: apiMocks.bootstrapApp,
  checkInstallerUpdate: apiMocks.checkInstallerUpdate,
  inspectGamePath: apiMocks.inspectGamePath,
  confirmGamePath: apiMocks.confirmGamePath,
  refreshCatalog: apiMocks.refreshCatalog,
  installAddon: apiMocks.installAddon,
  updateAddon: apiMocks.updateAddon,
  updateAllAddons: apiMocks.updateAllAddons,
  uninstallAddon: apiMocks.uninstallAddon,
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

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: apiMocks.dialogOpen,
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));

const configuredSnapshot: AppSnapshot = {
  installerVersion: '1.0.0',
  selectedTarget: 'Bronzebeard',
  gamePath: 'C:\\Games\\Ascension',
  gameExecutablePath: 'C:\\Games\\Ascension\\Ascension.exe',
  addonPath: 'C:\\Games\\Ascension\\Resources\\Client\\Interface\\Addons',
  pathVerification: 'verified',
  pathMessage: 'Found one valid addon directory.',
  needsSetup: false,
  catalogStatus: 'live',
  catalogMessage: null,
  catalogUrl: 'https://example.test/catalog.json',
  lastCatalogRefreshAt: '2026-03-26T12:00:00Z',
  addonRows: [
    {
      addonId: 'priest-helper',
      displayName: 'Priest Helper',
      description: 'Supports class helper overlays.',
      repoAttribution: 'owner/priest-helper',
      repoUrl: 'https://github.com/owner/priest-helper',
      managedFolders: ['PriestHelper'],
      installedVersion: '1.0.0',
      latestVersion: '1.1.0',
      latestPublishedAt: '2026-03-25T00:00:00Z',
      lastInstalledAt: '2026-03-20T00:00:00Z',
      releaseNotes: 'Improves priest logic.',
      status: 'updateAvailable',
      errorMessage: null,
      disabledReason: null,
      canInstall: false,
      canUpdate: true,
      canUninstall: true,
      canRollback: true,
      iconUrl: null,
    },
  ],
  logDirectory: 'C:\\Users\\dmedl\\AppData\\Local\\AscensionAddonInstaller\\logs',
  gameRunning: false,
  installerReleasePageUrl: 'https://github.com/owner/repo/releases/latest',
};

function hasExactText(expected: string) {
  return (_content: string, element: Element | null) =>
    element?.textContent?.replace(/\\s+/g, ' ').trim() === expected;
}

describe('App', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    apiMocks.bootstrapApp.mockResolvedValue(configuredSnapshot);
    apiMocks.refreshCatalog.mockResolvedValue(configuredSnapshot);
    apiMocks.checkInstallerUpdate.mockResolvedValue({
      currentVersion: '1.0.0',
      latestVersion: '1.0.1',
      downloadUrl: 'https://github.com/owner/repo/releases/latest/download/Ascension.zip',
      releasePageUrl: 'https://github.com/owner/repo/releases/latest',
      publishedAt: '2026-03-26T00:00:00Z',
      available: true,
      message: 'A newer installer version is available.',
    });
    apiMocks.updateAllAddons.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.installAddon.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.updateAddon.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.uninstallAddon.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.rollbackAddon.mockResolvedValue({ snapshot: configuredSnapshot, notice: null });
    apiMocks.openLogsFolder.mockResolvedValue(true);
    apiMocks.dialogOpen.mockResolvedValue(null);
  });

  it('renders the configured addon list', async () => {
    render(<App />);

    expect(
      await screen.findByRole('heading', { name: /Priest Helper/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Upgrade 1\.0\.0 to 1\.1\.0/i)).toBeInTheDocument();
    expect(screen.getByText(hasExactText('Installed: 1.0.0'))).toBeInTheDocument();
    expect(screen.getByText(hasExactText('Latest: 1.1.0'))).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Update All \(1\)/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /Addon Manager/i })).toBeInTheDocument();
  });

  it('shows setup guidance when the game path is missing', async () => {
    apiMocks.bootstrapApp.mockResolvedValue({
      ...configuredSnapshot,
      needsSetup: true,
      gamePath: null,
      addonPath: null,
      addonRows: [],
      pathVerification: 'invalid',
      pathMessage: 'Choose an Ascension folder or executable to begin.',
    });

    render(<App />);

    expect(
      await screen.findByRole('heading', {
        name: /Bind Install/i,
      }),
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /Welcome to AscensionUp/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /Setup Required/i })).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /Choose Folder/i }).length).toBeGreaterThan(0);
  });

  it('shows an in-app error when the dialog bridge is unavailable', async () => {
    apiMocks.bootstrapApp.mockResolvedValue({
      ...configuredSnapshot,
      needsSetup: true,
      gamePath: null,
      addonPath: null,
      addonRows: [],
      pathVerification: 'invalid',
      pathMessage: 'Choose an Ascension folder or executable to begin.',
    });
    apiMocks.dialogOpen.mockRejectedValue(
      new TypeError(`Cannot read properties of undefined (reading 'invoke')`),
    );

    render(<App />);

    const buttons = await screen.findAllByRole('button', { name: /Choose Folder/i });
    fireEvent.click(buttons[0]);

    expect(
      await screen.findByText(/Launch the app with `npm run tauri dev`/i),
    ).toBeInTheDocument();
  });

  it('confirms and runs uninstall for an installed addon', async () => {
    render(<App />);

    fireEvent.click(await screen.findByRole('button', { name: /Uninstall/i }));

    await waitFor(() =>
      expect(window.confirm).toHaveBeenCalledWith(
        'Uninstall Priest Helper? This removes only the managed addon folders from your AddOns directory.',
      ),
    );
    await waitFor(() =>
      expect(apiMocks.uninstallAddon).toHaveBeenCalledWith('priest-helper'),
    );
  });

  it('confirms and runs rollback for an updated addon', async () => {
    render(<App />);

    fireEvent.click(await screen.findByRole('button', { name: 'Rollback' }));

    await waitFor(() =>
      expect(window.confirm).toHaveBeenCalledWith(
        'Rollback Priest Helper to its previously installed version?',
      ),
    );
    await waitFor(() =>
      expect(apiMocks.rollbackAddon).toHaveBeenCalledWith('priest-helper'),
    );
  });

  it('disables the Update All button when there are no updates available', async () => {
    apiMocks.bootstrapApp.mockResolvedValue({
      ...configuredSnapshot,
      addonRows: [
        {
          ...configuredSnapshot.addonRows[0],
          status: 'installed',
          canUpdate: false,
        },
      ],
    });

    render(<App />);

    expect(await screen.findByRole('button', { name: /Update All/i })).toBeDisabled();
  });
});
