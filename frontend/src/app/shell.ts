import type { AuthUser, MenuId } from "../domain/types";
import { APP_VERSION } from "../config/app";
import { icon } from "../ui/icons";
import { escapeAttribute } from "../utils/html";

export interface AppShellState {
  theme: string;
  currentUser: AuthUser;
  activeMenuId: MenuId;
  homeLabel: string;
  userMenuOpen: boolean;
  mainContent: string;
  accountDropdown: string;
  authDialog: string;
}

export function renderAppShell({
  theme,
  currentUser,
  activeMenuId,
  homeLabel,
  userMenuOpen,
  mainContent,
  accountDropdown,
  authDialog,
}: AppShellState) {
  return `
    <div class="window theme-${theme}">
      <aside class="icon-rail" aria-label="主导航">
        <div class="rail-brand">
          <div class="brand-mark" aria-hidden="true">M</div>
        </div>
        <nav class="rail-nav">
          <button class="rail-btn ${activeMenuId === "channels" ? "active" : ""}" type="button" data-menu="channels" title="${escapeAttribute(homeLabel)}" aria-label="${escapeAttribute(homeLabel)}">
            ${icon("home")}
          </button>
        </nav>
        <div class="rail-bottom">
          <div class="rail-version" title="v${escapeAttribute(APP_VERSION)}">v${escapeAttribute(APP_VERSION)}</div>
          <div class="corner-menu-wrap">
            <button class="corner-menu-btn" type="button" data-action="toggle-user-menu" title="${escapeAttribute(currentUser.nickname)}" aria-expanded="${userMenuOpen ? "true" : "false"}">
              ${icon("menu")}
            </button>
            ${userMenuOpen ? accountDropdown : ""}
          </div>
        </div>
      </aside>

      <main class="main ${activeMenuId === "channels" ? "main-channels" : ""}">
        ${mainContent}
      </main>

      ${authDialog}
      <div class="toast" hidden></div>
    </div>
  `;
}
