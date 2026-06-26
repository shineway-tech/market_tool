import type { AuthUser } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { escapeHtml } from "../utils/html";
import { icon } from "./icons";

export interface AccountDropdownState {
  text: CopyText;
  user: AuthUser | null;
}

export function renderAccountDropdown({ text, user }: AccountDropdownState) {
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
      <div class="user-menu-separator" aria-hidden="true"></div>
      <button class="user-menu-item" type="button" data-menu="settings">
        ${icon("settings")}
        <span>${text.menu.settings}</span>
      </button>
      <button class="user-menu-item" type="button" data-menu="releases">
        ${icon("spark")}
        <span>${text.menu.releases}</span>
      </button>
      <button class="user-menu-item" type="button" data-menu="feedback">
        ${icon("message")}
        <span>${text.menu.feedback}</span>
      </button>
      <div class="user-menu-separator" aria-hidden="true"></div>
      <button class="user-menu-item danger" type="button" data-action="logout">
        ${icon("logout")}
        <span>${text.logout}</span>
      </button>
    </div>
  `;
}
