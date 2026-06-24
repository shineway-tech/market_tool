import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import "./styles.css";
import type {
  AuthSession,
  AuthSettings,
  AuthTaskStatus,
  AuthUser,
  AuthViewMode,
  Bootstrap,
  CaptchaResponse,
  ChannelAccount,
  LanguageMode,
  LoginTarget,
  MenuId,
  PlatformAuthSettings,
  PlatformInfo,
  SidebarMenuId,
  StartLoginResponse,
  ThemeMode,
  UpdateState,
} from "./domain/types";
import { fallbackPlatforms } from "./domain/platforms";
import { defaultPlatformSettings } from "./domain/auth-settings";
import { isQrAuth } from "./domain/auth-task";
import { releaseHistoryForLanguage } from "./domain/releases";
import { copy } from "./i18n/copy";
import { icon } from "./ui/icons";
import { escapeAttribute, escapeHtml } from "./utils/html";
import { readStoredBoolean, readStoredMode } from "./utils/storage";
import {
  accountCountLabel as formatAccountCountLabel,
  formatFollowersTotal as formatAccountFollowersTotal,
  initials as accountInitials,
} from "./utils/format";
import {
  accountFollowersText,
  accountSyncText,
  renderAccountItem,
  renderAuthDialog,
  renderEmptyAccounts,
  renderPlatformItem,
} from "./ui/channel-components";
import { renderNavItem } from "./ui/navigation";
import { renderAccountDropdown } from "./ui/user-menu";
import { renderAuthPage } from "./pages/auth";
import { renderChannelsPage } from "./pages/channels";
import { renderFeedbackPage } from "./pages/feedback";
import { renderPasswordPage } from "./pages/password";
import { renderProfilePage } from "./pages/profile";
import { renderReleasesPage } from "./pages/releases";
import { renderSettingsPage } from "./pages/settings";
import {
  API_BASE_URL,
  AUTH_TOKEN_KEY,
  AUTO_UPDATE_KEY,
  INPUT_HINTS_OFF,
} from "./config/app";
import { requestApi, type ApiRequestOptions } from "./services/api";
import { invokeCommand } from "./services/tauri-commands";

let platforms: PlatformInfo[] = fallbackPlatforms;
let accounts: ChannelAccount[] = [];
let settings: AuthSettings = {
  relay: {
    enabled: true,
    serverUrl: "https://aitoearn.cn/api",
    apiKey: "",
  },
  platforms: defaultPlatformSettings(),
};
let selectedPlatformId = "xiaohongshu";
let activeMenuId: MenuId = "channels";
let sidebarCollapsed = false;
let userMenuOpen = false;
let activeAuthTask: StartLoginResponse | null = null;
let activeAuthMessage = "";
let toastTimer: number | undefined;
let authPollTimer: number | undefined;
let refreshingAccountIds = new Set<string>();
let refreshingPlatformIds = new Set<string>();
let language: LanguageMode = readStoredMode("marketing-master-language", "zh", ["zh", "en"]);
let theme: ThemeMode = readStoredMode("marketing-master-theme", "dark", ["dark", "light"]);
let autoUpdateEnabled = readStoredBoolean(AUTO_UPDATE_KEY, true);
let autoUpdateChecked = false;
let expandedReleaseVersions = new Set<string>();
let pendingUpdate: Update | null = null;
let updateState: UpdateState = {
  status: "idle",
  downloadedBytes: 0,
};
let authToken = localStorage.getItem(AUTH_TOKEN_KEY) || "";
let currentUser: AuthUser | null = null;
let authViewMode: AuthViewMode = "login";
let captcha: CaptchaResponse | null = null;
let authBusy = false;
let profileBusy = false;
let passwordBusy = false;
let authDraft = {
  account: "",
  password: "",
  nickname: "",
  captchaCode: "",
};
let profileDraft = {
  nickname: "",
};
let passwordDraft = {
  currentPassword: "",
  newPassword: "",
  confirmPassword: "",
};
let feedbackBusy = false;
let feedbackDraft = {
  content: "",
  contact: "",
};

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root missing");
}

const appRoot = app;

void boot();

async function boot() {
  if (authToken) {
    try {
      currentUser = await apiRequest<AuthUser>("/v1/auth/me");
      profileDraft.nickname = currentUser.nickname;
      await loadClientData();
      render();
      return;
    } catch (error) {
      authToken = "";
      localStorage.removeItem(AUTH_TOKEN_KEY);
    }
  }
  await loadCaptcha();
  render();
}

async function loadClientData() {
  try {
    const bootstrap = await invokeCommand<Bootstrap>("get_bootstrap", {
      userId: requireCurrentUserId(),
    });
    platforms = bootstrap.platforms;
    accounts = bootstrap.accounts;
    settings = bootstrap.settings;
    await mirrorAccountsToBackend(accounts);
  } catch (error) {
    console.warn("Using browser fallback because Tauri is not available", error);
  }
}

function requireCurrentUserId() {
  if (!currentUser?.id) {
    throw new Error(copy[language].loginRequired);
  }
  return currentUser.id;
}

async function apiRequest<T>(path: string, options: ApiRequestOptions = {}): Promise<T> {
  return requestApi<T>(API_BASE_URL, path, {
    ...options,
    token: authToken,
    onUnauthorized: () => {
      authToken = "";
      currentUser = null;
      localStorage.removeItem(AUTH_TOKEN_KEY);
    },
  });
}

async function mirrorAccountsToBackend(items: ChannelAccount[]) {
  if (!authToken || !items.length) return;

  await Promise.allSettled(items.map((account) => apiRequest<ChannelAccount>("/v1/channel/accounts", {
    method: "POST",
    body: backendAccountPayload(account),
  })));
}

function backendAccountPayload(account: ChannelAccount) {
  return {
    platform_id: account.platformId,
    platform_uid: account.uid,
    nickname: account.nickname,
    avatar: account.avatar,
    followers: account.followers ?? null,
    status: account.status,
    relay_account_ref: account.relayAccountRef || "",
    homepage_url: "",
  };
}

function render() {
  const text = copy[language];

  if (!currentUser) {
    appRoot.innerHTML = authPage();
    bindAuthEvents();
    return;
  }

  appRoot.innerHTML = `
    <div class="window theme-${theme} ${sidebarCollapsed ? "is-collapsed" : ""}">
      <header class="appbar">
        <div class="brand">
          <div class="brand-mark" aria-hidden="true">M</div>
          <div class="brand-text">${text.appName}</div>
        </div>
        <button class="collapse" type="button" data-action="toggle-sidebar" title="${sidebarCollapsed ? "展开菜单" : "收起菜单"}">
          ${icon("chevron")}
        </button>
        <label class="search">
          ${icon("search")}
          <input type="search" ${INPUT_HINTS_OFF} placeholder="${text.search}" />
        </label>
        <div class="top-actions">
          <button class="icon-btn" type="button" data-menu="settings" title="${text.settingsTitle}">${icon("settings")}</button>
          <div class="user-menu-wrap">
            <button class="avatar-btn" type="button" data-action="toggle-user-menu" title="${escapeAttribute(currentUser.nickname)}" aria-expanded="${userMenuOpen ? "true" : "false"}">
              ${escapeHtml(currentUser.nickname.slice(0, 1) || "营")}
            </button>
            ${userMenuOpen ? accountDropdown() : ""}
          </div>
        </div>
      </header>

      <aside class="sidebar">
        <nav class="nav-group" aria-label="主菜单">
          ${navItem("channels", "layers")}
        </nav>
        <nav class="nav-group nav-bottom" aria-label="系统菜单">
          ${navItem("settings", "settings")}
          ${navItem("releases", "spark")}
          ${navItem("feedback", "message")}
        </nav>
      </aside>

      <main class="main">
        ${renderMainContent()}
      </main>

      ${activeAuthTask ? authDialog(activeAuthTask) : ""}
      <div class="toast" hidden></div>
    </div>
  `;

  bindEvents();
  scheduleAutoUpdateCheck();
}

function authPage() {
  return renderAuthPage({
    text: copy[language],
    theme,
    authViewMode,
    authDraft,
    captcha,
    authBusy,
    inputHints: INPUT_HINTS_OFF,
  });
}

function accountDropdown() {
  return renderAccountDropdown({
    text: copy[language],
    user: currentUser,
  });
}

function renderMainContent() {
  if (activeMenuId === "channels") return channelPage();
  if (activeMenuId === "settings") return settingsPage();
  if (activeMenuId === "profile") return profilePage();
  if (activeMenuId === "password") return passwordPage();
  if (activeMenuId === "feedback") return feedbackPage();
  if (activeMenuId === "releases") return releasesPage();
  return channelPage();
}

function channelPage() {
  const selectedPlatform = getSelectedPlatform();
  const selectedAccounts = accounts.filter((item) => item.platformId === selectedPlatform.id);
  const connectedCount = accounts.length;
  const activeAccountCount = selectedAccounts.filter((item) => item.status === "active").length;
  const platformRefreshing = refreshingPlatformIds.has(selectedPlatform.id);

  return renderChannelsPage({
    text: copy[language],
    selectedPlatform,
    selectedAccounts,
    connectedCount,
    activeAccountCount,
    platformRefreshing,
    platforms,
    platformItem,
    accountItem,
    emptyAccounts,
    formatFollowersTotal: (items) => formatAccountFollowersTotal(items, language, copy[language]),
  });
}

function settingsPage() {
  return renderSettingsPage({
    text: copy[language],
    language,
    theme,
  });
}

function profilePage() {
  const profileNickname = profileDraft.nickname || currentUser?.nickname || "";

  return renderProfilePage({
    text: copy[language],
    currentUser,
    profileNickname,
    profileBusy,
    inputHints: INPUT_HINTS_OFF,
  });
}

function passwordPage() {
  return renderPasswordPage({
    text: copy[language],
    passwordDraft,
    passwordBusy,
    inputHints: INPUT_HINTS_OFF,
  });
}

function feedbackPage() {
  return renderFeedbackPage({
    text: copy[language],
    feedbackDraft,
    feedbackBusy,
    inputHints: INPUT_HINTS_OFF,
  });
}

function releasesPage() {
  const entries = releaseHistoryForLanguage(language);
  const status = updateStatusText();
  const progress = updateProgressPercent();
  const isChecking = updateState.status === "checking";
  const isDownloading = updateState.status === "downloading";
  const canInstall = updateState.status === "available" && Boolean(pendingUpdate);

  return renderReleasesPage({
    text: copy[language],
    entries,
    updateState,
    autoUpdateEnabled,
    canInstall,
    isChecking,
    isDownloading,
    status,
    progress,
    expandedReleaseVersions,
  });
}

function updateStatusText() {
  const text = copy[language];
  if (updateState.status === "checking") return text.checkingUpdate;
  if (updateState.status === "latest") return text.latestVersion;
  if (updateState.status === "available") {
    return updateState.availableVersion
      ? `${text.updateAvailable} v${updateState.availableVersion}`
      : text.updateAvailable;
  }
  if (updateState.status === "downloading") return `${text.downloadingUpdate} ${updateProgressPercent()}%`;
  if (updateState.status === "installed") return text.updateInstalled;
  if (updateState.status === "error") return updateState.error || text.updateFailed;
  return autoUpdateEnabled ? text.autoUpdateOn : text.autoUpdateOff;
}

function updateProgressPercent() {
  if (!updateState.contentLength) return 0;
  return Math.min(100, Math.round((updateState.downloadedBytes / updateState.contentLength) * 100));
}

function scheduleAutoUpdateCheck() {
  if (!currentUser || !autoUpdateEnabled || autoUpdateChecked) return;

  autoUpdateChecked = true;
  window.setTimeout(() => {
    void checkForUpdates({ silent: true });
  }, 900);
}

async function checkForUpdates(options: { silent?: boolean } = {}) {
  if (updateState.status === "checking" || updateState.status === "downloading") return;

  pendingUpdate = null;
  updateState = {
    status: "checking",
    downloadedBytes: 0,
  };
  render();

  try {
    const update = await check({ timeout: 15000 });

    if (!update) {
      updateState = {
        status: "latest",
        downloadedBytes: 0,
      };
      if (!options.silent) showToast(copy[language].latestVersion);
      render();
      return;
    }

    pendingUpdate = update;
    updateState = {
      status: "available",
      availableVersion: update.version,
      notes: update.body,
      downloadedBytes: 0,
    };
    if (!options.silent) showToast(updateStatusText());
    render();
  } catch (error) {
    updateState = {
      status: "error",
      downloadedBytes: 0,
      error: normalizeUpdateError(error),
    };
    if (!options.silent) showToast(updateState.error || copy[language].updateFailed);
    render();
  }
}

async function installPendingUpdate() {
  if (updateState.status === "downloading") return;
  if (!pendingUpdate) {
    await checkForUpdates({ silent: false });
    if (!pendingUpdate) return;
  }

  updateState = {
    ...updateState,
    status: "downloading",
    downloadedBytes: 0,
    contentLength: undefined,
  };
  render();

  try {
    let downloadedBytes = 0;
    let contentLength: number | undefined;
    await pendingUpdate.downloadAndInstall((event: DownloadEvent) => {
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

      updateState = {
        ...updateState,
        status: "downloading",
        downloadedBytes,
        contentLength,
      };
      render();
    });

    pendingUpdate = null;
    updateState = {
      status: "installed",
      downloadedBytes: contentLength || downloadedBytes,
      contentLength,
    };
    showToast(copy[language].updateInstalled);
    render();
    await relaunch();
  } catch (error) {
    updateState = {
      status: "error",
      downloadedBytes: 0,
      error: normalizeUpdateError(error),
    };
    showToast(updateState.error || copy[language].updateFailed);
    render();
  }
}

function bindEvents() {
  document.querySelectorAll<HTMLElement>("[data-menu]").forEach((item) => {
    item.addEventListener("click", () => {
      captureSettingsDrafts();
      const nextMenu = item.dataset.menu as MenuId | undefined;
      if (nextMenu) {
        activeMenuId = nextMenu;
        userMenuOpen = false;
        render();
      }
    });
  });

  document.querySelectorAll<HTMLElement>("[data-platform]").forEach((item) => {
    item.addEventListener("click", () => {
      selectedPlatformId = item.dataset.platform || selectedPlatformId;
      activeMenuId = "channels";
      userMenuOpen = false;
      render();
    });
  });

  document.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    element.addEventListener("click", () => {
      const action = element.dataset.action;
      if (action === "toggle-sidebar") {
        sidebarCollapsed = !sidebarCollapsed;
        userMenuOpen = false;
        const windowEl = document.querySelector<HTMLElement>(".window");
        windowEl?.classList.toggle("is-collapsed", sidebarCollapsed);
        element.setAttribute("title", sidebarCollapsed ? "展开菜单" : "收起菜单");
        document.querySelector<HTMLElement>(".user-dropdown")?.remove();
        document.querySelector<HTMLElement>(".avatar-btn")?.setAttribute("aria-expanded", "false");
      }
      if (action === "toggle-user-menu") {
        captureSettingsDrafts();
        userMenuOpen = !userMenuOpen;
        render();
      }
      if (action === "refresh-platform") {
        userMenuOpen = false;
        void refreshPlatform(selectedPlatformId);
      }
      if (action === "close-auth") {
        stopAuthPolling();
        activeAuthTask = null;
        activeAuthMessage = "";
        render();
      }
      if (action === "check-auth") {
        void checkAuthOnce();
      }
      if (action === "logout") {
        userMenuOpen = false;
        logout();
      }
      if (action === "check-update") {
        void checkForUpdates({ silent: false });
      }
      if (action === "install-update") {
        void installPendingUpdate();
      }
      if (action === "toggle-release-content") {
        const releaseVersion = element.dataset.releaseVersion;
        if (!releaseVersion) return;
        if (expandedReleaseVersions.has(releaseVersion)) {
          expandedReleaseVersions.delete(releaseVersion);
        } else {
          expandedReleaseVersions.add(releaseVersion);
        }
        render();
      }
    });
  });

  document.querySelectorAll<HTMLElement>("[data-login]").forEach((element) => {
    element.addEventListener("click", (event) => {
      event.stopPropagation();
      activeMenuId = "channels";
      userMenuOpen = false;
      void startLogin(element.dataset.login || selectedPlatformId, readLoginTarget(element));
    });
  });

  document.querySelectorAll<HTMLElement>("[data-delete-account]").forEach((element) => {
    element.addEventListener("click", () => {
      void deleteAccount(element.dataset.deleteAccount || "");
    });
  });

  document.querySelectorAll<HTMLElement>("[data-refresh-account]").forEach((element) => {
    element.addEventListener("click", () => {
      void refreshAccount(element.dataset.refreshAccount || "");
    });
  });

  document.querySelectorAll<HTMLElement>("[data-open-homepage]").forEach((element) => {
    element.addEventListener("click", () => {
      void openHomepage(element.dataset.openHomepage || "");
    });
  });

  document.querySelectorAll<HTMLSelectElement>("[data-system-setting]").forEach((element) => {
    element.addEventListener("change", () => {
      captureSettingsDrafts();
      if (element.dataset.systemSetting === "language") {
        language = element.value === "en" ? "en" : "zh";
        localStorage.setItem("marketing-master-language", language);
      }
      if (element.dataset.systemSetting === "theme") {
        theme = element.value === "light" ? "light" : "dark";
        localStorage.setItem("marketing-master-theme", theme);
      }
      showToast(copy[language].settingsSaved);
      render();
    });
  });

  document.querySelectorAll<HTMLFormElement>("[data-settings-form]").forEach((form) => {
    form.addEventListener("submit", (event) => {
      event.preventDefault();

      if (!(event.currentTarget instanceof HTMLFormElement)) return;

      if (event.currentTarget.dataset.settingsForm === "profile") {
        void submitProfileForm(event.currentTarget);
      }

      if (event.currentTarget.dataset.settingsForm === "password") {
        void submitPasswordForm(event.currentTarget);
      }
    });
  });

  document.querySelectorAll<HTMLInputElement>(".settings-form input").forEach((input) => {
    input.addEventListener("input", () => {
      if (input.form?.dataset.settingsForm === "password") {
        input.form.querySelectorAll<HTMLInputElement>("input").forEach((field) => {
          field.setCustomValidity("");
        });
        return;
      }

      input.setCustomValidity("");
    });
  });

  document.querySelector<HTMLFormElement>("[data-feedback-form]")?.addEventListener("submit", (event) => {
    event.preventDefault();
    if (event.currentTarget instanceof HTMLFormElement) {
      void submitFeedbackForm(event.currentTarget);
    }
  });

  const autoUpdateInput = document.querySelector<HTMLInputElement>("[data-auto-update]");
  autoUpdateInput?.addEventListener("change", () => {
    autoUpdateEnabled = autoUpdateInput.checked;
    localStorage.setItem(AUTO_UPDATE_KEY, String(autoUpdateEnabled));
    autoUpdateChecked = false;
    showToast(autoUpdateEnabled ? copy[language].autoUpdateOn : copy[language].autoUpdateOff);
    render();
  });

  document.querySelectorAll<HTMLTextAreaElement | HTMLInputElement>("[data-feedback-form] textarea, [data-feedback-form] input")
    .forEach((field) => {
      field.addEventListener("input", () => field.setCustomValidity(""));
    });
}

function captureSettingsDrafts() {
  const profileForm = document.querySelector<HTMLFormElement>('[data-settings-form="profile"]');
  if (profileForm) {
    const formData = new FormData(profileForm);
    profileDraft = {
      nickname: String(formData.get("nickname") || ""),
    };
  }

  const passwordForm = document.querySelector<HTMLFormElement>('[data-settings-form="password"]');
  if (passwordForm) {
    const formData = new FormData(passwordForm);
    passwordDraft = {
      currentPassword: String(formData.get("currentPassword") || ""),
      newPassword: String(formData.get("newPassword") || ""),
      confirmPassword: String(formData.get("confirmPassword") || ""),
    };
  }

  const feedbackForm = document.querySelector<HTMLFormElement>("[data-feedback-form]");
  if (feedbackForm) {
    const formData = new FormData(feedbackForm);
    feedbackDraft = {
      content: String(formData.get("content") || ""),
      contact: String(formData.get("contact") || ""),
    };
  }
}

async function submitFeedbackForm(form: HTMLFormElement) {
  if (feedbackBusy) return;

  const formData = new FormData(form);
  feedbackDraft = {
    content: String(formData.get("content") || ""),
    contact: String(formData.get("contact") || ""),
  };
  feedbackBusy = true;
  render();

  try {
    await apiRequest<{ id: string }>("/v1/feedback", {
      method: "POST",
      body: {
        content: feedbackDraft.content,
        contact: feedbackDraft.contact,
      },
    });
    feedbackDraft = {
      content: "",
      contact: "",
    };
    feedbackBusy = false;
    render();
    showToast(copy[language].feedbackSubmitted);
  } catch (error) {
    const message = normalizeError(error);
    feedbackBusy = false;
    render();
    window.setTimeout(() => reportFeedbackFieldError(message), 0);
  }
}

function reportFeedbackFieldError(message: string) {
  const form = document.querySelector<HTMLFormElement>("[data-feedback-form]");
  if (!form) return;
  const field = form.elements.namedItem(message.includes("联系方式") ? "contact" : "content");

  if (field instanceof HTMLTextAreaElement || field instanceof HTMLInputElement) {
    field.setCustomValidity(message);
    field.reportValidity();
  }
}

async function submitProfileForm(form: HTMLFormElement) {
  if (profileBusy) return;

  const formData = new FormData(form);
  profileDraft = {
    nickname: String(formData.get("nickname") || ""),
  };
  profileBusy = true;
  render();

  try {
    currentUser = await apiRequest<AuthUser>("/v1/auth/profile", {
      method: "PUT",
      body: {
        nickname: profileDraft.nickname,
      },
    });
    profileDraft.nickname = currentUser.nickname;
    profileBusy = false;
    render();
    showToast(copy[language].profileSaved);
  } catch (error) {
    const message = normalizeError(error);
    profileBusy = false;
    render();
    window.setTimeout(() => reportSettingsFieldError("profile", "nickname", message), 0);
  }
}

async function submitPasswordForm(form: HTMLFormElement) {
  if (passwordBusy) return;

  const formData = new FormData(form);
  passwordDraft = {
    currentPassword: String(formData.get("currentPassword") || ""),
    newPassword: String(formData.get("newPassword") || ""),
    confirmPassword: String(formData.get("confirmPassword") || ""),
  };

  if (passwordDraft.newPassword !== passwordDraft.confirmPassword) {
    reportSettingsFieldError("password", "confirmPassword", copy[language].passwordMismatch);
    return;
  }

  passwordBusy = true;
  render();

  try {
    await apiRequest<AuthUser>("/v1/auth/password", {
      method: "PUT",
      body: {
        current_password: passwordDraft.currentPassword,
        new_password: passwordDraft.newPassword,
      },
    });
    passwordDraft = {
      currentPassword: "",
      newPassword: "",
      confirmPassword: "",
    };
    passwordBusy = false;
    render();
    showToast(copy[language].passwordChanged);
  } catch (error) {
    const message = normalizeError(error);
    passwordBusy = false;
    render();
    window.setTimeout(() => {
      reportSettingsFieldError("password", passwordErrorField(message), message);
    }, 0);
  }
}

function reportSettingsFieldError(formName: string, fieldName: string, message: string) {
  const form = document.querySelector<HTMLFormElement>(`[data-settings-form="${formName}"]`);
  if (!form) return;
  const field = form.elements.namedItem(fieldName);

  if (field instanceof HTMLInputElement) {
    field.setCustomValidity(message);
    field.reportValidity();
  }
}

function passwordErrorField(message: string) {
  const normalized = message.toLowerCase();

  if (message.includes("新密码") || normalized.includes("new password")) {
    return "newPassword";
  }

  return "currentPassword";
}

function bindAuthEvents() {
  document.querySelectorAll<HTMLElement>("[data-auth-action]").forEach((element) => {
    element.addEventListener("click", () => {
      const action = element.dataset.authAction;

      if (action === "refresh-captcha") {
        captureAuthDraftFromForm();
        void loadCaptchaAndRender();
      }

      if (action === "show-register") {
        captureAuthDraftFromForm();
        authViewMode = "register";
        void loadCaptchaAndRender();
      }

      if (action === "show-login") {
        captureAuthDraftFromForm();
        authViewMode = "login";
        void loadCaptchaAndRender();
      }
    });
  });

  document.querySelector<HTMLFormElement>("[data-auth-form]")?.addEventListener("submit", (event) => {
    event.preventDefault();
    if (event.currentTarget instanceof HTMLFormElement) {
      void submitAuthForm(event.currentTarget);
    }
  });

  document.querySelectorAll<HTMLInputElement>(".login-form input").forEach((input) => {
    input.addEventListener("input", () => {
      input.setCustomValidity("");
    });
  });
}

function captureAuthDraftFromForm() {
  const form = document.querySelector<HTMLFormElement>("[data-auth-form]");
  if (!form) return;
  const formData = new FormData(form);
  authDraft = {
    account: String(formData.get("account") || ""),
    password: String(formData.get("password") || ""),
    nickname: String(formData.get("nickname") || ""),
    captchaCode: String(formData.get("captchaCode") || ""),
  };
}

async function submitAuthForm(form: HTMLFormElement) {
  if (!captcha || authBusy) return;
  const formData = new FormData(form);
  authDraft = {
    account: String(formData.get("account") || ""),
    password: String(formData.get("password") || ""),
    nickname: String(formData.get("nickname") || ""),
    captchaCode: String(formData.get("captchaCode") || ""),
  };
  const isRegister = authViewMode === "register";
  authBusy = true;
  render();

  try {
    const session = await apiRequest<AuthSession>(isRegister ? "/v1/auth/register" : "/v1/auth/login", {
      method: "POST",
      body: {
        account: authDraft.account,
        password: authDraft.password,
        nickname: authDraft.nickname,
        captcha_id: captcha.captchaId,
        captcha_code: authDraft.captchaCode,
      },
    });

    authToken = session.token;
    currentUser = session.user;
    profileDraft.nickname = session.user.nickname;
    authDraft = {
      account: "",
      password: "",
      nickname: "",
      captchaCode: "",
    };
    localStorage.setItem(AUTH_TOKEN_KEY, authToken);
    await loadClientData();
    render();
    showToast(isRegister ? copy[language].registerSuccess : copy[language].loginSuccess);
  } catch (error) {
    const message = normalizeError(error);
    await loadCaptcha();
    authBusy = false;
    render();
    window.setTimeout(() => reportAuthFieldError(message), 0);
  } finally {
    authBusy = false;
  }
}

function reportAuthFieldError(message: string) {
  const form = document.querySelector<HTMLFormElement>("[data-auth-form]");
  if (!form) return;

  const field = getAuthErrorField(message, form);
  field.setCustomValidity(message);
  field.reportValidity();
}

function getAuthErrorField(message: string, form: HTMLFormElement) {
  const normalized = message.toLowerCase();
  const hasCaptchaError = message.includes("验证码") || normalized.includes("captcha");
  const hasPasswordError = message.includes("密码") || normalized.includes("password");
  const hasAccountError = message.includes("账号") || normalized.includes("account") || normalized.includes("user");
  const fieldName = hasCaptchaError ? "captchaCode" : hasPasswordError ? "password" : hasAccountError ? "account" : "account";
  const field = form.elements.namedItem(fieldName);

  if (field instanceof HTMLInputElement) {
    return field;
  }

  return form.querySelector<HTMLInputElement>("input") || document.createElement("input");
}

async function loadCaptchaAndRender() {
  await loadCaptcha();
  render();
}

async function loadCaptcha() {
  try {
    captcha = await apiRequest<CaptchaResponse>("/v1/auth/captcha", { skipAuth: true });
  } catch (error) {
    captcha = null;
    console.warn("Failed to load captcha", error);
  }
}

function logout() {
  stopAuthPolling();
  authToken = "";
  currentUser = null;
  authViewMode = "login";
  profileBusy = false;
  passwordBusy = false;
  authDraft = {
    account: "",
    password: "",
    nickname: "",
    captchaCode: "",
  };
  profileDraft = {
    nickname: "",
  };
  passwordDraft = {
    currentPassword: "",
    newPassword: "",
    confirmPassword: "",
  };
  activeAuthTask = null;
  accounts = [];
  localStorage.removeItem(AUTH_TOKEN_KEY);
  render();
  void loadCaptchaAndRender();
}

async function startLogin(platformId: string, loginTarget?: LoginTarget) {
  try {
    selectedPlatformId = platformId;
    const request = loginTarget
      ? { userId: requireCurrentUserId(), platformId, loginTarget }
      : { userId: requireCurrentUserId(), platformId };
    activeAuthTask = await invokeCommand<StartLoginResponse>("start_channel_login", {
      request,
    });
    activeAuthMessage =
      activeAuthTask.instructions ||
      (isQrAuth(activeAuthTask) ? copy[language].authQrDesc : copy[language].authDesc);
    showToast(isQrAuth(activeAuthTask) ? copy[language].authQrOpened : copy[language].authOpened);
    render();
    startAuthPolling();
  } catch (error) {
    showToast(normalizeError(error));
  }
}

function readLoginTarget(element: HTMLElement): LoginTarget | undefined {
  const target = element.dataset.loginTarget;
  return target === "home" || target === "creator" ? target : undefined;
}

function startAuthPolling() {
  stopAuthPolling();
  authPollTimer = window.setInterval(() => {
    void checkAuthOnce(false);
  }, 1800);
}

function stopAuthPolling() {
  if (authPollTimer) {
    window.clearInterval(authPollTimer);
    authPollTimer = undefined;
  }
}

async function checkAuthOnce(verbose = true) {
  if (!activeAuthTask) return;
  try {
    const result = await invokeCommand<AuthTaskStatus>("get_auth_task_status", {
      taskId: activeAuthTask.taskId,
      userId: requireCurrentUserId(),
    });
    if (result.status === "success") {
      stopAuthPolling();
      accounts = await invokeCommand<ChannelAccount[]>("list_channel_accounts", {
        userId: requireCurrentUserId(),
      });
      if (result.account) {
        await apiRequest<ChannelAccount>("/v1/channel/accounts", {
          method: "POST",
          body: backendAccountPayload(result.account),
        }).catch(() => undefined);
      } else {
        await mirrorAccountsToBackend(accounts);
      }
      activeAuthTask = null;
      activeAuthMessage = "";
      activeMenuId = "channels";
      showToast(copy[language].authDone);
      render();
    } else if (result.status === "failed") {
      stopAuthPolling();
      activeAuthTask = null;
      activeAuthMessage = "";
      showToast(result.message || copy[language].authFailed);
      render();
    } else {
      const message = result.message || copy[language].authWaiting;
      if (activeAuthMessage !== message) {
        activeAuthMessage = message;
        render();
      }
      if (verbose) showToast(message);
    }
  } catch (error) {
    if (verbose) {
      const message = normalizeError(error);
      activeAuthMessage = message;
      showToast(message);
      render();
    }
  }
}

async function refreshAccount(accountId: string) {
  if (!accountId || refreshingAccountIds.has(accountId)) return;
  refreshingAccountIds.add(accountId);
  render();
  let toastMessage = "";
  try {
    const updated = await invokeCommand<ChannelAccount>("refresh_channel_account", {
      accountId,
      userId: requireCurrentUserId(),
    });
    await apiRequest<ChannelAccount>("/v1/channel/accounts", {
      method: "POST",
      body: backendAccountPayload(updated),
    }).catch(() => undefined);
    accounts = accounts.map((item) => (item.id === updated.id ? updated : item));
    toastMessage = copy[language].accountRefreshed;
  } catch (error) {
    toastMessage = normalizeError(error);
  } finally {
    refreshingAccountIds.delete(accountId);
    render();
    if (toastMessage) showToast(toastMessage);
  }
}

async function refreshPlatform(platformId: string) {
  if (refreshingPlatformIds.has(platformId)) return;
  const list = accounts.filter((item) => item.platformId === platformId);
  refreshingPlatformIds.add(platformId);
  list.forEach((account) => refreshingAccountIds.add(account.id));
  render();
  let failedCount = 0;
  for (const account of list) {
    try {
      const updated = await invokeCommand<ChannelAccount>("refresh_channel_account", {
        accountId: account.id,
        userId: requireCurrentUserId(),
      });
      await apiRequest<ChannelAccount>("/v1/channel/accounts", {
        method: "POST",
        body: backendAccountPayload(updated),
      }).catch(() => undefined);
      accounts = accounts.map((item) => (item.id === updated.id ? updated : item));
    } catch {
      failedCount += 1;
    }
  }
  list.forEach((account) => refreshingAccountIds.delete(account.id));
  refreshingPlatformIds.delete(platformId);
  const toastMessage =
    failedCount
      ? `${copy[language].platformRefreshed} ${language === "zh" ? `${failedCount} 个账号失败。` : `${failedCount} failed.`}`
      : copy[language].platformRefreshed;
  render();
  showToast(toastMessage);
}

async function openHomepage(accountId: string) {
  if (!accountId) return;
  try {
    await invokeCommand<void>("open_account_homepage", {
      accountId,
      userId: requireCurrentUserId(),
    });
  } catch (error) {
    showToast(normalizeError(error));
  }
}

async function deleteAccount(accountId: string) {
  if (!accountId) return;
  try {
    await invokeCommand<void>("delete_channel_account", {
      accountId,
      userId: requireCurrentUserId(),
    });
    await apiRequest<void>(`/v1/channel/accounts/${encodeURIComponent(accountId)}`, {
      method: "DELETE",
    }).catch(() => undefined);
    accounts = accounts.filter((item) => item.id !== accountId);
    showToast(copy[language].accountDeleted);
    render();
  } catch (error) {
    showToast(normalizeError(error));
  }
}

function getSelectedPlatform() {
  return platforms.find((item) => item.id === selectedPlatformId) || platforms[0];
}

function getPlatformSettings(platformId: string): PlatformAuthSettings {
  let item = settings.platforms.find((entry) => entry.platformId === platformId);
  if (!item) {
    item = defaultPlatformSettings().find((entry) => entry.platformId === platformId)!;
    settings.platforms.push(item);
  }
  return item;
}

function platformItem(platform: PlatformInfo) {
  const count = accounts.filter((item) => item.platformId === platform.id).length;
  const active = platform.id === selectedPlatformId && activeMenuId === "channels";
  getPlatformSettings(platform.id);

  return renderPlatformItem({
    platform,
    text: copy[language],
    count,
    active,
    countLabel: formatAccountCountLabel(count, language),
  });
}

function accountItem(account: ChannelAccount) {
  const text = copy[language];
  const platform = platforms.find((item) => item.id === account.platformId);

  return renderAccountItem({
    account,
    text,
    platform,
    isRefreshing: refreshingAccountIds.has(account.id),
    followersText: accountFollowersText(account, text, language),
    syncText: accountSyncText(account, text, language),
    fallbackAvatar: accountInitials(account.nickname),
  });
}

function emptyAccounts(platform: PlatformInfo) {
  return renderEmptyAccounts({
    platform,
    text: copy[language],
  });
}

function authDialog(task: StartLoginResponse) {
  const text = copy[language];
  const platform = getSelectedPlatform();
  const description = activeAuthMessage || task.instructions || (isQrAuth(task) ? text.authQrDesc : text.authDesc);

  return renderAuthDialog({
    task,
    text,
    platform,
    description,
  });
}

function navItem(id: SidebarMenuId, iconName: string) {
  return renderNavItem({
    id,
    iconName,
    label: copy[language].menu[id],
    active: id === activeMenuId,
  });
}

function showToast(message: string) {
  const toast = document.querySelector<HTMLDivElement>(".toast");
  if (!toast) return;
  toast.textContent = message;
  toast.hidden = false;
  if (toastTimer) window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toast.hidden = true;
  }, 3200);
}

function normalizeError(error: unknown) {
  const message = typeof error === "string" ? error : error instanceof Error ? error.message : "";
  if (message) return message;
  return language === "zh" ? "操作失败，请稍后重试。" : "Operation failed. Please try again.";
}

function normalizeUpdateError(error: unknown) {
  const message = normalizeError(error);
  if (!message || /not implemented|not available|permission|plugin/i.test(message)) {
    return copy[language].updateUnavailable;
  }
  return message;
}
