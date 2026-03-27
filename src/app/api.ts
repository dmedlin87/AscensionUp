import { invoke } from "@tauri-apps/api/core";

import type {
  AppSnapshot,
  CommandEnvelope,
  InstallerUpdateStatus,
  OperationResult,
  PathInspection,
} from "../domain/types";

export class CommandError extends Error {
  code: string;
  details?: string | null;

  constructor(code: string, message: string, details?: string | null) {
    super(message);
    this.code = code;
    this.details = details;
  }
}

async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const envelope = await invoke<CommandEnvelope<T>>(command, args);
  if (envelope.error) {
    throw new CommandError(
      envelope.error.code,
      envelope.error.message,
      envelope.error.details,
    );
  }

  if (envelope.data === undefined || envelope.data === null) {
    throw new CommandError(
      "empty_response",
      `The command "${command}" returned no data.`,
    );
  }

  return envelope.data;
}

export function bootstrapApp() {
  return invokeCommand<AppSnapshot>("bootstrapApp");
}

export function inspectGamePath(selectedPath: string) {
  return invokeCommand<PathInspection>("inspectGamePath", { selectedPath });
}

export function confirmGamePath(
  gamePath: string,
  addonPath: string,
  gameExecutablePath?: string | null,
) {
  return invokeCommand<AppSnapshot>("confirmGamePath", {
    gamePath,
    addonPath,
    gameExecutablePath,
  });
}

export function refreshCatalog() {
  return invokeCommand<AppSnapshot>("refreshCatalog");
}

export function installAddon(addonId: string) {
  return invokeCommand<OperationResult>("installAddon", {
    addonId,
  });
}

export function updateAddon(addonId: string) {
  return invokeCommand<OperationResult>("updateAddon", {
    addonId,
  });
}

export function updateAllAddons() {
  return invokeCommand<OperationResult>("updateAllAddons");
}

export function uninstallAddon(addonId: string) {
  return invokeCommand<OperationResult>("uninstallAddon", {
    addonId,
  });
}

export function rollbackAddon(addonId: string) {
  return invokeCommand<OperationResult>("rollbackAddon", {
    addonId,
  });
}

export function checkInstallerUpdate() {
  return invokeCommand<InstallerUpdateStatus>("checkInstallerUpdate");
}

export function openLogsFolder() {
  return invokeCommand<boolean>("openLogsFolder");
}
