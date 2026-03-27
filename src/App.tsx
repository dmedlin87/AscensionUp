import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";

import {
  bootstrapApp,
  checkInstallerUpdate,
  CommandError,
  confirmGamePath,
  inspectGamePath,
  installAddon,
  openLogsFolder,
  uninstallAddon,
  updateAddon,
  updateAllAddons,
  rollbackAddon,
} from "./app/api";
import type {
  AddonRow,
  AppSnapshot,
  InstallerUpdateStatus,
  OperationResult,
  PathInspection,
} from "./domain/types";
import "./App.css";

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [updateStatus, setUpdateStatus] = useState<InstallerUpdateStatus | null>(null);
  const [inspection, setInspection] = useState<PathInspection | null>(null);
  const [selectedCandidatePath, setSelectedCandidatePath] = useState("");
  const [editingPath, setEditingPath] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    void loadData();
  }, []);

  const showSetup = editingPath || !snapshot || snapshot.needsSetup;
  const addons = snapshot?.addonRows ?? [];
  const updateAvailableCount = useMemo(
    () => addons.filter((addon) => addon.status === "updateAvailable").length,
    [addons],
  );

  async function loadData() {
    setLoading(true);
    setErrorMessage(null);

    try {
      const nextSnapshot = await bootstrapApp();
      setSnapshot(nextSnapshot);
      setUpdateStatus(await checkInstallerUpdate().catch(() => null));
      if (!nextSnapshot.needsSetup) {
        setInspection(null);
        setEditingPath(false);
      }
    } catch (error) {
      setErrorMessage(readError(error));
    } finally {
      setLoading(false);
    }
  }

  async function choosePath(mode: "directory" | "file") {
    setErrorMessage(null);
    setActionMessage(null);

    try {
      const selection = await open(
        mode === "directory"
          ? { directory: true, multiple: false }
          : {
              directory: false,
              multiple: false,
              filters: [{ name: "Executable", extensions: ["exe"] }],
            },
      );

      if (typeof selection !== "string") {
        return;
      }

      const nextInspection = await inspectGamePath(selection);
      setInspection(nextInspection);
      setSelectedCandidatePath(
        nextInspection.proposedAddonPath ??
          nextInspection.candidateAddonPaths.find((candidate) => candidate.exists)?.path ??
          "",
      );
      setEditingPath(true);
    } catch (error) {
      setErrorMessage(readError(error));
    }
  }

  async function confirmSelection() {
    if (!inspection || !selectedCandidatePath) {
      setErrorMessage("Select one of the detected addon directories before confirming.");
      return;
    }

    setBusyAction("confirm-path");
    setErrorMessage(null);

    try {
      const nextSnapshot = await confirmGamePath(
        inspection.normalizedGamePath,
        selectedCandidatePath,
        inspection.gameExecutablePath ?? null,
      );
      setSnapshot(nextSnapshot);
      setInspection(null);
      setEditingPath(false);
      setActionMessage("Saved the Ascension install path.");
    } catch (error) {
      setErrorMessage(readError(error));
    } finally {
      setBusyAction(null);
    }
  }

  async function runAddonOperation(
    actionKey: string,
    operation: (allowWhileGameRunning: boolean) => Promise<OperationResult>,
  ) {
    setBusyAction(actionKey);
    setErrorMessage(null);
    setActionMessage(null);

    try {
      const result = await tryWithGameWarning(operation);
      setSnapshot(result.snapshot);
      setActionMessage(result.notice ?? null);
      setUpdateStatus(await checkInstallerUpdate().catch(() => null));
    } catch (error) {
      setErrorMessage(readError(error));
    } finally {
      setBusyAction(null);
    }
  }

  async function tryWithGameWarning<T>(
    operation: (allowWhileGameRunning: boolean) => Promise<T>,
  ) {
    try {
      return await operation(false);
    } catch (error) {
      if (
        error instanceof CommandError &&
        error.code === "game_running" &&
        window.confirm(
          "Ascension appears to be running. Continue anyway? You may need to reload or restart the game afterward.",
        )
      ) {
        return operation(true);
      }

      throw error;
    }
  }

  async function handleRefresh() {
    setBusyAction("refresh");
    await loadData();
    setBusyAction(null);
  }

  async function handleOpenLogs() {
    setErrorMessage(null);

    try {
      await openLogsFolder();
    } catch (error) {
      setErrorMessage(readError(error));
    }
  }

  if (loading) {
    return <main className="shell loading">Loading installer state...</main>;
  }

  return (
    <main className="shell">
      <section className="masthead">
        <div>
          <p className="eyebrow">Ascension Addon Installer</p>
          <h1>Bronzebeard-only delivery for private Ascension addons.</h1>
          <p className="lede">
            Install, update, and roll back only the addon folders this app manages.
            Everything else stays untouched.
          </p>
        </div>
        <div className="masthead-card">
          <span className="meta-label">Installer</span>
          <strong>{snapshot?.installerVersion ?? "1.0.0"}</strong>
          <span className="meta-label">Target</span>
          <strong>{snapshot?.selectedTarget ?? "Bronzebeard"}</strong>
          <span className="meta-label">Catalog</span>
          <strong className={`catalog-state ${snapshot?.catalogStatus ?? "unavailable"}`}>
            {snapshot?.catalogStatus ?? "unavailable"}
          </strong>
        </div>
      </section>

      {updateStatus?.available ? (
        <section className="banner banner-highlight">
          <div>
            <strong>{updateStatus.message}</strong>
            <p>
              {updateStatus.latestVersion
                ? `Version ${updateStatus.latestVersion} was published ${formatDate(updateStatus.publishedAt)}.`
                : "A newer installer release is ready."}
            </p>
          </div>
          <div className="banner-actions">
            <button
              type="button"
              onClick={() => void openUrl(updateStatus.downloadUrl ?? updateStatus.releasePageUrl)}
            >
              Download Update
            </button>
            <button
              type="button"
              className="ghost"
              onClick={() => void openUrl(updateStatus.releasePageUrl)}
            >
              View Release
            </button>
          </div>
        </section>
      ) : null}

      {snapshot?.catalogMessage ? (
        <section className="banner">
          <strong>Catalog status</strong>
          <p>{snapshot.catalogMessage}</p>
        </section>
      ) : null}

      {snapshot?.gameRunning ? (
        <section className="banner banner-warning">
          <strong>Ascension looks open.</strong>
          <p>
            Installs and updates are still allowed, but locked files can fail until the
            game closes.
          </p>
        </section>
      ) : null}

      {errorMessage ? (
        <section className="banner banner-error">
          <strong>Action failed</strong>
          <p>{errorMessage}</p>
        </section>
      ) : null}

      {actionMessage ? (
        <section className="banner banner-success">
          <strong>Done</strong>
          <p>{actionMessage}</p>
        </section>
      ) : null}

      <section className="summary-grid">
        <article className="summary-card">
          <span className="meta-label">Ascension install</span>
          <strong>{snapshot?.gamePath ?? "Not configured"}</strong>
          <small>{snapshot?.pathMessage ?? "Choose the game folder or executable."}</small>
        </article>
        <article className="summary-card">
          <span className="meta-label">Addon directory</span>
          <strong>{snapshot?.addonPath ?? "Not configured"}</strong>
          <small>Only managed addon folders are written here.</small>
        </article>
        <article className="summary-card">
          <span className="meta-label">Update queue</span>
          <strong>{updateAvailableCount}</strong>
          <small>
            {updateAvailableCount === 1
              ? "One addon is ready to update."
              : `${updateAvailableCount} addons are ready to update.`}
          </small>
        </article>
      </section>

      {showSetup ? (
        <section className="setup-panel">
          <div className="setup-copy">
            <p className="eyebrow">First Run</p>
            <h2>Point the installer at the correct Ascension client.</h2>
            <p>
              Choose either the Ascension folder or the Ascension executable. The app
              only checks the documented Bronzebeard addon paths and will ask you to
              confirm the correct one if there is any ambiguity.
            </p>
            <div className="setup-actions">
              <button type="button" onClick={() => void choosePath("directory")}>
                Choose Folder
              </button>
              <button type="button" className="ghost" onClick={() => void choosePath("file")}>
                Choose Executable
              </button>
              {snapshot && !snapshot.needsSetup ? (
                <button
                  type="button"
                  className="ghost"
                  onClick={() => {
                    setInspection(null);
                    setEditingPath(false);
                  }}
                >
                  Cancel
                </button>
              ) : null}
            </div>
          </div>
          <div className="setup-result">
            {inspection ? (
              <>
                <div className={`pill ${inspection.verification}`}>
                  {inspection.verification}
                </div>
                <h3>{inspection.message}</h3>
                <p className="path-line">{inspection.normalizedGamePath}</p>
                {inspection.ascensionHints.length > 0 ? (
                  <ul className="hint-list">
                    {inspection.ascensionHints.map((hint) => (
                      <li key={hint}>{hint}</li>
                    ))}
                  </ul>
                ) : null}
                <div className="candidate-list">
                  {inspection.candidateAddonPaths.map((candidate) => (
                    <label
                      key={candidate.path}
                      className={`candidate ${candidate.exists ? "active" : "inactive"}`}
                    >
                      <input
                        type="radio"
                        name="addon-path"
                        value={candidate.path}
                        checked={selectedCandidatePath === candidate.path}
                        onChange={() => setSelectedCandidatePath(candidate.path)}
                        disabled={!candidate.exists}
                      />
                      <div>
                        <strong>{candidate.label}</strong>
                        <span>{candidate.path}</span>
                      </div>
                    </label>
                  ))}
                </div>
                <button
                  type="button"
                  disabled={!selectedCandidatePath || busyAction === "confirm-path"}
                  onClick={() => void confirmSelection()}
                >
                  {busyAction === "confirm-path" ? "Saving..." : "Confirm Path"}
                </button>
              </>
            ) : (
              <div className="placeholder-card">
                <h3>No install selected yet.</h3>
                <p>
                  The app will propose one addon directory when it finds exactly one valid
                  candidate. If it finds more than one, you will pick the one to manage.
                </p>
              </div>
            )}
          </div>
        </section>
      ) : null}

      <section className="toolbar">
        <div className="toolbar-left">
          <button
            type="button"
            disabled={busyAction !== null}
            onClick={() => void runAddonOperation("update-all", updateAllAddons)}
          >
            {busyAction === "update-all" ? "Updating..." : "Update All"}
          </button>
          <button type="button" className="ghost" onClick={() => setEditingPath(true)}>
            Change Game Folder
          </button>
          <button
            type="button"
            className="ghost"
            disabled={busyAction === "refresh"}
            onClick={() => void handleRefresh()}
          >
            {busyAction === "refresh" ? "Refreshing..." : "Refresh"}
          </button>
          <button type="button" className="ghost" onClick={() => void handleOpenLogs()}>
            Open Logs
          </button>
        </div>
        <div className="toolbar-right">
          <a href={snapshot?.catalogUrl ?? "#"} target="_blank" rel="noreferrer">
            View Catalog
          </a>
        </div>
      </section>

      <section className="addon-grid">
        {addons.length === 0 ? (
          <article className="placeholder-card">
            <h3>No catalog entries are available yet.</h3>
            <p>
              Add addons to the remote <code>catalog.json</code> and refresh the app to
              make them installable.
            </p>
          </article>
        ) : (
          addons.map((addon) => (
            <AddonCard
              key={addon.addonId}
              addon={addon}
              busyAction={busyAction}
              onInstall={() =>
                void runAddonOperation(`install-${addon.addonId}`, (allow) =>
                  installAddon(addon.addonId, allow),
                )
              }
              onUpdate={() =>
                void runAddonOperation(`update-${addon.addonId}`, (allow) =>
                  updateAddon(addon.addonId, allow),
                )
              }
              onRollback={() =>
                void runAddonOperation(`rollback-${addon.addonId}`, (allow) =>
                  rollbackAddon(addon.addonId, allow),
                )
              }
              onUninstall={() => {
                if (
                  !window.confirm(
                    `Uninstall ${addon.displayName}? This removes only the managed addon folders from your AddOns directory.`,
                  )
                ) {
                  return;
                }
                void runAddonOperation(`uninstall-${addon.addonId}`, (allow) =>
                  uninstallAddon(addon.addonId, allow),
                );
              }}
            />
          ))
        )}
      </section>
    </main>
  );
}

function AddonCard({
  addon,
  busyAction,
  onInstall,
  onUpdate,
  onUninstall,
  onRollback,
}: {
  addon: AddonRow;
  busyAction: string | null;
  onInstall: () => void;
  onUpdate: () => void;
  onUninstall: () => void;
  onRollback: () => void;
}) {
  return (
    <article className="addon-card">
      <div className="addon-header">
        <div className="addon-identity">
          <div className="icon-shell">
            {addon.iconUrl ? (
              <img src={addon.iconUrl} alt="" />
            ) : (
              addon.displayName
                .split(" ")
                .map((part) => part[0])
                .join("")
                .slice(0, 2)
            )}
          </div>
          <div>
            <h3>{addon.displayName}</h3>
            <p>{addon.description ?? "No description provided in the catalog."}</p>
          </div>
        </div>
        <span className={`status-badge ${addon.status}`}>{readableStatus(addon.status)}</span>
      </div>

      <dl className="addon-meta">
        <div>
          <dt>Installed</dt>
          <dd>{addon.installedVersion ?? "Not installed"}</dd>
        </div>
        <div>
          <dt>Latest</dt>
          <dd>{addon.latestVersion ?? "Unknown"}</dd>
        </div>
        <div>
          <dt>Published</dt>
          <dd>{formatDate(addon.latestPublishedAt)}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>
            <a href={addon.repoUrl} target="_blank" rel="noreferrer">
              {addon.repoAttribution}
            </a>
          </dd>
        </div>
      </dl>

      <p className="folders-line">Managed folders: {addon.managedFolders.join(", ")}</p>

      {addon.errorMessage ? <p className="row-error">{addon.errorMessage}</p> : null}
      {addon.disabledReason ? <p className="row-note">{addon.disabledReason}</p> : null}
      {addon.releaseNotes ? <p className="row-note">{addon.releaseNotes}</p> : null}

      <div className="card-actions">
        <button
          type="button"
          disabled={!addon.canInstall || busyAction !== null}
          onClick={onInstall}
        >
          {busyAction === `install-${addon.addonId}`
            ? "Installing..."
            : addon.installedVersion
              ? "Reinstall"
              : "Install"}
        </button>
        <button
          type="button"
          className="ghost"
          disabled={!addon.canUpdate || busyAction !== null}
          onClick={onUpdate}
        >
          {busyAction === `update-${addon.addonId}` ? "Updating..." : "Update"}
        </button>
        <button
          type="button"
          className="ghost"
          disabled={!addon.canUninstall || busyAction !== null}
          onClick={onUninstall}
        >
          {busyAction === `uninstall-${addon.addonId}` ? "Uninstalling..." : "Uninstall"}
        </button>
        <button
          type="button"
          className="ghost"
          disabled={!addon.canRollback || busyAction !== null}
          onClick={onRollback}
        >
          {busyAction === `rollback-${addon.addonId}` ? "Rolling back..." : "Rollback"}
        </button>
      </div>
    </article>
  );
}

function readableStatus(status: AddonRow["status"]) {
  switch (status) {
    case "notInstalled":
      return "Not Installed";
    case "installed":
      return "Installed";
    case "updateAvailable":
      return "Update Available";
    case "error":
      return "Error";
    default:
      return status;
  }
}

function formatDate(value?: string | null) {
  if (!value) {
    return "Unknown";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function readError(error: unknown) {
  if (error instanceof CommandError) {
    return error.message;
  }

  if (error instanceof Error) {
    if (
      error.message.includes("Cannot read properties of undefined") &&
      error.message.includes("invoke")
    ) {
      return "Tauri APIs are unavailable in this window. Launch the app with `npm run tauri dev` to use file dialogs and native commands.";
    }

    return error.message;
  }

  return "An unexpected error occurred.";
}

export default App;
