import type { SidebarMenuId } from "../domain/types";
import { icon } from "./icons";

export interface NavItemState {
  id: SidebarMenuId;
  iconName: string;
  label: string;
  active: boolean;
}

export function renderNavItem({ id, iconName, label, active }: NavItemState) {
  return `
    <button class="nav-item ${active ? "active" : ""}" type="button" data-menu="${id}" title="${label}">
      ${icon(iconName)}
      <span class="nav-label">${label}</span>
    </button>
  `;
}
