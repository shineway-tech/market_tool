import type { AccountStatus, ChannelAccount, LanguageMode } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { copy } from "../i18n/copy";
import { escapeHtml } from "./html";

export function statusLabel(status: AccountStatus, text: CopyText) {
  if (status === "active") return text.statusActive;
  if (status === "expired") return text.statusExpired;
  return text.statusPending;
}

export function accountCountLabel(count: number, language: LanguageMode) {
  if (language === "zh") {
    return count > 0 ? `${count} ${copy.zh.accountUnit}` : copy.zh.notConnected;
  }
  return count > 0 ? `${count} ${count === 1 ? "account" : copy.en.accountUnit}` : copy.en.notConnected;
}

export function formatFollowers(value: number | null | undefined, language: LanguageMode, text: CopyText) {
  if (value === undefined || value === null) return text.fansPending;
  if (value >= 10000) return `${(value / 10000).toFixed(1)} 万粉丝`;
  return language === "zh" ? `${value} 粉丝` : `${value} followers`;
}

export function formatFollowersTotal(items: ChannelAccount[], language: LanguageMode, text: CopyText) {
  const values = items
    .map((item) => item.followers)
    .filter((value): value is number => typeof value === "number");
  if (!values.length) return "-";
  return formatFollowers(values.reduce((sum, value) => sum + value, 0), language, text);
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
