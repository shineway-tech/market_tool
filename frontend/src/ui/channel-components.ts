import type { ChannelAccount, PlatformInfo, StartLoginResponse } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { statusLabel } from "../utils/format";
import { escapeAttribute, escapeHtml } from "../utils/html";
import { isQrAuth } from "../domain/auth-task";
import { icon } from "./icons";
import { platformLogo } from "./platform-logo";

export interface PlatformTreeItemState {
  platform: PlatformInfo;
  count: number;
  active: boolean;
  expanded: boolean;
  canToggle: boolean;
  accountsHtml: string;
}

export interface AccountNavItemState {
  account: ChannelAccount;
  text: CopyText;
  platform?: PlatformInfo;
  active: boolean;
  isUnavailable: boolean;
  fallbackAvatar: string;
}

export interface AuthDialogState {
  task: StartLoginResponse;
  text: CopyText;
  platform: PlatformInfo;
  description: string;
}

export function renderPlatformTreeItem({
  platform,
  count,
  active,
  expanded,
  canToggle,
  accountsHtml,
}: PlatformTreeItemState) {
  return `
    <div class="platform-tree-group ${active ? "active" : ""} ${expanded ? "expanded" : ""} ${count === 0 || !canToggle ? "is-empty" : ""}">
      <div class="platform-tree-head" data-platform="${escapeAttribute(platform.id)}">
        <button class="platform-select" type="button">
          <span class="platform-logo-wrap">
            ${platformLogo(platform)}
          </span>
          <span class="platform-copy">
            <strong>${escapeHtml(platform.name)}</strong>
          </span>
        </button>
        <span class="platform-count-text">${count}</span>
        ${
          count > 0 && canToggle
            ? `<button class="platform-toggle ${expanded ? "expanded" : ""}" type="button" data-toggle-platform="${escapeAttribute(platform.id)}" title="${escapeAttribute(platform.name)}">${icon("chevron")}</button>`
            : ""
        }
      </div>
      ${
        expanded && accountsHtml
          ? `<div class="platform-account-list">${accountsHtml}</div>`
          : ""
      }
    </div>
  `;
}

export function renderAccountNavItem({
  account,
  text,
  platform,
  active,
  isUnavailable,
  fallbackAvatar,
}: AccountNavItemState) {
  return `
    <button class="account-nav-item ${active ? "active" : ""} ${isUnavailable ? "is-unavailable" : ""}" type="button" data-account="${escapeAttribute(account.id)}" title="${escapeAttribute(`${account.nickname} · ${statusLabel(account.status, text)}`)}">
      ${renderAccountAvatar(account, platform, fallbackAvatar, `account-nav-avatar status-${account.status}`)}
      <span class="account-nav-copy">
        <strong>${escapeHtml(account.nickname)}</strong>
      </span>
    </button>
  `;
}

export function renderAccountAvatar(
  account: ChannelAccount,
  platform: PlatformInfo | undefined,
  fallbackAvatar: string,
  className: string,
) {
  return `
    <div class="${className}">
      ${
        account.avatar
          ? `<img src="${escapeAttribute(account.avatar)}" alt="">`
          : platform
            ? platformLogo(platform, "avatar")
            : fallbackAvatar
      }
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
