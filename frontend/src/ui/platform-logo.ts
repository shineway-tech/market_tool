import type { PlatformInfo } from "../domain/types";
import { platformIcons } from "../domain/platforms";
import { escapeAttribute, escapeHtml } from "../utils/html";

export function platformLogo(platform: PlatformInfo, size: "default" | "large" | "avatar" = "default") {
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
