export type AuthMode = "creator" | "oAuth";
export type AccountStatus = "active" | "expired" | "pending";
export type UserMenuPageId =
  | "settings"
  | "releases"
  | "feedback";
export type AppPageId = "channels" | UserMenuPageId;
export type MenuId = AppPageId | "profile" | "password";
export type LanguageMode = "zh" | "en";
export type ThemeMode = "dark" | "light";
export type LoginTarget = "home" | "creator";
export type AuthViewMode = "login" | "register";
export type UpdateStatus = "idle" | "checking" | "latest" | "available" | "downloading" | "installed" | "error";

export interface ApiResponse<T> {
  err_code: number;
  err_msg: string;
  data: T;
}

export interface AuthUser {
  id: string;
  account: string;
  nickname: string;
  status: string;
  lastLoginAt?: string | null;
}

export interface CaptchaResponse {
  captchaId: string;
  image: string;
  expiresAt: string;
}

export interface AuthSession {
  token: string;
  tokenName: string;
  expiresIn: number;
  user: AuthUser;
}

export interface PlatformInfo {
  id: string;
  name: string;
  slug: string;
  color: string;
  description: string;
}

export interface PlatformAuthSettings {
  platformId: string;
  mode: AuthMode;
  authUrl: string;
  tokenUrl: string;
  profileUrl: string;
  clientId: string;
  clientSecret: string;
  scopes: string[];
}

export interface AuthSettings {
  platforms: PlatformAuthSettings[];
}

export interface ChannelAccount {
  id: string;
  userId?: string;
  platformId: string;
  uid: string;
  nickname: string;
  avatar: string;
  followers?: number | null;
  likes?: number | null;
  status: AccountStatus;
  createdAt: string;
  updatedAt: string;
  lastSyncAt?: string | null;
}

export interface Bootstrap {
  platforms: PlatformInfo[];
  accounts: ChannelAccount[];
  settings: AuthSettings;
  callbackBaseUrl?: string | null;
}

export interface StartLoginResponse {
  taskId: string;
  url: string;
  callbackUrl: string;
  mode: AuthMode;
  authType?: string;
  sessionId?: string | null;
  expiresAt?: string | null;
  instructions?: string | null;
}

export interface AuthTaskStatus {
  taskId: string;
  status: "pending" | "success" | "failed" | "unknown";
  account?: ChannelAccount | null;
  message?: string | null;
}

export interface UpdateState {
  status: UpdateStatus;
  availableVersion?: string;
  notes?: string;
  downloadedBytes: number;
  contentLength?: number;
  error?: string;
}
