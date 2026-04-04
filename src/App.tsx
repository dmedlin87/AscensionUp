import { useDeferredValue, useEffect, useMemo, useState } from "react";
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
  refreshCatalog,
  rollbackAddon,
  uninstallAddon,
  updateAddon,
  updateAllAddons,
} from "./app/api";
import type {
  AddonRow,
  AppSnapshot,
  InstallerUpdateStatus,
  OperationResult,
  PathInspection,
} from "./domain/types";
import "./App.css";

type LibraryFilter = "all" | "updates" | "installed" | "issues";
type GameTarget = "Bronzebeard" | "CoA";

const TARGET_OPTIONS: Array<{ value: GameTarget; label: string; description: string }> = [
  {
    value: "Bronzebeard",
    label: "Live / Bronzebeard",
    description: "Use the live client AddOns folder.",
  },
  {
    value: "CoA",
    label: "PTR / CoA",
    description: "Use the PTR AddOns folder for Conquest of Azeroth.",
  },
];

const FILTER_LABELS: Record<LibraryFilter, string> = {
  all: "All Addons",
  updates: "Needs Update",
  installed: "Installed",
  issues: "Issues",
};

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [updateStatus, setUpdateStatus] = useState<InstallerUpdateStatus | null>(null);
  const [inspection, setInspection] = useState<PathInspection | null>(null);
  const [selectedCandidatePath, setSelectedCandidatePath] = useState("");
  const [editingPath, setEditingPath] = useState(false);
  const [selectedTargetChoice, setSelectedTargetChoice] = useState<GameTarget>("Bronzebeard");
  const [loading, setLoading] = useState(true);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [activeFilter, setActiveFilter] = useState<LibraryFilter>("all");
  const [searchQuery, setSearchQuery] = useState("");

  useEffect(() => {
    void loadData();
  }, []);

  const showSetup = editingPath || !snapshot || snapshot.needsSetup;
  const addons = snapshot?.addonRows ?? [];
  const deferredSearch = useDeferredValue(searchQuery.trim().toLowerCase());

  useEffect(() => {
    if (snapshot?.selectedTarget === "Bronzebeard" || snapshot?.selectedTarget === "CoA") {
      setSelectedTargetChoice(snapshot.selectedTarget);
    }
  }, [snapshot?.selectedTarget]);

  useEffect(() => {
    if (!inspection) {
      return;
    }

    const preferredPath = getPreferredAddonPath(inspection, selectedTargetChoice);
    if (!preferredPath) {
      return;
    }

    setSelectedCandidatePath(preferredPath);
  }, [inspection, selectedTargetChoice]);

  const metrics = useMemo(() => {
    const counts: Record<LibraryFilter, number> = {
      all: addons.length,
      updates: 0,
      installed: 0,
      issues: 0,
    };

    for (const addon of addons) {
      if (addon.status === "updateAvailable") counts.updates++;
      if (addon.installedVersion) counts.installed++;
      if (addon.status === "error") counts.issues++;
    }

    return counts;
  }, [addons]);

  const filteredAddons = useMemo(
    () =>
      addons.filter((addon) => {
        if (!matchesFilter(addon, activeFilter)) {
          return false;
        }

        if (!deferredSearch) {
          return true;
        }

        const haystack = [
          addon.displayName,
          addon.description,
          addon.repoAttribution,
          addon.managedFolders.join(" "),
          addon.installedVersion,
          addon.latestVersion,
          addon.releaseNotes,
          addon.errorMessage,
        ]
          .filter(Boolean)
          .join(" ")
          .toLowerCase();

        return haystack.includes(deferredSearch);
      }),
    [activeFilter, addons, deferredSearch],
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
        setSelectedTargetChoice(
          nextSnapshot.selectedTarget === "CoA" ? "CoA" : "Bronzebeard",
        );
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
      setEditingPath(true);
    } catch (error) {
      setErrorMessage(readError(error));
    }
  }

  async function confirmSelection() {
    if (!inspection) {
      setErrorMessage("Inspect a game folder before confirming the install path.");
      return;
    }

    const addonPath = getPreferredAddonPath(inspection, selectedTargetChoice);
    if (!addonPath) {
      setErrorMessage("Select one of the detected addon directories before confirming.");
      return;
    }

    setBusyAction("confirm-path");
    setErrorMessage(null);

    try {
      const nextSnapshot = await confirmGamePath(
        inspection.normalizedGamePath,
        addonPath,
        inspection.gameExecutablePath ?? null,
        selectedTargetChoice,
      );
      setSnapshot(nextSnapshot);
      setInspection(null);
      setEditingPath(false);
      setActionMessage("Saved the current install path.");
    } catch (error) {
      setErrorMessage(readError(error));
    } finally {
      setBusyAction(null);
    }
  }

  async function runAddonOperation(actionKey: string, operation: () => Promise<OperationResult>) {
    setBusyAction(actionKey);
    setErrorMessage(null);
    setActionMessage(null);

    try {
      const result = await operation();
      setSnapshot(result.snapshot);
      setActionMessage(result.notice ?? null);
      setUpdateStatus(await checkInstallerUpdate().catch(() => null));
    } catch (error) {
      setErrorMessage(readError(error));
    } finally {
      setBusyAction(null);
    }
  }

  async function handleRefresh() {
    setBusyAction("refresh");
    setErrorMessage(null);
    setActionMessage(null);

    try {
      const nextSnapshot = await refreshCatalog();
      setSnapshot(nextSnapshot);
      setUpdateStatus(await checkInstallerUpdate().catch(() => null));
    } catch (error) {
      setErrorMessage(readError(error));
    } finally {
      setBusyAction(null);
    }
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
    return (
      <main className="manager-shell loading">
        <div className="loading-screen">
          <div className="spinner" />
          <span>Loading addon manager...</span>
        </div>
      </main>
    );
  }

  return (
    <main className="manager-shell">
      <div className="titlebar">
        <span className="titlebar-label">AscensionUp</span>
      </div>

      <div className="manager-frame">
        <aside className="nav-rail">
          <section className="rail-card brand-card">
            <div className="brand-icon">
              <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            </div>
            <h1>Addon Manager</h1>
            <div className="brand-meta">
              <span className="brand-tag">{snapshot?.selectedTarget ?? "Bronzebeard"}</span>
              <span className="brand-tag">v{snapshot?.installerVersion ?? "1.0.0"}</span>
            </div>
          </section>

          <section className="rail-card">
            <p className="section-label">Library Health</p>
            <div className="rail-stats">
              <StatTile label="Tracked" value={String(metrics.all)} tone="neutral" />
              <StatTile label="Updates" value={String(metrics.updates)} tone="warm" />
              <StatTile label="Installed" value={String(metrics.installed)} tone="good" />
              <StatTile label="Issues" value={String(metrics.issues)} tone="bad" />
            </div>
          </section>

          <section className="rail-card">
            <div className="section-heading">
              <h2>Environment</h2>
              <div className={`catalog-chip ${snapshot?.catalogStatus ?? "unavailable"}`}>
                {snapshot?.catalogStatus ?? "unavailable"}
              </div>
            </div>
            <dl className="path-grid">
              <div className="path-item">
                <dt>Game Folder</dt>
                <dd>{snapshot?.gamePath ?? "Not configured"}</dd>
              </div>
              <div className="path-item">
                <dt>AddOn Folder</dt>
                <dd>{snapshot?.addonPath ?? "Not configured"}</dd>
              </div>
            </dl>
            <div className="rail-actions">
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
          </section>

          <section className="rail-card">
            <p className="section-label">Catalog</p>
            <p className="rail-copy">
              {snapshot?.catalogMessage ??
                "Release metadata and availability are driven from the remote catalog."}
            </p>
            <a href={snapshot?.catalogUrl ?? "#"} target="_blank" rel="noreferrer">
              Open catalog source
            </a>
          </section>

          {showSetup ? (
            <section className="rail-card setup-card">
              <div className="section-heading">
                <h2>Bind Install</h2>
              </div>
              <p className="rail-copy">
                Select the live or PTR profile, then choose the game folder or executable. The AddOns
                path will follow the selected profile automatically.
              </p>
              <div className="target-select" role="radiogroup" aria-label="Game target">
                {TARGET_OPTIONS.map((target) => (
                  <label
                    key={target.value}
                    className={`target-option ${selectedTargetChoice === target.value ? "selected" : ""}`}
                  >
                    <input
                      type="radio"
                      name="game-target"
                      value={target.value}
                      checked={selectedTargetChoice === target.value}
                      onChange={() => setSelectedTargetChoice(target.value)}
                    />
                    <div>
                      <strong>{target.label}</strong>
                      <span>{target.description}</span>
                    </div>
                  </label>
                ))}
              </div>
              <div className="rail-actions">
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

              {inspection ? (
                <div className="setup-result">
                  <div className={`verification-chip ${inspection.verification}`}>
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
                </div>
              ) : (
                <p className="rail-copy muted">
                  No path inspected yet. The manager will auto-select the matching AddOns location
                  once a game folder is inspected.
                </p>
              )}
            </section>
          ) : null}
        </aside>

        <section className="content-stage">
          <header className="hero-card">
            <div className="hero-copy">
              <p className="eyebrow">My Addons</p>
              <h2>
                {showSetup
                  ? "Welcome to AscensionUp"
                  : metrics.updates > 0
                    ? `${metrics.updates} addon${metrics.updates === 1 ? "" : "s"} waiting for update`
                    : "Managed addons are currently in sync"}
              </h2>
              <p>
                {showSetup
                  ? "Bind your game and AddOn folders in the sidebar to begin managing your library."
                  : "Track versions, spot failures, and push updates from a single library without touching unmanaged AddOns."}
              </p>
            </div>
            <span className="hero-refresh">
              Last refresh: {formatDateTime(snapshot?.lastCatalogRefreshAt)}
            </span>
          </header>

          <section className="command-bar">
            <div className="search-block">
              <label htmlFor="addon-search" className="sr-only">Search library</label>
              <svg className="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" />
                <path d="m21 21-4.3-4.3" />
              </svg>
              <input
                id="addon-search"
                type="text"
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder="Search addons..."
              />
            </div>
            <div className="command-actions">
              {updateStatus?.available ? (
                <button
                  type="button"
                  className="ghost"
                  onClick={() =>
                    void openUrl(updateStatus.downloadUrl ?? updateStatus.releasePageUrl)
                  }
                >
                  Installer Update
                </button>
              ) : null}
              <button
                type="button"
                disabled={busyAction !== null || metrics.updates === 0 || showSetup}
                onClick={() => void runAddonOperation("update-all", () => updateAllAddons())}
              >
                {busyAction === "update-all" ? "Updating..." : metrics.updates > 0 ? `Update All (${metrics.updates})` : "Update All"}
              </button>
            </div>
          </section>

          <section className="filter-bar">
            {(Object.keys(FILTER_LABELS) as LibraryFilter[]).map((filterKey) => (
              <button
                key={filterKey}
                type="button"
                className={`filter-chip ${activeFilter === filterKey ? "active" : ""}`}
                onClick={() => setActiveFilter(filterKey)}
              >
                <span>{FILTER_LABELS[filterKey]}</span>
                <strong>{metrics[filterKey]}</strong>
              </button>
            ))}
          </section>

          {errorMessage ? (
            <section className="message-strip error">
              <strong>Action failed</strong>
              <p>{errorMessage}</p>
            </section>
          ) : null}

          {actionMessage ? (
            <section className="message-strip success">
              <strong>Action complete</strong>
              <p>{actionMessage}</p>
            </section>
          ) : null}

          {updateStatus?.available ? (
            <section className="message-strip highlight">
              <strong>{updateStatus.message}</strong>
              <p>
                {updateStatus.latestVersion
                  ? `Version ${updateStatus.latestVersion} published ${formatDate(updateStatus.publishedAt)}.`
                  : "A newer installer release is ready."}
              </p>
              <button
                type="button"
                className="ghost"
                onClick={() => void openUrl(updateStatus.releasePageUrl)}
              >
                View Release
              </button>
            </section>
          ) : null}

          <section className="list-shell">
            <div className="list-header">
              <h2>{FILTER_LABELS[activeFilter]}</h2>
              <span className="list-count">
                {filteredAddons.length} addon{filteredAddons.length !== 1 ? "s" : ""}
                {deferredSearch ? ` matching "${searchQuery}"` : ""}
              </span>
            </div>

            {filteredAddons.length === 0 ? (
              <article className="empty-state">
                <div className="empty-icon">
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
                    <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
                    <line x1="12" y1="22.08" x2="12" y2="12" />
                  </svg>
                </div>
                {showSetup ? (
                  <>
                    <h3>Setup Required</h3>
                    <p>Please complete the setup to begin managing your library.</p>
                    <div className="empty-actions">
                      <button type="button" onClick={() => void choosePath("directory")}>
                        Choose Folder
                      </button>
                      <button type="button" className="ghost" onClick={() => void choosePath("file")}>
                        Choose Executable
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <h3>No addons match this view.</h3>
                    <p>
                      {addons.length === 0
                        ? "Add entries to the remote catalog and refresh the library."
                        : "Adjust the search or filter to bring addons back into view."}
                    </p>
                  </>
                )}
              </article>
            ) : (
              <div className="addon-list">
                {filteredAddons.map((addon) => (
                  <AddonListRow
                    key={addon.addonId}
                    addon={addon}
                    busyAction={busyAction}
                    onInstall={() =>
                      void runAddonOperation(`install-${addon.addonId}`, () =>
                        installAddon(addon.addonId),
                      )
                    }
                    onUpdate={() =>
                      void runAddonOperation(`update-${addon.addonId}`, () =>
                        updateAddon(addon.addonId),
                      )
                    }
                    onRollback={() => {
                      if (
                        !window.confirm(
                          `Rollback ${addon.displayName} to its previously installed version?`,
                        )
                      ) {
                        return;
                      }

                      void runAddonOperation(`rollback-${addon.addonId}`, () =>
                        rollbackAddon(addon.addonId),
                      );
                    }}
                    onUninstall={() => {
                      if (
                        !window.confirm(
                          `Uninstall ${addon.displayName}? This removes only the managed addon folders from your AddOns directory.`,
                        )
                      ) {
                        return;
                      }

                      void runAddonOperation(`uninstall-${addon.addonId}`, () =>
                        uninstallAddon(addon.addonId),
                      );
                    }}
                  />
                ))}
              </div>
            )}
          </section>
        </section>
      </div>
    </main>
  );
}

function StatTile({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "neutral" | "good" | "warm" | "bad";
}) {
  return (
    <article className={`stat-tile ${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

function AddonListRow({
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
  const busyInstall = busyAction === `install-${addon.addonId}`;
  const busyUpdate = busyAction === `update-${addon.addonId}`;
  const busyRollback = busyAction === `rollback-${addon.addonId}`;
  const busyUninstall = busyAction === `uninstall-${addon.addonId}`;

  return (
    <article className={`addon-row ${statusTone(addon.status)}`}>
      <div className="row-identity">
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
        <div className="identity-copy">
          <div className="title-line">
            <h3>{addon.displayName}</h3>
            <span className="state-chip">{describeStatus(addon)}</span>
          </div>
          <p>{addon.description ?? "No description provided in the catalog."}</p>
          <p className="version-line">{statusHeadline(addon)}</p>
          <div className="version-pills" aria-label="Addon versions">
            <span className="version-pill">
              Installed: <strong>{addon.installedVersion ?? "Not installed"}</strong>
            </span>
            <span className="version-pill">
              Latest: <strong>{addon.latestVersion ?? "Unknown"}</strong>
            </span>
          </div>
          <div className="identity-meta">
            <a href={addon.repoUrl} target="_blank" rel="noreferrer">
              {addon.repoAttribution}
            </a>
            <span>Folders: {addon.managedFolders.join(", ")}</span>
            {addon.latestPublishedAt ? <span>Published {formatDate(addon.latestPublishedAt)}</span> : null}
          </div>
          {addon.releaseNotes ? <small className="release-notes">Release notes: {addon.releaseNotes}</small> : null}
          {addon.disabledReason ? <small>{addon.disabledReason}</small> : null}
          {addon.errorMessage ? <small className="error-text">{addon.errorMessage}</small> : null}
        </div>
      </div>

      <div className="row-actions">
        {addon.canInstall || busyInstall ? (
          <button
            type="button"
            className={addon.canUpdate || busyUpdate || addon.installedVersion ? "ghost" : ""}
            disabled={!addon.canInstall || busyAction !== null}
            onClick={onInstall}
          >
            {busyInstall ? "Installing..." : addon.installedVersion ? "Reinstall" : "Install"}
          </button>
        ) : null}
        {addon.canUpdate || busyUpdate ? (
          <button
            type="button"
            className=""
            disabled={!addon.canUpdate || busyAction !== null}
            onClick={onUpdate}
          >
            {busyUpdate ? "Updating..." : "Update"}
          </button>
        ) : null}
        {addon.canRollback || busyRollback ? (
          <button
            type="button"
            className="ghost"
            disabled={!addon.canRollback || busyAction !== null}
            onClick={onRollback}
          >
            {busyRollback ? "Rolling back..." : "Rollback"}
          </button>
        ) : null}
        {addon.canUninstall || busyUninstall ? (
          <button
            type="button"
            className="ghost danger"
            disabled={!addon.canUninstall || busyAction !== null}
            onClick={onUninstall}
          >
            {busyUninstall ? "Uninstalling..." : "Uninstall"}
          </button>
        ) : null}
      </div>
    </article>
  );
}

function getPreferredAddonPath(
  inspection: PathInspection,
  target: GameTarget,
) {
  const preferredPatterns = target === "CoA"
    ? [
        /resources\/ptr\/interface\/addons/i,
        /resources\/ascension_ptr\/interface\/addons/i,
        /(?:^|\/)ptr\/interface\/addons/i,
        /(?:^|\/)ascension_ptr\/interface\/addons/i,
      ]
    : [
        /resources\/client\/interface\/addons/i,
        /(?:^|\/)client\/interface\/addons/i,
      ];

  for (const pattern of preferredPatterns) {
    const match = inspection.candidateAddonPaths.find((candidate) => {
      const haystack = `${candidate.label} ${candidate.path}`.replace(/\\/g, "/");
      return candidate.exists && pattern.test(haystack);
    });

    if (match) {
      return match.path;
    }
  }

  return (
    inspection.proposedAddonPath ??
    inspection.candidateAddonPaths.find((candidate) => candidate.exists)?.path ??
    null
  );
}

function matchesFilter(addon: AddonRow, filter: LibraryFilter) {
  switch (filter) {
    case "updates":
      return addon.status === "updateAvailable";
    case "installed":
      return Boolean(addon.installedVersion);
    case "issues":
      return addon.status === "error";
    default:
      return true;
  }
}

function statusTone(status: AddonRow["status"]) {
  switch (status) {
    case "updateAvailable":
      return "warm";
    case "installed":
      return "good";
    case "error":
      return "bad";
    default:
      return "neutral";
  }
}

function describeStatus(addon: AddonRow) {
  switch (addon.status) {
    case "notInstalled":
      return "Ready to install";
    case "installed":
      return "Current";
    case "updateAvailable":
      return "Update available";
    case "error":
      return "Needs attention";
    default:
      return addon.status;
  }
}

function statusHeadline(addon: AddonRow) {
  switch (addon.status) {
    case "notInstalled":
      return `Version ${addon.latestVersion ?? "unknown"} ready to install`;
    case "installed":
      return `Version ${addon.installedVersion} installed and up to date`;
    case "updateAvailable":
      return `Upgrade ${addon.installedVersion ?? "current"} to ${addon.latestVersion ?? "latest"}`;
    case "error":
      return "Catalog or package validation needs review";
    default:
      return addon.status;
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

function formatDateTime(value?: string | null) {
  if (!value) {
    return "Unknown";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
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
