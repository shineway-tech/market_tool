import type { UpdateState } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { icon } from "../ui/icons";

export interface ReleaseHistorySection {
  icon: string;
  title: string;
  items: string[];
}

export interface ReleaseHistoryEntry {
  version: string;
  date: string;
  icon: string;
  sections: ReleaseHistorySection[];
}

export interface ReleasesPageState {
  text: CopyText;
  entries: ReleaseHistoryEntry[];
  updateState: UpdateState;
  autoUpdateEnabled: boolean;
  canInstall: boolean;
  isChecking: boolean;
  isDownloading: boolean;
  status: string;
  progress: number;
  expandedReleaseVersions: Set<string>;
}

export function renderReleasesPage({
  text,
  entries,
  updateState,
  autoUpdateEnabled,
  canInstall,
  isChecking,
  isDownloading,
  status,
  progress,
  expandedReleaseVersions,
}: ReleasesPageState) {
  return `
    <section class="page-head">
      <div>
        <h1>${text.releasesTitle}</h1>
      </div>
    </section>
    <section class="release-page">
      <div class="release-controls">
        <div class="release-control-bar">
          <div class="release-control-main">
            <label class="switch-row">
              <input type="checkbox" data-auto-update ${autoUpdateEnabled ? "checked" : ""} />
              <span>${text.autoUpdate}</span>
              <em>${autoUpdateEnabled ? text.autoUpdateOn : text.autoUpdateOff}</em>
            </label>
            <span class="release-status status-${updateState.status}">${status}</span>
          </div>
          <div class="release-actions">
            <button class="ghost-btn" type="button" data-action="check-update" ${isChecking || isDownloading ? "disabled" : ""}>
              ${icon("refresh")}${isChecking ? text.checkingUpdate : text.checkUpdate}
            </button>
            <button class="primary-btn" type="button" data-action="install-update" ${canInstall || isDownloading ? "" : "disabled"}>
              ${icon("download")}${isDownloading ? text.downloadingUpdate : text.installUpdate}
            </button>
          </div>
        </div>

        ${
          isDownloading
            ? `<div class="release-progress" aria-label="${text.updateProgress}">
                <span style="width:${progress}%"></span>
              </div>`
            : ""
        }
      </div>

      ${entries.map((entry) => releaseHistoryCard(entry, text, expandedReleaseVersions)).join("")}
    </section>
  `;
}

function releaseHistoryCard(entry: ReleaseHistoryEntry, text: CopyText, expandedReleaseVersions: Set<string>) {
  const isExpanded = expandedReleaseVersions.has(entry.version);
  return `
    <article class="release-card">
      <div class="release-card-head">
        <div class="release-title-block">
          <div class="release-version-icon">${icon(entry.icon)}</div>
          <div>
            <h2>v${entry.version}</h2>
            <span>${text.releaseDate} ${entry.date}</span>
          </div>
        </div>
        <button class="release-toggle ${isExpanded ? "is-open" : ""}" type="button" data-action="toggle-release-content" data-release-version="${entry.version}" aria-expanded="${isExpanded ? "true" : "false"}">
          ${icon("chevron")}
          <span>${isExpanded ? text.hideReleaseContent : text.showReleaseContent}</span>
        </button>
      </div>

      ${
        isExpanded
          ? `<div class="release-list">
              ${entry.sections
                .map(
                  (section) => `
                    <div class="release-item">
                      <div class="release-icon">${icon(section.icon)}</div>
                      <div>
                        <h3>${section.title}</h3>
                        <ul>
                          ${section.items.map((item) => `<li>${item}</li>`).join("")}
                        </ul>
                      </div>
                    </div>
                  `,
                )
                .join("")}
            </div>`
          : ""
      }
    </article>
  `;
}
