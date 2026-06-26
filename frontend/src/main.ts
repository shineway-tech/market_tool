import { listen } from "@tauri-apps/api/event";
import "./styles.css";
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
} from "./domain/types";
import { fallbackPlatforms } from "./domain/platforms";
import { isQrAuth } from "./domain/auth-task";
import { releaseHistoryForLanguage } from "./domain/releases";
import { copy } from "./i18n/copy";
import { readStoredMode } from "./utils/storage";
import { normalizeError as normalizeErrorMessage } from "./utils/errors";
import {
  clearFieldError,
  clearFormFieldErrors,
  formValue,
  reportFieldError,
  reportNamedFieldError,
} from "./utils/forms";
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
import { renderAccountDropdown } from "./ui/user-menu";
import { renderAuthPage } from "./pages/auth";
import { renderChannelsPage } from "./pages/channels";
import { renderFeedbackPage } from "./pages/feedback";
import { renderPasswordPage } from "./pages/password";
import { renderProfilePage } from "./pages/profile";
import { renderReleasesPage } from "./pages/releases";
import { renderSettingsPage } from "./pages/settings";
import { renderAppShell } from "./app/shell";
import { API_BASE_URL, AUTH_TOKEN_KEY, INPUT_HINTS_OFF } from "./config/app";
import { UpdateController } from "./features/updater";
import { accountBackendPayload, mirrorAccounts, upsertAccount } from "./features/channel-sync";
import { requestApi, type ApiRequestOptions } from "./services/api";
import { invokeCommand } from "./services/tauri-commands";

let platforms: PlatformInfo[] = fallbackPlatforms;
let accounts: ChannelAccount[] = [];
let selectedPlatformId = "xiaohongshu";
let activeMenuId: MenuId = "channels";
let userMenuOpen = false;
let activeAuthTask: StartLoginResponse | null = null;
let activeAuthMessage = "";
let toastTimer: number | undefined;
let authPollTimer: number | undefined;
let refreshingAccountIds = new Set<string>();
let refreshingPlatformIds = new Set<string>();
let openingHomepageIds = new Set<string>();
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

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root missing");
}

const appRoot = app;

void bindTauriAccountEvents();
void boot();

async function bindTauriAccountEvents() {
  try {
    await listen<ChannelAccount>("channel-account-updated", (event) => {
      void applyAccountUpdate(event.payload, { mirror: true, rerender: true });
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

  await mirrorAccounts(items, (account) => apiRequest<ChannelAccount>("/v1/channel/accounts", {
    method: "POST",
    body: accountBackendPayload(account),
  }));
}

async function applyAccountUpdate(
  updated: ChannelAccount,
  options: { mirror?: boolean; rerender?: boolean } = {},
) {
  if (currentUser?.id && updated.userId && updated.userId !== currentUser.id) {
    return;
  }

  accounts = upsertAccount(accounts, updated, currentUser?.id);

  if (options.mirror && authToken) {
    await apiRequest<ChannelAccount>("/v1/channel/accounts", {
      method: "POST",
      body: accountBackendPayload(updated),
    }).catch(() => undefined);
  }

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
    item.addEventListener("click", () => {
      const nextPlatformId = item.dataset.platform || selectedPlatformId;
      selectedPlatformId = nextPlatformId;
      activeMenuId = "channels";
      userMenuOpen = false;
      render();
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
          body: accountBackendPayload(result.account),
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
    await applyAccountUpdate(updated, { mirror: true });
    toastMessage = copy[language].accountRefreshed;
  } catch (error) {
    const expired = await markAccountUnavailable(accountId);
    if (expired) {
      await applyAccountUpdate(expired, { mirror: true });
    }
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
      await applyAccountUpdate(updated, { mirror: true });
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
    const updated = await invokeCommand<ChannelAccount>("open_account_homepage", {
      accountId,
      userId: requireCurrentUserId(),
    });
    await applyAccountUpdate(updated, { mirror: true });
  } catch (error) {
    showToast(normalizeError(error));
  } finally {
    openingHomepageIds.delete(accountId);
    render();
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

function platformItem(platform: PlatformInfo) {
  const count = accounts.filter((item) => item.platformId === platform.id).length;
  const active = platform.id === selectedPlatformId && activeMenuId === "channels";

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
    isOpeningHomepage: openingHomepageIds.has(account.id),
    isUnavailable: account.status !== "active",
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
