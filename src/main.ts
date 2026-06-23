import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import {
  siBilibili,
  siKuaishou,
  siTiktok,
  siXiaohongshu,
  type SimpleIcon,
} from "simple-icons";
import "./styles.css";

declare const __APP_VERSION__: string;

type AuthMode = "relay" | "oAuth";
type HttpMethod = "GET" | "POST";
type AccountStatus = "active" | "expired" | "pending";
type SidebarMenuId =
  | "channels"
  | "settings"
  | "releases"
  | "feedback";
type MenuId = SidebarMenuId | "profile" | "password";
type LanguageMode = "zh" | "en";
type ThemeMode = "dark" | "light";
type LoginTarget = "home" | "creator";
type AuthViewMode = "login" | "register";
type UpdateStatus = "idle" | "checking" | "latest" | "available" | "downloading" | "installed" | "error";

interface ApiResponse<T> {
  err_code: number;
  err_msg: string;
  data: T;
}

interface AuthUser {
  id: string;
  account: string;
  nickname: string;
  status: string;
  lastLoginAt?: string | null;
}

interface CaptchaResponse {
  captchaId: string;
  image: string;
  expiresAt: string;
}

interface AuthSession {
  token: string;
  tokenName: string;
  expiresIn: number;
  user: AuthUser;
}

interface PlatformInfo {
  id: string;
  name: string;
  slug: string;
  color: string;
  description: string;
  supportsBuiltinOauth: boolean;
}

interface RelaySettings {
  enabled: boolean;
  serverUrl: string;
  apiKey: string;
}

interface PlatformAuthSettings {
  platformId: string;
  mode: AuthMode;
  relayPath: string;
  relayMethod: HttpMethod;
  authUrl: string;
  tokenUrl: string;
  profileUrl: string;
  clientId: string;
  clientSecret: string;
  scopes: string[];
}

interface AuthSettings {
  relay: RelaySettings;
  platforms: PlatformAuthSettings[];
}

interface ChannelAccount {
  id: string;
  userId?: string;
  platformId: string;
  uid: string;
  nickname: string;
  avatar: string;
  followers?: number | null;
  status: AccountStatus;
  relayAccountRef?: string | null;
  createdAt: string;
  updatedAt: string;
  lastSyncAt?: string | null;
}

interface Bootstrap {
  platforms: PlatformInfo[];
  accounts: ChannelAccount[];
  settings: AuthSettings;
  callbackBaseUrl?: string | null;
}

interface StartLoginResponse {
  taskId: string;
  url: string;
  callbackUrl: string;
  mode: AuthMode;
  authType?: "oauth" | "qrcode" | string;
  sessionId?: string | null;
  expiresAt?: string | null;
  instructions?: string | null;
}

interface AuthTaskStatus {
  taskId: string;
  status: "pending" | "success" | "failed" | "unknown";
  account?: ChannelAccount | null;
  message?: string | null;
}

interface UpdateState {
  status: UpdateStatus;
  availableVersion?: string;
  notes?: string;
  downloadedBytes: number;
  contentLength?: number;
  error?: string;
}

const fallbackPlatforms: PlatformInfo[] = [
  {
    id: "xiaohongshu",
    name: "小红书",
    slug: "XHS",
    color: "#ff2442",
    description: "添加并管理多个小红书账号。",
    supportsBuiltinOauth: true,
  },
  {
    id: "wechat-channels",
    name: "视频号",
    slug: "WX",
    color: "#ff9f2e",
    description: "添加并管理多个微信视频号账号。",
    supportsBuiltinOauth: true,
  },
  {
    id: "douyin",
    name: "抖音",
    slug: "DY",
    color: "#111111",
    description: "添加并管理多个抖音账号。",
    supportsBuiltinOauth: true,
  },
  {
    id: "bilibili",
    name: "哔哩哔哩",
    slug: "BILI",
    color: "#00a1d6",
    description: "添加并管理多个 B 站账号。",
    supportsBuiltinOauth: true,
  },
  {
    id: "kuaishou",
    name: "快手",
    slug: "KS",
    color: "#ff4906",
    description: "添加并管理多个快手账号。",
    supportsBuiltinOauth: true,
  },
];

type PlatformIcon = SimpleIcon | { title: string; hex: string; markup: string };

const wechatChannelsIcon: PlatformIcon = {
  title: "视频号",
  hex: "ff9f2e",
  markup: '<svg viewBox="0 0 24 24" role="img" aria-label="视频号"><path d="M11.2 12.1C9.3 7.6 6.3 4.3 4.1 5.8 1.8 7.4 4.4 16 8.9 15.6c1.4-.1 2.2-1.4 2.3-3.5ZM12.8 12.1c1.9-4.5 4.9-7.8 7.1-6.3 2.3 1.6-.3 10.2-4.8 9.8-1.4-.1-2.2-1.4-2.3-3.5Z" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round"/></svg>',
};

const platformIcons: Record<string, PlatformIcon> = {
  xiaohongshu: siXiaohongshu,
  "wechat-channels": wechatChannelsIcon,
  douyin: siTiktok,
  bilibili: siBilibili,
  kuaishou: siKuaishou,
};

const API_BASE_URL = "https://market-api.honeykid.cn";
const AUTH_TOKEN_KEY = "marketing-master-api-token";
const AUTO_UPDATE_KEY = "marketing-master-auto-update";
const APP_VERSION = __APP_VERSION__;
const INPUT_HINTS_OFF = 'autocomplete="off" autocorrect="off" autocapitalize="none" spellcheck="false" data-lpignore="true" data-1p-ignore="true"';

const copy = {
  zh: {
    appName: "营销大师",
    loginTitle: "登录营销大师",
    registerTitle: "创建账号",
    account: "账号",
    password: "密码",
    nickname: "昵称",
    captcha: "验证码",
    loginSubmit: "登录",
    registerSubmit: "注册并登录",
    switchToRegister: "创建新账号",
    switchToLogin: "已有账号，去登录",
    logout: "退出登录",
    authAccountPlaceholder: "请输入账号",
    authPasswordPlaceholder: "请输入密码",
    authNicknamePlaceholder: "用于界面显示",
    authCaptchaPlaceholder: "输入验证码",
    captchaRefresh: "刷新验证码",
    loginRequired: "请先登录后使用客户端。",
    loginSuccess: "登录成功。",
    registerSuccess: "注册成功。",
    search: "搜索渠道 / 账号",
    localClient: "本地客户端",
    loginAccount: "登录账号",
    homepage: "主页",
    refresh: "刷新",
    refreshing: "刷新中",
    refreshPlatform: "刷新当前平台",
    channelsTitle: "渠道管理",
    channelsDesc: "连接内容平台账号，用于账号授权和数据同步。",
    platformPanel: "渠道平台",
    accountPanel: "已授权账号",
    availableAccount: "可用账号",
    totalFans: "粉丝总数",
    accountUnit: "个账号",
    notConnected: "未连接",
    noAccountPrefix: "还没有授权的",
    noAccountSuffix: "账号",
    noAccountDesc: "点击左侧渠道列表后的加号完成授权；同一个平台可以重复添加多个账号。",
    fansPending: "粉丝数待同步",
    notSynced: "尚未同步",
    syncedAt: "同步于",
    statusActive: "正常",
    statusExpired: "已过期",
    statusPending: "处理中",
    accountRefreshed: "账号状态已刷新。",
    platformRefreshed: "当前平台账号已刷新。",
    accountDeleted: "账号已删除。",
    authOpened: "授权窗口已打开，请在打开的窗口中完成登录。",
    authQrOpened: "请使用对应平台 App 扫码完成授权。",
    authDone: "账号授权成功，已同步到渠道列表。",
    authFailed: "账号授权没有完成。",
    authWaiting: "还没有收到平台授权结果，请确认授权窗口是否完成登录。",
    authTitle: "等待平台授权",
    authDesc: "授权窗口已经打开。完成登录后，客户端会自动同步账号列表。",
    authQrDesc: "请使用对应平台 App 扫描二维码，并在手机端完成账号授权。",
    checkStatus: "检查状态",
    later: "稍后再说",
    feedbackTitle: "意见反馈",
    feedbackContent: "反馈内容",
    feedbackContact: "联系方式",
    feedbackContentPlaceholder: "请输入你的意见或遇到的问题",
    feedbackContactPlaceholder: "手机号 / 微信 / 邮箱（选填）",
    feedbackSubmit: "提交反馈",
    feedbackSubmitted: "反馈已提交。",
    settingsTitle: "系统设置",
    settingsDesc: "调整客户端的语言、主题和本地展示偏好。",
    profileSettings: "个人信息",
    profileSettingsDesc: "修改当前登录账号在客户端中显示的资料。",
    accountReadonly: "账号不可修改",
    saveProfile: "保存资料",
    profileSaved: "个人信息已更新。",
    passwordSettings: "修改密码",
    passwordSettingsDesc: "输入当前密码后设置新的登录密码。",
    currentPassword: "当前密码",
    newPassword: "新密码",
    confirmPassword: "确认新密码",
    changePassword: "修改密码",
    passwordChanged: "密码已更新，请使用新密码登录。",
    passwordMismatch: "两次输入的新密码不一致",
    language: "语言",
    chinese: "中文",
    english: "英文",
    theme: "主题",
    dark: "深色",
    light: "浅色",
    settingsSaved: "设置已更新。",
    releasesTitle: "更新公告",
    releasesCurrent: "当前功能",
    versionLabel: "当前版本",
    releaseDate: "发布日期",
    autoUpdate: "自动更新",
    autoUpdateOn: "已开启",
    autoUpdateOff: "已关闭",
    checkUpdate: "检查更新",
    checkingUpdate: "检查中",
    installUpdate: "立即更新",
    showReleaseContent: "展开更新内容",
    hideReleaseContent: "收起更新内容",
    downloadingUpdate: "更新中",
    latestVersion: "已是最新版本",
    updateAvailable: "发现新版本",
    updateInstalled: "更新已安装，正在重启。",
    updateFailed: "更新失败，请稍后重试。",
    updateUnavailable: "更新检查不可用。",
    updateProgress: "下载进度",
    menu: {
      channels: "渠道管理",
      settings: "系统设置",
      releases: "更新公告",
      feedback: "意见反馈",
    } satisfies Record<SidebarMenuId, string>,
  },
  en: {
    appName: "Marketing Master",
    loginTitle: "Sign In",
    registerTitle: "Create Account",
    account: "Account",
    password: "Password",
    nickname: "Nickname",
    captcha: "Captcha",
    loginSubmit: "Sign In",
    registerSubmit: "Create and Sign In",
    switchToRegister: "Create account",
    switchToLogin: "Already have an account",
    logout: "Sign Out",
    authAccountPlaceholder: "Enter account",
    authPasswordPlaceholder: "Enter password",
    authNicknamePlaceholder: "Display name",
    authCaptchaPlaceholder: "Captcha",
    captchaRefresh: "Refresh captcha",
    loginRequired: "Sign in to use the client.",
    loginSuccess: "Signed in.",
    registerSuccess: "Account created.",
    search: "Search channels / accounts",
    localClient: "Local client",
    loginAccount: "Sign in",
    homepage: "Homepage",
    refresh: "Refresh",
    refreshing: "Refreshing",
    refreshPlatform: "Refresh platform",
    channelsTitle: "Channel Management",
    channelsDesc: "Connect content platform accounts for authorization and data sync.",
    platformPanel: "Platforms",
    accountPanel: "Authorized Accounts",
    availableAccount: "Available",
    totalFans: "Total Followers",
    accountUnit: "accounts",
    notConnected: "Not connected",
    noAccountPrefix: "No authorized",
    noAccountSuffix: "accounts yet",
    noAccountDesc: "Use the plus button in the platform list to authorize accounts. You can add multiple accounts per platform.",
    fansPending: "Followers pending",
    notSynced: "Not synced",
    syncedAt: "Synced",
    statusActive: "Active",
    statusExpired: "Expired",
    statusPending: "Pending",
    accountRefreshed: "Account status refreshed.",
    platformRefreshed: "Platform accounts refreshed.",
    accountDeleted: "Account deleted.",
    authOpened: "Authorization window opened. Finish sign-in in the opened window.",
    authQrOpened: "Scan the QR code in the platform app to finish authorization.",
    authDone: "Account authorized and synced.",
    authFailed: "Account authorization did not finish.",
    authWaiting: "No authorization result yet. Check whether sign-in is complete.",
    authTitle: "Waiting for Authorization",
    authDesc: "The authorization window is open. Accounts will sync automatically after sign-in.",
    authQrDesc: "Scan the QR code in the platform app and finish account authorization on your phone.",
    checkStatus: "Check status",
    later: "Later",
    feedbackTitle: "Feedback",
    feedbackContent: "Feedback",
    feedbackContact: "Contact",
    feedbackContentPlaceholder: "Describe your feedback or issue",
    feedbackContactPlaceholder: "Phone / WeChat / Email (optional)",
    feedbackSubmit: "Submit Feedback",
    feedbackSubmitted: "Feedback submitted.",
    settingsTitle: "System Settings",
    settingsDesc: "Adjust client language, theme, and local display preferences.",
    profileSettings: "Profile",
    profileSettingsDesc: "Update the display details for the current account.",
    accountReadonly: "Account cannot be changed",
    saveProfile: "Save Profile",
    profileSaved: "Profile updated.",
    passwordSettings: "Change Password",
    passwordSettingsDesc: "Enter your current password before setting a new one.",
    currentPassword: "Current Password",
    newPassword: "New Password",
    confirmPassword: "Confirm Password",
    changePassword: "Change Password",
    passwordChanged: "Password updated. Use the new password next time.",
    passwordMismatch: "The new passwords do not match",
    language: "Language",
    chinese: "Chinese",
    english: "English",
    theme: "Theme",
    dark: "Dark",
    light: "Light",
    settingsSaved: "Settings updated.",
    releasesTitle: "Release Notes",
    releasesCurrent: "Current Features",
    versionLabel: "Current Version",
    releaseDate: "Release Date",
    autoUpdate: "Auto Update",
    autoUpdateOn: "On",
    autoUpdateOff: "Off",
    checkUpdate: "Check",
    checkingUpdate: "Checking",
    installUpdate: "Update Now",
    showReleaseContent: "Show Updates",
    hideReleaseContent: "Hide Updates",
    downloadingUpdate: "Updating",
    latestVersion: "You are up to date",
    updateAvailable: "New version available",
    updateInstalled: "Update installed. Relaunching.",
    updateFailed: "Update failed. Please try again.",
    updateUnavailable: "Update check unavailable.",
    updateProgress: "Download progress",
    menu: {
      channels: "Channels",
      settings: "Settings",
      releases: "Updates",
      feedback: "Feedback",
    } satisfies Record<SidebarMenuId, string>,
  },
};

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
let authError = "";
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
      authError = normalizeError(error);
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

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

function requireCurrentUserId() {
  if (!currentUser?.id) {
    throw new Error(copy[language].loginRequired);
  }
  return currentUser.id;
}

async function apiRequest<T>(
  path: string,
  options: {
    method?: string;
    body?: Record<string, unknown>;
    skipAuth?: boolean;
  } = {},
): Promise<T> {
  const headers: Record<string, string> = {};

  if (authToken && !options.skipAuth) {
    headers["X-Token"] = authToken;
  }

  if (options.body) {
    headers["Content-Type"] = "application/json";
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    method: options.method || "GET",
    headers,
    body: options.body ? JSON.stringify(options.body) : undefined,
  });
  const payload = (await response.json()) as ApiResponse<T>;

  if (!response.ok || payload.err_code !== 0) {
    throw new Error(payload.err_msg || `HTTP ${response.status}`);
  }

  return payload.data;
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
  const text = copy[language];
  const isRegister = authViewMode === "register";

  return `
    <div class="auth-shell theme-${theme}">
      <section class="auth-card">
        <div class="auth-brand">
          <div class="brand-mark" aria-hidden="true">M</div>
          <div>
            <strong>${text.appName}</strong>
          </div>
        </div>
        <form class="login-form" data-auth-form="${authViewMode}">
          <div class="auth-form-head">
            <h1>${isRegister ? text.registerTitle : text.loginTitle}</h1>
          </div>
          <label>
            <span>${text.account}</span>
            <input name="account" ${INPUT_HINTS_OFF} placeholder="${text.authAccountPlaceholder}" value="${escapeAttribute(authDraft.account)}" required />
          </label>
          ${
            isRegister
              ? `<label>
                  <span>${text.nickname}</span>
                  <input name="nickname" ${INPUT_HINTS_OFF} placeholder="${text.authNicknamePlaceholder}" value="${escapeAttribute(authDraft.nickname)}" />
                </label>`
              : ""
          }
          <label>
            <span>${text.password}</span>
            <input name="password" type="password" ${INPUT_HINTS_OFF} placeholder="${text.authPasswordPlaceholder}" value="${escapeAttribute(authDraft.password)}" required />
          </label>
          <label>
            <span>${text.captcha}</span>
            <div class="captcha-row">
              <input name="captchaCode" ${INPUT_HINTS_OFF} placeholder="${text.authCaptchaPlaceholder}" value="${escapeAttribute(authDraft.captchaCode)}" required />
              <button class="captcha-img" type="button" data-auth-action="refresh-captcha" title="${text.captchaRefresh}">
                ${captcha ? `<img src="${escapeAttribute(captcha.image)}" alt="${text.captcha}" />` : icon("refresh")}
              </button>
            </div>
          </label>
          <button class="primary-btn auth-submit" type="submit" ${authBusy ? "disabled" : ""}>
            <span class="auth-submit-icon ${authBusy ? "is-visible" : ""}">${icon("refresh")}</span>
            <span>${isRegister ? text.registerSubmit : text.loginSubmit}</span>
          </button>
          <button class="auth-switch" type="button" data-auth-action="${isRegister ? "show-login" : "show-register"}">
            ${isRegister ? text.switchToLogin : text.switchToRegister}
          </button>
        </form>
      </section>
    </div>
  `;
}

function accountDropdown() {
  const text = copy[language];
  const user = currentUser;
  if (!user) return "";

  return `
    <div class="user-dropdown">
      <div class="user-dropdown-head">
        <strong>${escapeHtml(user.nickname)}</strong>
        <span>${escapeHtml(user.account)}</span>
      </div>
      <button class="user-menu-item" type="button" data-menu="profile">
        ${icon("user")}
        <span>${text.profileSettings}</span>
      </button>
      <button class="user-menu-item" type="button" data-menu="password">
        ${icon("lock")}
        <span>${text.passwordSettings}</span>
      </button>
      <button class="user-menu-item danger" type="button" data-action="logout">
        ${icon("logout")}
        <span>${text.logout}</span>
      </button>
    </div>
  `;
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
  const text = copy[language];
  const selectedPlatform = getSelectedPlatform();
  const selectedAccounts = accounts.filter((item) => item.platformId === selectedPlatform.id);
  const connectedCount = accounts.length;
  const activeAccountCount = selectedAccounts.filter((item) => item.status === "active").length;
  const platformRefreshing = refreshingPlatformIds.has(selectedPlatform.id);

  return `
    <section class="channel-head">
      <div>
        <h1>${text.channelsTitle}</h1>
      </div>
    </section>

    <section class="channel-layout">
      <aside class="platform-panel">
        <div class="panel-title">
          <span>${text.platformPanel}</span>
          <strong>${connectedCount}</strong>
        </div>
        <div class="platform-list">
          ${platforms.map((platform) => platformItem(platform)).join("")}
        </div>
      </aside>

      <section class="account-panel">
        <div class="account-panel-head">
          <div class="selected-platform">
            ${platformLogo(selectedPlatform, "large")}
            <div>
              <h2>${selectedPlatform.name}</h2>
            </div>
          </div>
          <div class="account-actions">
            <button class="ghost-btn ${platformRefreshing ? "is-loading" : ""}" type="button" data-action="refresh-platform" ${platformRefreshing ? "disabled" : ""}>${icon("refresh")}${platformRefreshing ? text.refreshing : text.refresh}</button>
          </div>
        </div>
        <div class="account-summary">
          <div><span>${text.accountPanel}</span><strong>${selectedAccounts.length}</strong></div>
          <div><span>${text.availableAccount}</span><strong>${activeAccountCount}</strong></div>
          <div><span>${text.totalFans}</span><strong>${formatFollowersTotal(selectedAccounts)}</strong></div>
        </div>
        <div class="account-list">
          ${
            selectedAccounts.length
              ? selectedAccounts.map((account) => accountItem(account)).join("")
              : emptyAccounts(selectedPlatform)
          }
        </div>
      </section>
    </section>
  `;
}

function settingsPage() {
  const text = copy[language];
  return `
    <section class="page-head">
      <div>
        <h1>${text.settingsTitle}</h1>
      </div>
    </section>
    <section class="settings-grid-page settings-grid-compact">
      <article class="settings-card">
        <div>
          <h2>${text.language}</h2>
        </div>
        <select data-system-setting="language" aria-label="${text.language}">
          <option value="zh" ${language === "zh" ? "selected" : ""}>${text.chinese}</option>
          <option value="en" ${language === "en" ? "selected" : ""}>${text.english}</option>
        </select>
      </article>
      <article class="settings-card">
        <div>
          <h2>${text.theme}</h2>
        </div>
        <select data-system-setting="theme" aria-label="${text.theme}">
          <option value="dark" ${theme === "dark" ? "selected" : ""}>${text.dark}</option>
          <option value="light" ${theme === "light" ? "selected" : ""}>${text.light}</option>
        </select>
      </article>
    </section>
  `;
}

function profilePage() {
  const text = copy[language];
  const profileNickname = profileDraft.nickname || currentUser?.nickname || "";
  return `
    <section class="page-head">
      <div>
        <h1>${text.profileSettings}</h1>
      </div>
    </section>
    <section class="single-form-page">
      <article class="settings-card settings-card-form">
        <form class="settings-form" data-settings-form="profile">
          <div class="form-grid">
            <label>
              <span>${text.account}</span>
              <input name="account" ${INPUT_HINTS_OFF} value="${escapeAttribute(currentUser?.account || "")}" readonly aria-label="${text.accountReadonly}" />
            </label>
            <label>
              <span>${text.nickname}</span>
              <input name="nickname" ${INPUT_HINTS_OFF} maxlength="32" value="${escapeAttribute(profileNickname)}" required />
            </label>
          </div>
          <div class="settings-form-actions">
            <button class="primary-btn" type="submit" ${profileBusy ? "disabled" : ""}>${icon("save")}${text.saveProfile}</button>
          </div>
        </form>
      </article>
    </section>
  `;
}

function passwordPage() {
  const text = copy[language];
  return `
    <section class="page-head">
      <div>
        <h1>${text.passwordSettings}</h1>
      </div>
    </section>
    <section class="single-form-page">
      <article class="settings-card settings-card-form">
        <form class="settings-form" data-settings-form="password">
          <div class="form-grid compact">
            <label>
              <span>${text.currentPassword}</span>
              <input name="currentPassword" type="password" ${INPUT_HINTS_OFF} minlength="6" maxlength="64" value="${escapeAttribute(passwordDraft.currentPassword)}" required />
            </label>
            <label>
              <span>${text.newPassword}</span>
              <input name="newPassword" type="password" ${INPUT_HINTS_OFF} minlength="6" maxlength="64" value="${escapeAttribute(passwordDraft.newPassword)}" required />
            </label>
            <label>
              <span>${text.confirmPassword}</span>
              <input name="confirmPassword" type="password" ${INPUT_HINTS_OFF} minlength="6" maxlength="64" value="${escapeAttribute(passwordDraft.confirmPassword)}" required />
            </label>
          </div>
          <div class="settings-form-actions">
            <button class="primary-btn" type="submit" ${passwordBusy ? "disabled" : ""}>${icon("lock")}${text.changePassword}</button>
          </div>
        </form>
      </article>
    </section>
  `;
}

function feedbackPage() {
  const text = copy[language];
  return `
    <section class="page-head">
      <div>
        <h1>${text.feedbackTitle}</h1>
      </div>
    </section>
    <section class="single-form-page">
      <article class="settings-card settings-card-form">
        <form class="settings-form" data-feedback-form>
          <label>
            <span>${text.feedbackContent}</span>
            <textarea name="content" ${INPUT_HINTS_OFF} maxlength="2000" placeholder="${text.feedbackContentPlaceholder}" required>${escapeHtml(feedbackDraft.content)}</textarea>
          </label>
          <label>
            <span>${text.feedbackContact}</span>
            <input name="contact" ${INPUT_HINTS_OFF} maxlength="191" placeholder="${text.feedbackContactPlaceholder}" value="${escapeAttribute(feedbackDraft.contact)}" />
          </label>
          <div class="settings-form-actions">
            <button class="primary-btn" type="submit" ${feedbackBusy ? "disabled" : ""}>${icon("send")}${text.feedbackSubmit}</button>
          </div>
        </form>
      </article>
    </section>
  `;
}

function releasesPage() {
  const text = copy[language];
  const entries = releaseHistory();
  const status = updateStatusText();
  const progress = updateProgressPercent();
  const isChecking = updateState.status === "checking";
  const isDownloading = updateState.status === "downloading";
  const canInstall = updateState.status === "available" && Boolean(pendingUpdate);
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

      ${entries.map((entry) => releaseHistoryCard(entry)).join("")}
    </section>
  `;
}

function releaseHistoryCard(entry: ReturnType<typeof releaseHistory>[number]) {
  const text = copy[language];
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

function releaseHistory() {
  if (language === "en") {
    return [
      {
        version: "1.0.0",
        date: "2026.06.23",
        icon: "spark",
        sections: [
          {
            icon: "lock",
            title: "Account Access",
            items: ["Password sign-in and registration", "Captcha verification", "Profile editing and password change"],
          },
          {
            icon: "layers",
            title: "Channel Management",
            items: ["Xiaohongshu, WeChat Channels, Douyin, Bilibili, and Kuaishou", "Multiple accounts per platform", "Avatar, nickname, followers, and status display"],
          },
          {
            icon: "refresh",
            title: "Account Operations",
            items: ["Refresh account data", "Delete connected accounts", "Open the platform creator homepage"],
          },
          {
            icon: "settings",
            title: "Client Settings",
            items: ["Chinese and English language switch", "Dark and light themes", "Local JSON configuration"],
          },
          {
            icon: "message",
            title: "Feedback",
            items: ["Submit feedback from the client", "Store feedback in the local service"],
          },
        ],
      },
    ];
  }

  return [
    {
      version: "1.0.0",
      date: "2026.06.23",
      icon: "spark",
      sections: [
        {
          icon: "lock",
          title: "账号体系",
          items: ["账号密码登录与注册", "验证码校验", "个人信息和密码修改"],
        },
        {
          icon: "layers",
          title: "渠道管理",
          items: ["小红书、视频号、抖音、哔哩哔哩、快手授权", "同一平台支持多个账号", "展示头像、昵称、粉丝数和状态"],
        },
        {
          icon: "refresh",
          title: "账号操作",
          items: ["刷新账号数据", "删除已授权账号", "打开对应平台创作者主页"],
        },
        {
          icon: "settings",
          title: "客户端设置",
          items: ["中文 / 英文切换", "深色 / 浅色主题", "本地 JSON 配置"],
        },
        {
          icon: "message",
          title: "意见反馈",
          items: ["客户端内提交反馈", "反馈内容保存到本地服务"],
        },
      ],
    },
  ];
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
        authError = "";
        void loadCaptchaAndRender();
      }

      if (action === "show-login") {
        captureAuthDraftFromForm();
        authViewMode = "login";
        authError = "";
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
  authError = "";
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
    authError = "";
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
    authError = normalizeError(error);
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
  authError = "";
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

function defaultPlatformSettings(): PlatformAuthSettings[] {
  return [
    defaultPlatformSetting("xiaohongshu", "plat/xhs/auth/url/pc"),
    defaultPlatformSetting("wechat-channels", "plat/wxSph/auth/url/pc"),
    defaultPlatformSetting("douyin", "plat/douyin/auth/url"),
    defaultPlatformSetting("bilibili", "plat/bilibili/auth/url/pc"),
    defaultPlatformSetting("kuaishou", "plat/kwai/auth/url/pc"),
  ];
}

function defaultPlatformSetting(platformId: string, relayPath: string): PlatformAuthSettings {
  return {
    platformId,
    mode: "relay",
    relayPath,
    relayMethod: "GET",
    authUrl: "",
    tokenUrl: "",
    profileUrl: "",
    clientId: "",
    clientSecret: "",
    scopes: [],
  };
}

function platformItem(platform: PlatformInfo) {
  const text = copy[language];
  const count = accounts.filter((item) => item.platformId === platform.id).length;
  const active = platform.id === selectedPlatformId && activeMenuId === "channels";
  getPlatformSettings(platform.id);
  return `
    <button class="platform-item ${active ? "active" : ""}" type="button" data-platform="${platform.id}">
      ${platformLogo(platform)}
      <span class="platform-copy">
        <strong>${platform.name}</strong>
        <em>${accountCountLabel(count)}</em>
      </span>
      <span class="count">${count}</span>
      <span class="mini-login" data-login="${platform.id}"${platform.id === "xiaohongshu" ? ' data-login-target="creator"' : ""} title="${text.loginAccount} ${platform.name}">${icon("plus")}</span>
    </button>
  `;
}

function accountItem(account: ChannelAccount) {
  const text = copy[language];
  const platform = platforms.find((item) => item.id === account.platformId);
  const isRefreshing = refreshingAccountIds.has(account.id);
  return `
    <article class="account-card">
      <div class="account-avatar">
        ${
          account.avatar
            ? `<img src="${escapeAttribute(account.avatar)}" alt="">`
            : platform
              ? platformLogo(platform, "avatar")
              : initials(account.nickname)
        }
      </div>
      <div class="account-main">
        <div class="account-line">
          <h3>${escapeHtml(account.nickname)}</h3>
          <span class="status ${account.status}">${statusLabel(account.status)}</span>
        </div>
        <div class="account-meta">
          <span>${platform?.name || account.platformId}</span>
          <span>${formatFollowers(account.followers)}</span>
          <span>${account.lastSyncAt ? `${text.syncedAt} ${formatDate(account.lastSyncAt)}` : text.notSynced}</span>
        </div>
      </div>
      <div class="account-card-actions">
        <button class="icon-btn" type="button" data-open-homepage="${escapeAttribute(account.id)}" title="${text.homepage}">${icon("home")}</button>
        <button class="icon-btn ${isRefreshing ? "is-loading" : ""}" type="button" data-refresh-account="${escapeAttribute(account.id)}" title="${text.refresh}" ${isRefreshing ? "disabled" : ""}>${icon("refresh")}</button>
        <button class="icon-btn danger" type="button" data-delete-account="${escapeAttribute(account.id)}" title="删除账号" ${isRefreshing ? "disabled" : ""}>${icon("trash")}</button>
      </div>
    </article>
  `;
}

function emptyAccounts(platform: PlatformInfo) {
  const text = copy[language];
  return `
    <div class="empty-state">
      <div class="empty-logo">${platformLogo(platform, "large")}</div>
      <h3>${text.noAccountPrefix} ${platform.name} ${text.noAccountSuffix}</h3>
    </div>
  `;
}

function authDialog(task: StartLoginResponse) {
  const text = copy[language];
  const qrAuth = isQrAuth(task);
  const platform = getSelectedPlatform();
  const description = activeAuthMessage || task.instructions || (qrAuth ? text.authQrDesc : text.authDesc);
  return `
    <div class="modal-backdrop">
      <section class="auth-modal" role="dialog" aria-modal="true" aria-label="${text.authTitle}">
        ${
          qrAuth
            ? `<div class="auth-qr"><img src="${escapeAttribute(task.url)}" alt="${escapeAttribute(platform.name)}" /></div>`
            : `<div class="auth-spinner">${icon("refresh")}</div>`
        }
        <h2>${text.authTitle}</h2>
        <p>${escapeHtml(description)}</p>
        <div class="auth-platform">
          ${platformLogo(platform)}
          <span>${platform.name}</span>
        </div>
        <div class="auth-actions">
          <button class="ghost-btn" type="button" data-action="check-auth">${icon("refresh")}${text.checkStatus}</button>
          <button class="ghost-btn" type="button" data-action="close-auth">${text.later}</button>
        </div>
      </section>
    </div>
  `;
}

function isQrAuth(task: StartLoginResponse) {
  return task.authType === "qrcode" || task.url.startsWith("data:image");
}

function navItem(id: SidebarMenuId, iconName: string) {
  const label = copy[language].menu[id];
  const active = id === activeMenuId;
  return `
    <button class="nav-item ${active ? "active" : ""}" type="button" data-menu="${id}" title="${label}">
      ${icon(iconName)}
      <span class="nav-label">${label}</span>
    </button>
  `;
}

function platformLogo(platform: PlatformInfo, size: "default" | "large" | "avatar" = "default") {
  const brand = platformIcons[platform.id];
  const color = brand ? `#${brand.hex}` : platform.color;
  const inner = brand
    ? "markup" in brand
      ? brand.markup
      : `<svg viewBox="0 0 24 24" role="img" aria-label="${escapeAttribute(platform.name)}"><path d="${brand.path}"></path></svg>`
    : `<span>${escapeHtml(platform.slug)}</span>`;
  return `
    <span class="platform-logo platform-logo-${size}" style="--platform-color:${color}">
      ${inner}
    </span>
  `;
}

function icon(name: string) {
  const icons: Record<string, string> = {
    activity: '<svg viewBox="0 0 24 24"><path d="M3 12h4l3 7 4-14 3 7h4"/></svg>',
    calendar: '<svg viewBox="0 0 24 24"><path d="M8 3v4M16 3v4M4 9h16M6 5h12a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z"/></svg>',
    chevron: '<svg viewBox="0 0 24 24"><path d="m15 18-6-6 6-6"/></svg>',
    download: '<svg viewBox="0 0 24 24"><path d="M12 3v12M7 10l5 5 5-5"/><path d="M5 21h14"/></svg>',
    folder: '<svg viewBox="0 0 24 24"><path d="M3 7h7l2 2h9v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z"/></svg>',
    grid: '<svg viewBox="0 0 24 24"><path d="M4 4h7v7H4ZM13 4h7v7h-7ZM4 13h7v7H4ZM13 13h7v7h-7Z"/></svg>',
    help: '<svg viewBox="0 0 24 24"><path d="M12 17h.01M9.1 9a3 3 0 1 1 5.8 1c-.5 1.4-1.9 1.8-2.5 2.7-.3.4-.4.8-.4 1.3"/><circle cx="12" cy="12" r="9"/></svg>',
    home: '<svg viewBox="0 0 24 24"><path d="m4 11 8-7 8 7"/><path d="M6 10v10h12V10"/><path d="M10 20v-6h4v6"/></svg>',
    layers: '<svg viewBox="0 0 24 24"><path d="m12 3 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 16l9 5 9-5"/></svg>',
    lock: '<svg viewBox="0 0 24 24"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>',
    logout: '<svg viewBox="0 0 24 24"><path d="M10 17l5-5-5-5M15 12H3"/><path d="M14 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-4"/></svg>',
    message: '<svg viewBox="0 0 24 24"><path d="M4 5h16v11H7l-3 3Z"/></svg>',
    plus: '<svg viewBox="0 0 24 24"><path d="M12 5v14M5 12h14"/></svg>',
    refresh: '<svg viewBox="0 0 24 24"><path d="M20 12a8 8 0 0 1-14.5 4.7M4 12A8 8 0 0 1 18.5 7.3"/><path d="M20 5v6h-6M4 19v-6h6"/></svg>',
    save: '<svg viewBox="0 0 24 24"><path d="M5 3h12l2 2v16H5Z"/><path d="M8 3v6h8V3M8 21v-7h8v7"/></svg>',
    search: '<svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>',
    send: '<svg viewBox="0 0 24 24"><path d="m22 2-7 20-4-9-9-4 20-7Z"/><path d="M22 2 11 13"/></svg>',
    settings: '<svg viewBox="0 0 24 24"><path d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2 3.4-.2-.1a1.7 1.7 0 0 0-1.9.3 1.7 1.7 0 0 0-.8 1.7V22H9v-.3a1.7 1.7 0 0 0-.8-1.7 1.7 1.7 0 0 0-1.9-.3l-.2.1-2-3.4.1-.1A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-1.4-1H3v-4h.2a1.7 1.7 0 0 0 1.4-1 1.7 1.7 0 0 0-.3-1.9l-.1-.1 2-3.4.2.1a1.7 1.7 0 0 0 1.9-.3A1.7 1.7 0 0 0 9 1.7V1h6v.7a1.7 1.7 0 0 0 .8 1.7 1.7 1.7 0 0 0 1.9.3l.2-.1 2 3.4-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.4 1h.1v4h-.2a1.7 1.7 0 0 0-1.4 1Z"/></svg>',
    spark: '<svg viewBox="0 0 24 24"><path d="m12 2 1.8 6.2L20 10l-6.2 1.8L12 18l-1.8-6.2L4 10l6.2-1.8L12 2Z"/></svg>',
    trash: '<svg viewBox="0 0 24 24"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 14h10l1-14M9 7V4h6v3"/></svg>',
    user: '<svg viewBox="0 0 24 24"><circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/></svg>',
  };
  return `<span class="svg-icon" aria-hidden="true">${icons[name] || icons.grid}</span>`;
}

function statusLabel(status: AccountStatus) {
  const text = copy[language];
  if (status === "active") return text.statusActive;
  if (status === "expired") return text.statusExpired;
  return text.statusPending;
}

function accountCountLabel(count: number) {
  if (language === "zh") {
    return count > 0 ? `${count} ${copy.zh.accountUnit}` : copy.zh.notConnected;
  }
  return count > 0 ? `${count} ${count === 1 ? "account" : copy.en.accountUnit}` : copy.en.notConnected;
}

function formatFollowers(value?: number | null) {
  if (value === undefined || value === null) return copy[language].fansPending;
  if (value >= 10000) return `${(value / 10000).toFixed(1)} 万粉丝`;
  return language === "zh" ? `${value} 粉丝` : `${value} followers`;
}

function formatFollowersTotal(items: ChannelAccount[]) {
  const values = items
    .map((item) => item.followers)
    .filter((value): value is number => typeof value === "number");
  if (!values.length) return "-";
  return formatFollowers(values.reduce((sum, value) => sum + value, 0));
}

function formatDate(value: string) {
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en-US", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function initials(value: string) {
  return escapeHtml((value || "渠").slice(0, 1));
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

function readStoredMode<T extends string>(key: string, fallback: T, allowed: readonly T[]): T {
  const value = localStorage.getItem(key) as T | null;
  return value && allowed.includes(value) ? value : fallback;
}

function readStoredBoolean(key: string, fallback: boolean) {
  const value = localStorage.getItem(key);
  if (value === "true") return true;
  if (value === "false") return false;
  return fallback;
}

function escapeHtml(value: string) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function escapeAttribute(value: string) {
  return escapeHtml(value).replace(/`/g, "&#96;");
}
