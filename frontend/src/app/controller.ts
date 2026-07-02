import { listen } from "@tauri-apps/api/event";
import type {
  AuthSession,
  AuthTaskStatus,
  AuthUser,
  AuthViewMode,
  Bootstrap,
  CaptchaResponse,
  ChannelAccount,
  LanguageMode,
  LoginTarget,
  MenuId,
  PlatformInfo,
  StartLoginResponse,
  ThemeMode,
} from "../domain/types";
import {
  type ChannelAccountContent,
  mockCommentsForAccounts,
  mockWorksForAccounts,
  type ChannelWorksPage,
  type ChannelWork,
  type ContentTab,
} from "../domain/channel-content";
import { fallbackPlatforms } from "../domain/platforms";
import { isQrAuth } from "../domain/auth-task";
import { releaseHistoryForLanguage } from "../domain/releases";
import { copy } from "../i18n/copy";
import { readStoredMode } from "../utils/storage";
import { normalizeError as normalizeErrorMessage } from "../utils/errors";
import {
  clearFieldError,
  clearFormFieldErrors,
  formValue,
  reportFieldError,
  reportNamedFieldError,
} from "../utils/forms";
import {
  formatFollowersTotal as formatAccountFollowersTotal,
  initials as accountInitials,
} from "../utils/format";
import {
  renderAccountNavItem,
  renderAuthDialog,
  renderPlatformTreeItem,
} from "../ui/channel-components";
import { renderAccountDropdown } from "../ui/user-menu";
import { renderAuthPage } from "../pages/auth";
import { renderChannelsPage } from "../pages/channels";
import { renderFeedbackPage } from "../pages/feedback";
import { renderPasswordPage } from "../pages/password";
import { renderProfilePage } from "../pages/profile";
import { renderReleasesPage } from "../pages/releases";
import { renderSettingsPage } from "../pages/settings";
import { renderAppShell } from "./shell";
import { API_BASE_URL, AUTH_TOKEN_KEY, INPUT_HINTS_OFF } from "../config/app";
import { UpdateController } from "../features/updater";
import { upsertAccount } from "../features/channel-sync";
import { requestApi, type ApiRequestOptions } from "../services/api";
import { invokeCommand } from "../services/tauri-commands";

let platforms: PlatformInfo[] = fallbackPlatforms;
let accounts: ChannelAccount[] = [];
let selectedPlatformId = "xiaohongshu";
let selectedAccountId: string | null = null;
let expandedPlatformIds = new Set<string>();
let activeContentTab: ContentTab = "overview";
type ChannelWorkType = "video" | "article";
type OverviewPeriod = 1 | 7 | 30 | 90 | 36500 | 65535;
const BILIBILI_TOTAL_PERIOD: OverviewPeriod = 65535;
let channelSearchQuery = "";
let channelSearchComposing = false;
let activeMenuId: MenuId = "channels";
let userMenuOpen = false;
let activeAuthTask: StartLoginResponse | null = null;
let activeAuthMessage = "";
let toastTimer: number | undefined;
let authPollTimer: number | undefined;
let refreshingAccountIds = new Set<string>();
let refreshingPlatformIds = new Set<string>();
let openingHomepageIds = new Set<string>();
let syncingContentIds = new Set<string>();
let loadingWorksPageIds = new Set<string>();
let accountContentCache = new Map<string, ChannelAccountContent>();
let accountWorksPages = new Map<string, ChannelWorksPage[]>();
let overviewPeriodByAccount = new Map<string, OverviewPeriod>();
let workTypeByAccount = new Map<string, ChannelWorkType>();
let language: LanguageMode = readStoredMode("channel-nest-language", "zh", ["zh", "en"]);
let theme: ThemeMode = readStoredMode("channel-nest-theme", "dark", ["dark", "light"]);
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

const updates = new UpdateController({
  getText: () => copy[language],
  getFallbackError: () => (language === "zh" ? "操作失败，请稍后重试。" : "Operation failed. Please try again."),
  canAutoCheck: () => Boolean(currentUser),
  render: () => render(),
  showToast: (message) => showToast(message),
});

let appRoot: HTMLDivElement;

export async function startApp() {
  const app = document.querySelector<HTMLDivElement>("#app");

  if (!app) {
    throw new Error("App root missing");
  }

  appRoot = app;
  await bindTauriAccountEvents();
  await boot();
}

async function bindTauriAccountEvents() {
  try {
    await listen<ChannelAccount>("channel-account-updated", (event) => {
      void applyAccountUpdate(event.payload, { rerender: true });
    });
  } catch (error) {
    console.warn("Account update events are not available", error);
  }
}

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
    syncChannelSelection({ preferFirstWithAccounts: true, expandSelected: true });
  } catch (error) {
    console.warn("Using browser fallback because Tauri is not available", error);
    syncChannelSelection({ preferFirstWithAccounts: true, expandSelected: true });
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

async function applyAccountUpdate(
  updated: ChannelAccount,
  options: { rerender?: boolean } = {},
) {
  if (currentUser?.id && updated.userId && updated.userId !== currentUser.id) {
    return;
  }

  accounts = upsertAccount(accounts, updated, currentUser?.id);
  expandedPlatformIds.add(updated.platformId);
  syncChannelSelection();

  if (options.rerender) {
    render();
  }
}

function render() {
  if (!currentUser) {
    appRoot.innerHTML = authPage();
    bindAuthEvents();
    return;
  }

  appRoot.innerHTML = renderAppShell({
    theme,
    currentUser,
    activeMenuId,
    homeLabel: copy[language].homepage,
    userMenuOpen,
    mainContent: renderMainContent(),
    accountDropdown: accountDropdown(),
    authDialog: activeAuthTask ? authDialog(activeAuthTask) : "",
  });

  bindEvents();
  updates.scheduleAutoCheck();
}

function renderPreservingWorkspaceScroll() {
  const scrollTop = document.querySelector<HTMLElement>(".workspace-body")?.scrollTop;
  render();
  if (typeof scrollTop !== "number") return;

  const restore = () => {
    const workspaceBody = document.querySelector<HTMLElement>(".workspace-body");
    if (workspaceBody) {
      workspaceBody.scrollTop = scrollTop;
    }
  };
  restore();
  window.requestAnimationFrame(restore);
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
    hasUpdate: updates.state.status === "available",
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
  syncChannelSelection();
  const selectedPlatform = getSelectedPlatform();
  const selectedAccount = getSelectedAccount();
  const selectedWorkType = selectedAccount ? selectedAccountWorkType(selectedAccount.id) : "video";
  const selectedAccounts = accounts.filter((item) => item.platformId === selectedPlatform.id);
  const visiblePlatforms = visibleChannelPlatforms();
  const allWorks = worksForCurrentSelection(selectedAccount, selectedPlatform.id);
  const allComments = mockCommentsForAccounts(accounts);
  const works = allWorks.filter((item) =>
    selectedAccount ? item.accountId === selectedAccount.id : item.platformId === selectedPlatform.id,
  );
  const comments = allComments.filter((item) =>
    selectedAccount ? item.accountId === selectedAccount.id : item.platformId === selectedPlatform.id,
  );
  const platformRefreshing = refreshingPlatformIds.has(selectedPlatform.id);
  const selectedAccountContent = selectedAccount ? accountContentCache.get(selectedAccount.id) || null : null;
  const selectedWorksPages = selectedAccount ? accountWorksPages.get(worksStateKey(selectedAccount.id, selectedWorkType)) || [] : [];
  const overviewPeriod = selectedAccount ? selectedAccountOverviewPeriod(selectedAccount) : 7;

  return renderChannelsPage({
    text: copy[language],
    language,
    selectedPlatform,
    selectedAccount,
    selectedAccounts,
    platformRefreshing,
    selectedAccountRefreshing: selectedAccount ? refreshingAccountIds.has(selectedAccount.id) : false,
    selectedAccountOpeningHomepage: selectedAccount ? openingHomepageIds.has(selectedAccount.id) : false,
    selectedAccountContent,
    selectedAccountContentLoading: selectedAccount ? syncingContentIds.has(selectedAccount.id) : false,
    selectedWorksPages,
    selectedWorksLoading: selectedAccount ? isWorksLoading(selectedAccount.id) : false,
    selectedWorkType,
    overviewPeriod,
    activeTab: activeContentTab,
    works,
    comments,
    platforms: visiblePlatforms,
    searchQuery: channelSearchQuery,
    hasSearchResults: visiblePlatforms.length > 0,
    platformTree,
    formatFollowersTotal: (items) => formatAccountFollowersTotal(items, language),
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

  return renderReleasesPage({
    text: copy[language],
    entries,
    updateState: updates.state,
    autoUpdateEnabled: updates.autoEnabled,
    canInstall: updates.canInstall,
    isChecking: updates.isChecking,
    isDownloading: updates.isDownloading,
    status: updates.statusText(),
    progress: updates.progressPercent(),
    expandedReleaseVersions: updates.expandedVersions,
  });
}

function bindEvents() {
  document.querySelector<HTMLElement>(".window")?.addEventListener("click", (event) => {
    if (!userMenuOpen) return;
    if (event.target instanceof Element && event.target.closest(".corner-menu-wrap")) return;

    captureSettingsDrafts();
    userMenuOpen = false;
    render();
  });

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
    item.addEventListener("click", (event) => {
      if (event.target instanceof Element && event.target.closest("[data-toggle-platform]")) return;
      const nextPlatformId = item.dataset.platform || selectedPlatformId;
      selectedPlatformId = nextPlatformId;
      selectedAccountId = null;
      if (!normalizedChannelSearch()) {
        togglePlatformExpanded(nextPlatformId);
      }
      activeMenuId = "channels";
      userMenuOpen = false;
      render();
    });
  });

  const channelSearch = document.querySelector<HTMLInputElement>("[data-channel-search]");
  channelSearch?.addEventListener("compositionstart", () => {
    channelSearchComposing = true;
  });
  channelSearch?.addEventListener("compositionend", (event) => {
    if (!(event.currentTarget instanceof HTMLInputElement)) return;
    channelSearchComposing = false;
    updateChannelSearch(event.currentTarget.value);
  });
  channelSearch?.addEventListener("input", (event) => {
    if (!(event.currentTarget instanceof HTMLInputElement) || channelSearchComposing) return;
    updateChannelSearch(event.currentTarget.value);
  });

  document.querySelectorAll<HTMLElement>("[data-toggle-platform]").forEach((item) => {
    item.addEventListener("click", (event) => {
      event.stopPropagation();
      const platformId = item.dataset.togglePlatform;
      if (!platformId) return;
      togglePlatformExpanded(platformId);
      activeMenuId = "channels";
      userMenuOpen = false;
      render();
    });
  });

  document.querySelectorAll<HTMLElement>("[data-account]").forEach((item) => {
    item.addEventListener("click", () => {
      const accountId = item.dataset.account;
      const account = accounts.find((candidate) => candidate.id === accountId);
      if (!account) return;
      selectedAccountId = account.id;
      selectedPlatformId = account.platformId;
      expandedPlatformIds.add(account.platformId);
      activeContentTab = "overview";
      activeMenuId = "channels";
      userMenuOpen = false;
      void syncAccountContent(account.id);
      render();
    });
  });

  document.querySelectorAll<HTMLElement>("[data-channel-tab]").forEach((item) => {
    item.addEventListener("click", () => {
      const nextTab = item.dataset.channelTab;
      if (!isContentTab(nextTab)) return;
      activeContentTab = nextTab;
      activeMenuId = "channels";
      userMenuOpen = false;
      render();
      if (nextTab === "works") {
        void loadWorksPageForSelectedAccount({ force: !selectedAccountHasWorksPage() });
      }
    });
  });

  document.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    element.addEventListener("click", () => {
      const action = element.dataset.action;
      if (action === "toggle-user-menu") {
        captureSettingsDrafts();
        userMenuOpen = !userMenuOpen;
        render();
      }
      if (action === "refresh-platform") {
        userMenuOpen = false;
        void refreshPlatform(selectedPlatformId);
      }
      if (action === "overview-period") {
        const account = getSelectedAccount();
        if (!account) return;
        overviewPeriodByAccount.set(account.id, readOverviewPeriod(element.dataset.period));
        render();
      }
      if (action === "work-type") {
        const account = getSelectedAccount();
        const workType = element.dataset.workType === "article" ? "article" : "video";
        if (!account || !["wechat-channels", "bilibili"].includes(account.platformId)) return;
        workTypeByAccount.set(account.id, workType);
        render();
        if (activeContentTab === "works") {
          void loadWorksPageForSelectedAccount({ force: !selectedAccountHasWorksPage() });
        }
      }
      if (action === "load-more-works") {
        void loadWorksPageForSelectedAccount({ next: true });
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
        void updates.check({ silent: false });
      }
      if (action === "install-update") {
        void updates.installPending();
      }
      if (action === "toggle-release-content") {
        const releaseVersion = element.dataset.releaseVersion;
        if (!releaseVersion) return;
        updates.toggleRelease(releaseVersion);
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

  document.querySelectorAll<HTMLElement>("[data-copy-account]").forEach((element) => {
    element.addEventListener("click", (event) => {
      event.stopPropagation();
      const value = element.dataset.copyAccount || "";
      void copyAccountValue(value);
    });
  });

  document.querySelectorAll<HTMLSelectElement>("[data-system-setting]").forEach((element) => {
    element.addEventListener("change", () => {
      captureSettingsDrafts();
      if (element.dataset.systemSetting === "language") {
        language = element.value === "en" ? "en" : "zh";
        localStorage.setItem("channel-nest-language", language);
      }
      if (element.dataset.systemSetting === "theme") {
        theme = element.value === "light" ? "light" : "dark";
        localStorage.setItem("channel-nest-theme", theme);
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
        clearFormFieldErrors(input.form);
        return;
      }

      clearFieldError(input);
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
    updates.setAutoEnabled(autoUpdateInput.checked);
  });

  document.querySelectorAll<HTMLTextAreaElement | HTMLInputElement>("[data-feedback-form] textarea, [data-feedback-form] input")
    .forEach((field) => {
      field.addEventListener("input", () => clearFieldError(field));
    });
}

function captureSettingsDrafts() {
  const profileForm = document.querySelector<HTMLFormElement>('[data-settings-form="profile"]');
  if (profileForm) {
    profileDraft = {
      nickname: formValue(profileForm, "nickname"),
    };
  }

  const passwordForm = document.querySelector<HTMLFormElement>('[data-settings-form="password"]');
  if (passwordForm) {
    passwordDraft = {
      currentPassword: formValue(passwordForm, "currentPassword"),
      newPassword: formValue(passwordForm, "newPassword"),
      confirmPassword: formValue(passwordForm, "confirmPassword"),
    };
  }

  const feedbackForm = document.querySelector<HTMLFormElement>("[data-feedback-form]");
  if (feedbackForm) {
    feedbackDraft = {
      content: formValue(feedbackForm, "content"),
      contact: formValue(feedbackForm, "contact"),
    };
  }
}

async function submitFeedbackForm(form: HTMLFormElement) {
  if (feedbackBusy) return;

  feedbackDraft = {
    content: formValue(form, "content"),
    contact: formValue(form, "contact"),
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
  reportNamedFieldError(form, message.includes("联系方式") ? "contact" : "content", message);
}

async function submitProfileForm(form: HTMLFormElement) {
  if (profileBusy) return;

  profileDraft = {
    nickname: formValue(form, "nickname"),
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

  passwordDraft = {
    currentPassword: formValue(form, "currentPassword"),
    newPassword: formValue(form, "newPassword"),
    confirmPassword: formValue(form, "confirmPassword"),
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
  reportNamedFieldError(form, fieldName, message);
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
      clearFieldError(input);
    });
  });
}

function captureAuthDraftFromForm() {
  const form = document.querySelector<HTMLFormElement>("[data-auth-form]");
  if (!form) return;
  authDraft = {
    account: formValue(form, "account"),
    password: formValue(form, "password"),
    nickname: formValue(form, "nickname"),
    captchaCode: formValue(form, "captchaCode"),
  };
}

async function submitAuthForm(form: HTMLFormElement) {
  if (!captcha || authBusy) return;
  authDraft = {
    account: formValue(form, "account"),
    password: formValue(form, "password"),
    nickname: formValue(form, "nickname"),
    captchaCode: formValue(form, "captchaCode"),
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
  reportFieldError(field, message);
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
  selectedAccountId = null;
  activeContentTab = "overview";
  accounts = [];
  localStorage.removeItem(AUTH_TOKEN_KEY);
  render();
  void loadCaptchaAndRender();
}

async function startLogin(platformId: string, loginTarget?: LoginTarget) {
  try {
    selectedPlatformId = platformId;
    selectedAccountId = null;
    expandedPlatformIds.add(platformId);
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
      let accountIdToSync: string | null = null;
      accounts = await invokeCommand<ChannelAccount[]>("list_channel_accounts", {
        userId: requireCurrentUserId(),
      });
      if (result.account?.id) {
        selectedAccountId = result.account.id;
        activeContentTab = "overview";
        accountIdToSync = result.account.id;
      }
      syncChannelSelection({ expandSelected: true });
      activeAuthTask = null;
      activeAuthMessage = "";
      activeMenuId = "channels";
      showToast(copy[language].authDone);
      render();
      if (accountIdToSync) {
        void syncAccountContent(accountIdToSync, { force: true });
      }
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

function markAccountUnavailableLocally(accountId: string) {
  const syncedAt = new Date().toISOString();
  let updatedAccount: ChannelAccount | null = null;

  accounts = accounts.map((item) => {
    if (item.id !== accountId) return item;

    updatedAccount = {
      ...item,
      status: "expired",
      lastSyncAt: syncedAt,
      updatedAt: syncedAt,
    };
    return updatedAccount;
  });

  return updatedAccount;
}

async function markAccountUnavailable(accountId: string) {
  try {
    const updated = await invokeCommand<ChannelAccount>("mark_channel_account_unavailable", {
      accountId,
      userId: requireCurrentUserId(),
    });
    accounts = accounts.map((item) => (item.id === updated.id ? updated : item));
    return updated;
  } catch (error) {
    console.warn("Failed to persist unavailable account status", error);
    return markAccountUnavailableLocally(accountId);
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
    await applyAccountUpdate(updated);
    await syncAccountContent(accountId, { force: true, silent: true });
    toastMessage = copy[language].accountRefreshed;
  } catch (error) {
    const expired = await markAccountUnavailable(accountId);
    if (expired) {
      await applyAccountUpdate(expired);
    }
    toastMessage = normalizeError(error);
  } finally {
    refreshingAccountIds.delete(accountId);
    render();
    if (toastMessage) showToast(toastMessage);
  }
}

async function syncAccountContent(
  accountId: string,
  options: { force?: boolean; silent?: boolean } = {},
) {
  const account = accounts.find((item) => item.id === accountId);
  if (!account || !supportsAccountContent(account.platformId) || syncingContentIds.has(accountId)) return;
  syncingContentIds.add(accountId);
  render();
  try {
    const content = await invokeCommand<ChannelAccountContent>("sync_channel_account_content", {
      request: {
        accountId,
        userId: requireCurrentUserId(),
        force: Boolean(options.force),
      },
    });
    accountContentCache.set(accountId, content);
    applyContentProfileToAccount(content);
    if (content.error && !options.silent) {
      showToast(content.error);
    }
  } catch (error) {
    if (!options.silent) showToast(normalizeError(error));
  } finally {
    syncingContentIds.delete(accountId);
    render();
  }
}

async function loadWorksPageForSelectedAccount(options: { next?: boolean; force?: boolean } = {}) {
  const account = getSelectedAccount();
  if (!account || !supportsWorksPages(account.platformId)) return;
  const workType = selectedAccountWorkType(account.id);
  const pagesKey = worksStateKey(account.id, workType);
  const pages = accountWorksPages.get(pagesKey) || [];
  const pageKey = options.next ? pages[pages.length - 1]?.nextPageKey || "" : "";
  if (options.next && !pageKey) return;
  const loadingKey = `${pagesKey}:${pageKey}`;
  if (loadingWorksPageIds.has(loadingKey)) return;
  loadingWorksPageIds.add(loadingKey);
  if (options.next) {
    renderPreservingWorkspaceScroll();
  } else {
    render();
  }
  try {
    const page = await invokeCommand<ChannelWorksPage>("load_channel_account_works_page", {
      request: {
        accountId: account.id,
        userId: requireCurrentUserId(),
        pageKey,
        workType: ["wechat-channels", "bilibili"].includes(account.platformId) ? workType : undefined,
        force: Boolean(options.force),
      },
    });
    const existingPages = options.next ? pages : [];
    const nextPages = [...existingPages.filter((item) => item.pageKey !== page.pageKey), page]
      .sort((a, b) => pageSortValue(a.pageKey) - pageSortValue(b.pageKey));
    accountWorksPages.set(pagesKey, nextPages);
    if (page.error && options.next) {
      showToast(page.error);
    }
  } catch (error) {
    if (options.next) showToast(normalizeError(error));
  } finally {
    loadingWorksPageIds.delete(loadingKey);
    if (options.next) {
      renderPreservingWorkspaceScroll();
    } else {
      render();
    }
  }
}

function applyContentProfileToAccount(content: ChannelAccountContent) {
  if (!content.profile) return;
  accounts = accounts.map((account) => {
    if (account.id !== content.accountId) return account;
    return {
      ...account,
      followers: content.profile?.followers ?? account.followers,
      following: content.profile?.following ?? account.following,
      likes: content.profile?.likes ?? account.likes,
      lastSyncAt: content.profile?.lastSyncAt || account.lastSyncAt,
    };
  });
}

function pageSortValue(pageKey: string) {
  const normalized = pageKey.includes(":") ? pageKey.split(":").pop() || "" : pageKey;
  if (!normalized) return 0;
  const value = Number(normalized);
  return Number.isFinite(value) ? value : Number.MAX_SAFE_INTEGER;
}

async function refreshPlatform(platformId: string) {
  if (refreshingPlatformIds.has(platformId)) return;
  const list = accounts.filter((item) => item.platformId === platformId);
  if (!list.length) return;

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
      await applyAccountUpdate(updated);
    } catch (error) {
      console.warn("Failed to refresh account status", error);
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
  if (!accountId || openingHomepageIds.has(accountId) || refreshingAccountIds.has(accountId)) return;
  openingHomepageIds.add(accountId);
  render();
  try {
    await withTimeout(
      invokeCommand<ChannelAccount>("open_account_homepage", {
        accountId,
        userId: requireCurrentUserId(),
      }),
      5000,
      language === "zh"
        ? "打开主页窗口超时，请稍后重试。"
        : "Opening the homepage window timed out. Please try again.",
    );
  } catch (error) {
    showToast(normalizeError(error));
  } finally {
    openingHomepageIds.delete(accountId);
    render();
  }
}

async function copyAccountValue(value: string) {
  const account = value.trim();
  if (!account) return;
  try {
    await writeClipboardText(account);
    showToast(copy[language].accountCopied);
  } catch (error) {
    showToast(normalizeError(error));
  }
}

async function writeClipboardText(value: string) {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(value);
      return;
    } catch {
      // Fall back to the legacy command below for embedded WebViews.
    }
  }
  const input = document.createElement("textarea");
  input.value = value;
  input.setAttribute("readonly", "true");
  input.style.position = "fixed";
  input.style.left = "-9999px";
  document.body.appendChild(input);
  input.select();
  const copied = document.execCommand("copy");
  input.remove();
  if (!copied) {
    throw new Error(copy[language].accountCopyFailed);
  }
}

async function deleteAccount(accountId: string) {
  if (!accountId) return;
  try {
    await invokeCommand<void>("delete_channel_account", {
      accountId,
      userId: requireCurrentUserId(),
    });
    accounts = accounts.filter((item) => item.id !== accountId);
    accountContentCache.delete(accountId);
    deleteWorksStateForAccount(accountId);
    overviewPeriodByAccount.delete(accountId);
    workTypeByAccount.delete(accountId);
    if (selectedAccountId === accountId) {
      selectedAccountId = null;
    }
    syncChannelSelection();
    showToast(copy[language].accountDeleted);
    render();
  } catch (error) {
    showToast(normalizeError(error));
  }
}

function getSelectedPlatform() {
  return platforms.find((item) => item.id === selectedPlatformId) || platforms[0] || fallbackPlatforms[0];
}

function getSelectedAccount() {
  return selectedAccountId ? accounts.find((item) => item.id === selectedAccountId) || null : null;
}

function worksForCurrentSelection(selectedAccount: ChannelAccount | null, platformId: string): ChannelWork[] {
  if (selectedAccount && supportsWorksPages(selectedAccount.platformId)) {
    return worksForAccount(selectedAccount.id, selectedAccountWorkType(selectedAccount.id));
  }
  const mockWorks = mockWorksForAccounts(accounts);
  if (selectedAccount) return mockWorks.filter((item) => item.accountId === selectedAccount.id);
  return mockWorks.filter((item) => item.platformId === platformId);
}

function worksForAccount(accountId: string, workType: ChannelWorkType = "video"): ChannelWork[] {
  const pages = accountWorksPages.get(worksStateKey(accountId, workType)) || [];
  const works = pages.flatMap((page) => page.works || []);
  const seen = new Set<string>();
  return works.filter((work) => {
    if (seen.has(work.id)) return false;
    seen.add(work.id);
    return true;
  });
}

function isWorksLoading(accountId: string) {
  const prefix = `${worksStateKey(accountId, selectedAccountWorkType(accountId))}:`;
  return Array.from(loadingWorksPageIds).some((key) => key.startsWith(prefix));
}

function supportsAccountContent(platformId: string) {
  return platformId === "xiaohongshu" || platformId === "wechat-channels" || platformId === "douyin" || platformId === "bilibili" || platformId === "kuaishou";
}

function supportsWorksPages(platformId: string) {
  return platformId === "xiaohongshu" || platformId === "wechat-channels" || platformId === "douyin" || platformId === "bilibili" || platformId === "kuaishou";
}

function selectedAccountWorkType(accountId: string): ChannelWorkType {
  return workTypeByAccount.get(accountId) || "video";
}

function selectedAccountHasWorksPage() {
  const account = getSelectedAccount();
  if (!account) return false;
  const workType = selectedAccountWorkType(account.id);
  return (accountWorksPages.get(worksStateKey(account.id, workType)) || []).length > 0;
}

function selectedAccountOverviewPeriod(account: ChannelAccount): OverviewPeriod {
  return overviewPeriodByAccount.get(account.id) || (account.platformId === "douyin" ? 1 : account.platformId === "bilibili" ? BILIBILI_TOTAL_PERIOD : 7);
}

function readOverviewPeriod(value: string | undefined): OverviewPeriod {
  if (value === "1") return 1;
  if (value === "30") return 30;
  if (value === "90") return 90;
  if (value === "36500") return 36500;
  if (value === "65535") return 65535;
  return 7;
}

function worksStateKey(accountId: string, workType: ChannelWorkType) {
  return `${accountId}:${workType}`;
}

function deleteWorksStateForAccount(accountId: string) {
  Array.from(accountWorksPages.keys()).forEach((key) => {
    if (key === accountId || key.startsWith(`${accountId}:`)) {
      accountWorksPages.delete(key);
    }
  });
}

function syncChannelSelection(options: { preferFirstWithAccounts?: boolean; expandSelected?: boolean } = {}) {
  if (!platforms.length) return;

  const selectedAccount = getSelectedAccount();
  if (selectedAccount) {
    selectedPlatformId = selectedAccount.platformId;
    if (options.expandSelected) {
      expandedPlatformIds.add(selectedAccount.platformId);
    }
    return;
  }

  selectedAccountId = null;

  if (options.preferFirstWithAccounts) {
    const firstPlatformWithAccounts = platforms.find((platform) =>
      accounts.some((account) => account.platformId === platform.id),
    );
    if (firstPlatformWithAccounts) {
      selectedPlatformId = firstPlatformWithAccounts.id;
    }
  }

  if (!platforms.some((item) => item.id === selectedPlatformId)) {
    selectedPlatformId = platforms[0].id;
  }

  if (options.expandSelected) {
    expandedPlatformIds.add(selectedPlatformId);
  }
}

function isContentTab(value: string | undefined): value is ContentTab {
  return value === "overview" || value === "works" || value === "comments" || value === "data";
}

function updateChannelSearch(value: string) {
  channelSearchQuery = value;
  render();
  window.requestAnimationFrame(() => {
    const search = document.querySelector<HTMLInputElement>("[data-channel-search]");
    if (!search) return;
    search.focus();
    search.setSelectionRange(search.value.length, search.value.length);
  });
}

function togglePlatformExpanded(platformId: string) {
  if (!accounts.some((account) => account.platformId === platformId)) return;
  if (expandedPlatformIds.has(platformId)) {
    expandedPlatformIds.delete(platformId);
  } else {
    expandedPlatformIds.add(platformId);
  }
}

function visibleChannelPlatforms() {
  const query = normalizedChannelSearch();
  if (!query) return platforms;

  return platforms.filter((platform) => {
    if (matchesPlatformSearch(platform, query)) return true;
    return accounts.some((account) => account.platformId === platform.id && matchesAccountSearch(account, query));
  });
}

function platformTree(platform: PlatformInfo) {
  const query = normalizedChannelSearch();
  const platformMatches = query ? matchesPlatformSearch(platform, query) : false;
  const allPlatformAccounts = accounts.filter((item) => item.platformId === platform.id);
  const platformAccounts = query && !platformMatches
    ? allPlatformAccounts.filter((account) => matchesAccountSearch(account, query))
    : allPlatformAccounts;
  const count = allPlatformAccounts.length;
  const active = !selectedAccountId && platform.id === selectedPlatformId && activeMenuId === "channels";

  return renderPlatformTreeItem({
    platform,
    count,
    active,
    expanded: query ? platformAccounts.length > 0 : expandedPlatformIds.has(platform.id),
    canToggle: !query,
    accountsHtml: platformAccounts.map((account) => accountNavItem(account)).join(""),
  });
}

function normalizedChannelSearch() {
  return channelSearchQuery.trim().toLowerCase();
}

function matchesPlatformSearch(platform: PlatformInfo, query: string) {
  return platform.name.toLowerCase().includes(query);
}

function matchesAccountSearch(account: ChannelAccount, query: string) {
  return account.nickname.toLowerCase().includes(query);
}

function accountNavItem(account: ChannelAccount) {
  const platform = platforms.find((item) => item.id === account.platformId);

  return renderAccountNavItem({
    account,
    text: copy[language],
    platform,
    active: selectedAccountId === account.id,
    isUnavailable: account.status !== "active",
    fallbackAvatar: accountInitials(account.nickname),
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
  return normalizeErrorMessage(
    error,
    language === "zh" ? "操作失败，请稍后重试。" : "Operation failed. Please try again.",
  );
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number, message: string): Promise<T> {
  let timer: number | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = window.setTimeout(() => reject(new Error(message)), timeoutMs);
  });

  return Promise.race([
    promise.finally(() => {
      if (timer) window.clearTimeout(timer);
    }),
    timeout,
  ]);
}
