import type { AccountStatus, ChannelAccount, LanguageMode } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { escapeHtml } from "./html";

export function statusLabel(status: AccountStatus, text: CopyText) {
  if (status === "active") return text.statusActive;
  if (status === "expired") return text.statusExpired;
  return text.statusPending;
}

export function formatFollowers(value: number | null | undefined, language: LanguageMode) {
  if (value === undefined || value === null) return "-";
  if (language === "zh" && value >= 10000) return `${(value / 10000).toFixed(1)}万`;
  return new Intl.NumberFormat(language === "zh" ? "zh-CN" : "en-US").format(value);
}

export function formatFollowersTotal(items: ChannelAccount[], language: LanguageMode) {
  const values = items
    .map((item) => item.followers)
    .filter((value): value is number => typeof value === "number");
  if (!values.length) return "-";
  return formatFollowers(values.reduce((sum, value) => sum + value, 0), language);
}

export function formatDate(value: string, language: LanguageMode) {
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : "en-US", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

export function initials(value: string) {
  return escapeHtml((value || "渠").slice(0, 1));
}
