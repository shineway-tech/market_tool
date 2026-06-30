import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import type { CopyText } from "../i18n/copy";
import type { UpdateState } from "../domain/types";
import { AUTO_UPDATE_KEY } from "../config/app";
import { normalizeUpdateError } from "../utils/errors";
import { readStoredBoolean } from "../utils/storage";

export interface UpdateControllerOptions {
  getText: () => CopyText;
  getFallbackError: () => string;
  canAutoCheck: () => boolean;
  render: () => void;
  showToast: (message: string) => void;
}

export class UpdateController {
  autoEnabled = readStoredBoolean(AUTO_UPDATE_KEY, true);
  autoChecked = false;
  expandedVersions = new Set<string>();
  state: UpdateState = {
    status: "idle",
    downloadedBytes: 0,
  };

  private pendingUpdate: Update | null = null;

  constructor(private readonly options: UpdateControllerOptions) {}

  get canInstall() {
    return this.state.status === "available" && Boolean(this.pendingUpdate);
  }

  get isChecking() {
    return this.state.status === "checking";
  }

  get isDownloading() {
    return this.state.status === "downloading";
  }

  statusText() {
    const text = this.options.getText();
    if (this.state.status === "checking") return text.checkingUpdate;
    if (this.state.status === "latest") return text.latestVersion;
    if (this.state.status === "available") {
      return this.state.availableVersion
        ? `${text.updateAvailable} v${this.state.availableVersion}`
        : text.updateAvailable;
    }
    if (this.state.status === "downloading") return `${text.downloadingUpdate} ${this.progressPercent()}%`;
    if (this.state.status === "installed") return text.updateInstalled;
    if (this.state.status === "error") return this.state.error || text.updateFailed;
    return this.autoEnabled ? text.autoUpdateOn : text.autoUpdateOff;
  }

  progressPercent() {
    if (!this.state.contentLength) return 0;
    return Math.min(100, Math.round((this.state.downloadedBytes / this.state.contentLength) * 100));
  }

  setAutoEnabled(enabled: boolean) {
    this.autoEnabled = enabled;
    this.autoChecked = false;
    localStorage.setItem(AUTO_UPDATE_KEY, String(enabled));
    const text = this.options.getText();
    this.options.render();
    this.options.showToast(enabled ? text.autoUpdateOn : text.autoUpdateOff);
  }

  toggleRelease(version: string) {
    if (this.expandedVersions.has(version)) {
      this.expandedVersions.delete(version);
    } else {
      this.expandedVersions.add(version);
    }
    this.options.render();
  }

  scheduleAutoCheck() {
    if (!this.options.canAutoCheck() || !this.autoEnabled || this.autoChecked) return;

    this.autoChecked = true;
    window.setTimeout(() => {
      void this.check({ silent: true });
    }, 900);
  }

  async check(options: { silent?: boolean } = {}) {
    if (this.state.status === "checking" || this.state.status === "downloading") return;

    this.pendingUpdate = null;
    this.state = {
      status: "checking",
      downloadedBytes: 0,
    };
    this.options.render();

    try {
      const update = await check({ timeout: 15000 });
      const text = this.options.getText();

      if (!update) {
        this.state = {
          status: "latest",
          downloadedBytes: 0,
        };
        this.options.render();
        if (!options.silent) this.options.showToast(text.latestVersion);
        return;
      }

      this.pendingUpdate = update;
      this.state = {
        status: "available",
        availableVersion: update.version,
        notes: update.body,
        downloadedBytes: 0,
      };
      this.options.render();
      if (!options.silent) this.options.showToast(this.statusText());
    } catch (error) {
      const text = this.options.getText();
      this.state = {
        status: "error",
        downloadedBytes: 0,
        error: this.normalizeUpdateError(error),
      };
      this.options.render();
      if (!options.silent) this.options.showToast(this.state.error || text.updateFailed);
    }
  }

  async installPending() {
    if (this.state.status === "downloading") return;
    if (!this.pendingUpdate) {
      await this.check({ silent: false });
      if (!this.pendingUpdate) return;
    }

    this.state = {
      ...this.state,
      status: "downloading",
      downloadedBytes: 0,
      contentLength: undefined,
    };
    this.options.render();

    try {
      let downloadedBytes = 0;
      let contentLength: number | undefined;
      await this.pendingUpdate.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") {
          contentLength = event.data.contentLength;
          downloadedBytes = 0;
        }

        if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
        }

        if (event.event === "Finished") {
          downloadedBytes = contentLength || downloadedBytes;
        }

        this.state = {
          ...this.state,
          status: "downloading",
          downloadedBytes,
          contentLength,
        };
        this.options.render();
      });

      this.pendingUpdate = null;
      this.state = {
        status: "installed",
        downloadedBytes: contentLength || downloadedBytes,
        contentLength,
      };
      this.options.render();
      this.options.showToast(this.options.getText().updateInstalled);
      await relaunch();
    } catch (error) {
      const text = this.options.getText();
      this.state = {
        status: "error",
        downloadedBytes: 0,
        error: this.normalizeUpdateError(error),
      };
      this.options.render();
      this.options.showToast(this.state.error || text.updateFailed);
    }
  }

  private normalizeUpdateError(error: unknown) {
    return normalizeUpdateError(
      error,
      this.options.getFallbackError(),
      this.options.getText().updateUnavailable,
    );
  }
}
