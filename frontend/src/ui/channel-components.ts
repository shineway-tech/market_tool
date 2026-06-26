import type { ChannelAccount, PlatformInfo, StartLoginResponse } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { formatDate, formatFollowers, statusLabel } from "../utils/format";
import { escapeAttribute, escapeHtml } from "../utils/html";
import { isQrAuth } from "../domain/auth-task";
import { icon } from "./icons";
import { platformLogo } from "./platform-logo";

export interface PlatformItemState {
  platform: PlatformInfo;
  text: CopyText;
  count: number;
  active: boolean;
  countLabel: string;
}

export interface AccountItemState {
  account: ChannelAccount;
  text: CopyText;
  platform?: PlatformInfo;
  isRefreshing: boolean;
  isOpeningHomepage: boolean;
  isUnavailable: boolean;
  followersText: string;
  syncText: string;
  fallbackAvatar: string;
}

export interface EmptyAccountsState {
  platform: PlatformInfo;
  text: CopyText;
}

export interface AuthDialogState {
  task: StartLoginResponse;
  text: CopyText;
  platform: PlatformInfo;
  description: string;
}

export function renderPlatformItem({
  platform,
  text,
  count,
  active,
  countLabel,
}: PlatformItemState) {
  return `
    <button class="platform-item ${active ? "active" : ""}" type="button" data-platform="${platform.id}">
      ${platformLogo(platform)}
      <span class="platform-copy">
        <strong>${platform.name}</strong>
        <em>${countLabel}</em>
      </span>
      <span class="count">${count}</span>
      <span class="mini-login" data-login="${platform.id}"${platform.id === "xiaohongshu" ? ' data-login-target="creator"' : ""} title="${text.loginAccount} ${platform.name}">${icon("plus")}</span>
    </button>
  `;
}

export function renderAccountItem({
  account,
  text,
  platform,
  isRefreshing,
  isOpeningHomepage,
  isUnavailable,
  followersText,
  syncText,
  fallbackAvatar,
}: AccountItemState) {
  const platformId = platform?.id || account.platformId;
  return `
    <article class="account-card ${isUnavailable ? "is-unavailable" : ""}">
      <div class="account-avatar">
        ${
          account.avatar
            ? `<img src="${escapeAttribute(account.avatar)}" alt="">`
            : platform
              ? platformLogo(platform, "avatar")
              : fallbackAvatar
        }
      </div>
      <div class="account-main">
        <div class="account-line">
          <h3>${escapeHtml(account.nickname)}</h3>
          <span class="status ${account.status}">${statusLabel(account.status, text)}</span>
        </div>
        <div class="account-meta">
          <span>${platform?.name || account.platformId}</span>
          <span>${followersText}</span>
          <span>${syncText}</span>
        </div>
      </div>
      <div class="account-card-actions">
        ${
          isUnavailable
            ? `<button class="ghost-btn relogin-btn" type="button" data-login="${escapeAttribute(platformId)}"${platformId === "xiaohongshu" ? ' data-login-target="creator"' : ""} title="${text.reloginAccount}" ${isRefreshing ? "disabled" : ""}>${icon("refresh")}${text.reloginAccount}</button>`
            : `<button class="icon-btn" type="button" data-open-homepage="${escapeAttribute(account.id)}" title="${text.homepage}" ${isOpeningHomepage || isRefreshing ? "disabled" : ""}>${icon("home")}</button>`
        }
        <button class="icon-btn ${isRefreshing ? "is-loading" : ""}" type="button" data-refresh-account="${escapeAttribute(account.id)}" title="${text.refresh}" ${isRefreshing ? "disabled" : ""}>${icon("refresh")}</button>
        <button class="icon-btn danger" type="button" data-delete-account="${escapeAttribute(account.id)}" title="删除账号" ${isRefreshing || isOpeningHomepage ? "disabled" : ""}>${icon("trash")}</button>
      </div>
    </article>
  `;
}

export function renderEmptyAccounts({ platform, text }: EmptyAccountsState) {
  return `
    <div class="empty-state">
      <div class="empty-logo">${platformLogo(platform, "large")}</div>
      <h3>${text.noAccountPrefix} ${platform.name} ${text.noAccountSuffix}</h3>
    </div>
  `;
}

export function renderAuthDialog({ task, text, platform, description }: AuthDialogState) {
  const qrAuth = isQrAuth(task);
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

export function accountSyncText(account: ChannelAccount, text: CopyText, language: "zh" | "en") {
  return account.lastSyncAt ? `${text.syncedAt} ${formatDate(account.lastSyncAt, language)}` : text.notSynced;
}

export function accountFollowersText(account: ChannelAccount, text: CopyText, language: "zh" | "en") {
  return formatFollowers(account.followers, language, text);
}
