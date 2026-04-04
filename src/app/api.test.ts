import { describe, expect, it, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { bootstrapApp, inspectGamePath, CommandError, confirmGamePath, refreshCatalog, installAddon, updateAddon, updateAllAddons, uninstallAddon, rollbackAddon, checkInstallerUpdate, openLogsFolder } from './api';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

describe('api functions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('invokeCommand wrapper', () => {
    it('returns data when command succeeds', async () => {
      vi.mocked(invoke).mockResolvedValue({
        data: { test: 'value' },
        error: null
      });

      const result = await bootstrapApp();
      expect(result).toEqual({ test: 'value' });
      expect(invoke).toHaveBeenCalledWith('bootstrapApp', undefined);
    });

    it('throws CommandError when envelope has an error', async () => {
      vi.mocked(invoke).mockResolvedValue({
        data: null,
        error: {
          code: 'test_error',
          message: 'An error occurred',
          details: 'Some details'
        }
      });

      await expect(bootstrapApp()).rejects.toThrowError(CommandError);
      try {
        await bootstrapApp();
      } catch (err) {
        expect(err).toBeInstanceOf(CommandError);
        expect((err as CommandError).code).toBe('test_error');
        expect((err as CommandError).message).toBe('An error occurred');
        expect((err as CommandError).details).toBe('Some details');
      }
    });

    it('throws CommandError when data is undefined', async () => {
      vi.mocked(invoke).mockResolvedValue({
        data: undefined,
        error: null
      });

      await expect(bootstrapApp()).rejects.toThrow('The command "bootstrapApp" returned no data.');
    });

    it('throws CommandError when data is null', async () => {
      vi.mocked(invoke).mockResolvedValue({
        data: null,
        error: null
      });

      await expect(bootstrapApp()).rejects.toThrow('The command "bootstrapApp" returned no data.');
    });
  });

  describe('specific functions', () => {
    it('inspectGamePath calls invoke with selectedPath', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await inspectGamePath('C:\\test');
      expect(invoke).toHaveBeenCalledWith('inspectGamePath', { selectedPath: 'C:\\test' });
    });

    it('confirmGamePath calls invoke with paths', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await confirmGamePath('game/path', 'addon/path', 'exe/path', 'CoA');
      expect(invoke).toHaveBeenCalledWith('confirmGamePath', {
        gamePath: 'game/path',
        addonPath: 'addon/path',
        gameExecutablePath: 'exe/path',
        selectedTarget: 'CoA',
      });
    });

    it('confirmGamePath allows omitting executable path', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await confirmGamePath('game/path', 'addon/path');
      expect(invoke).toHaveBeenCalledWith('confirmGamePath', {
        gamePath: 'game/path',
        addonPath: 'addon/path',
        gameExecutablePath: undefined,
        selectedTarget: undefined,
      });
    });

    it('refreshCatalog calls invoke', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await refreshCatalog();
      expect(invoke).toHaveBeenCalledWith('refreshCatalog', undefined);
    });

    it('installAddon calls invoke with addonId', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await installAddon('test-addon');
      expect(invoke).toHaveBeenCalledWith('installAddon', { addonId: 'test-addon' });
    });

    it('updateAddon calls invoke with addonId', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await updateAddon('test-addon');
      expect(invoke).toHaveBeenCalledWith('updateAddon', { addonId: 'test-addon' });
    });

    it('updateAllAddons calls invoke', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await updateAllAddons();
      expect(invoke).toHaveBeenCalledWith('updateAllAddons', undefined);
    });

    it('uninstallAddon calls invoke with addonId', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await uninstallAddon('test-addon');
      expect(invoke).toHaveBeenCalledWith('uninstallAddon', { addonId: 'test-addon' });
    });

    it('rollbackAddon calls invoke with addonId', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await rollbackAddon('test-addon');
      expect(invoke).toHaveBeenCalledWith('rollbackAddon', { addonId: 'test-addon' });
    });

    it('checkInstallerUpdate calls invoke', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await checkInstallerUpdate();
      expect(invoke).toHaveBeenCalledWith('checkInstallerUpdate', undefined);
    });

    it('openLogsFolder calls invoke', async () => {
      vi.mocked(invoke).mockResolvedValue({ data: {}, error: null });
      await openLogsFolder();
      expect(invoke).toHaveBeenCalledWith('openLogsFolder', undefined);
    });
  });
});
