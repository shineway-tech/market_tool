import type {
  ChannelAccountContent,
  ChannelComment,
  ChannelWork,
  ChannelWorksPage,
  ContentTab,
} from "../domain/channel-content";
import type { ChannelAccount, LanguageMode, PlatformInfo } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { formatDate, formatFollowers, statusLabel } from "../utils/format";
import { escapeAttribute, escapeHtml } from "../utils/html";
import { icon } from "../ui/icons";
import { renderAccountAvatar } from "../ui/channel-components";
import { platformLogo } from "../ui/platform-logo";

type OverviewPeriod = 1 | 7 | 30 | 90 | 36500 | 65535;
const BILIBILI_HISTORY_PERIOD: OverviewPeriod = 36500;
const BILIBILI_TOTAL_PERIOD: OverviewPeriod = 65535;
const BILIBILI_DEFAULT_COVER =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 320 180'%3E%3Crect width='320' height='180' rx='10' fill='%23eef5f8'/%3E%3Crect x='102' y='49' width='116' height='82' rx='18' fill='%23ffffff' stroke='%2300a1d6' stroke-width='6'/%3E%3Cpath d='M132 42l-16-19M188 42l16-19' stroke='%2300a1d6' stroke-width='8' stroke-linecap='round'/%3E%3Ccircle cx='143' cy='91' r='7' fill='%2300a1d6'/%3E%3Ccircle cx='177' cy='91' r='7' fill='%2300a1d6'/%3E%3Cpath d='M142 113h36' stroke='%2300a1d6' stroke-width='7' stroke-linecap='round'/%3E%3Ctext x='160' y='157' text-anchor='middle' font-family='Arial, sans-serif' font-size='22' font-weight='700' fill='%2300a1d6'%3Ebilibili%3C/text%3E%3C/svg%3E";

export interface ChannelsPageState {
  text: CopyText;
  language: LanguageMode;
  selectedPlatform: PlatformInfo;
  selectedAccount: ChannelAccount | null;
  selectedAccounts: ChannelAccount[];
  platformRefreshing: boolean;
  selectedAccountRefreshing: boolean;
  selectedAccountOpeningHomepage: boolean;
  selectedAccountContent: ChannelAccountContent | null;
  selectedAccountContentLoading: boolean;
  selectedWorksPages: ChannelWorksPage[];
  selectedWorksLoading: boolean;
  selectedWorkType: "video" | "article";
  overviewPeriod: OverviewPeriod;
  activeTab: ContentTab;
  works: ChannelWork[];
  comments: ChannelComment[];
  platforms: PlatformInfo[];
  searchQuery: string;
  hasSearchResults: boolean;
  platformTree: (platform: PlatformInfo) => string;
  formatFollowersTotal: (items: ChannelAccount[]) => string;
}

export function renderChannelsPage({
  text,
  language,
  selectedPlatform,
  selectedAccount,
  selectedAccounts,
  platformRefreshing,
  selectedAccountRefreshing,
  selectedAccountOpeningHomepage,
  selectedAccountContent,
  selectedAccountContentLoading,
  selectedWorksPages,
  selectedWorksLoading,
  selectedWorkType,
  overviewPeriod,
  activeTab,
  works,
  comments,
  platforms,
  searchQuery,
  hasSearchResults,
  platformTree,
  formatFollowersTotal,
}: ChannelsPageState) {
  const targetAccounts = selectedAccount ? [selectedAccount] : selectedAccounts;
  const targetAccountCount = targetAccounts.length;
  const targetActiveCount = targetAccounts.filter((item) => item.status === "active").length;
  const totalViews = works.reduce((sum, item) => sum + (item.views || 0), 0);
  const totalLikes = works.reduce((sum, item) => sum + (item.likes || 0), 0);
  const pendingComments = comments.filter((item) => item.status === "unread" || item.status === "risk").length;
  const workspaceState: WorkspaceTabState = {
    text,
    language,
    activeTab,
    selectedPlatform,
    selectedAccount,
    selectedAccounts: targetAccounts,
    allSelectedAccounts: selectedAccounts,
    activeAccountCount: targetActiveCount,
    connectedCount: targetAccountCount,
    works,
    comments,
    accountContent: selectedAccountContent,
    accountContentLoading: selectedAccountContentLoading,
    worksPages: selectedWorksPages,
    worksLoading: selectedWorksLoading,
    selectedWorkType,
    overviewPeriod,
    totalViews,
    totalLikes,
    pendingComments,
    formatFollowersTotal,
  };

  return `
    <section class="channel-layout">
      <aside class="platform-panel">
        <div class="platform-search">
          <label class="platform-search-box">
            ${icon("search")}
            <input type="search" data-channel-search value="${escapeAttribute(searchQuery)}" placeholder="${escapeAttribute(text.channelSearchPlaceholder)}" autocomplete="off" spellcheck="false" />
          </label>
        </div>
        <div class="platform-list">
          ${hasSearchResults ? platforms.map((platform) => platformTree(platform)).join("") : `<div class="platform-search-empty">${text.channelSearchEmpty}</div>`}
        </div>
      </aside>

      <section class="workspace-panel">
        ${renderWorkspaceHead({
          text,
          language,
          selectedPlatform,
          selectedAccount,
          selectedAccountRefreshing,
          selectedAccountOpeningHomepage,
          platformRefreshing,
        })}
        ${selectedAccount ? renderAccountWorkspace(workspaceState) : renderPlatformWorkspace(workspaceState)}
      </section>
    </section>
  `;
}

interface WorkspaceHeadState {
  text: CopyText;
  language: LanguageMode;
  selectedPlatform: PlatformInfo;
  selectedAccount: ChannelAccount | null;
  selectedAccountRefreshing: boolean;
  selectedAccountOpeningHomepage: boolean;
  platformRefreshing: boolean;
}

interface WorkspaceTabState {
  text: CopyText;
  language: LanguageMode;
  activeTab: ContentTab;
  selectedPlatform: PlatformInfo;
  selectedAccount: ChannelAccount | null;
  selectedAccounts: ChannelAccount[];
  allSelectedAccounts: ChannelAccount[];
  activeAccountCount: number;
  connectedCount: number;
  works: ChannelWork[];
  comments: ChannelComment[];
  accountContent: ChannelAccountContent | null;
  accountContentLoading: boolean;
  worksPages: ChannelWorksPage[];
  worksLoading: boolean;
  selectedWorkType: "video" | "article";
  overviewPeriod: OverviewPeriod;
  totalViews: number;
  totalLikes: number;
  pendingComments: number;
  formatFollowersTotal: (items: ChannelAccount[]) => string;
}

function renderWorkspaceHead({
  text,
  language,
  selectedPlatform,
  selectedAccount,
  selectedAccountRefreshing,
  selectedAccountOpeningHomepage,
  platformRefreshing,
}: WorkspaceHeadState) {
  const accountUnavailable = selectedAccount ? selectedAccount.status !== "active" : false;
  const accountHeaderStatsHtml = selectedAccount
    ? renderAccountHeaderStats(selectedAccount, selectedPlatform, text, language)
    : "";
  const title = selectedAccount ? escapeHtml(selectedAccount.nickname) : escapeHtml(selectedPlatform.name);
  const subtitleHtml = selectedAccount
    ? renderPlatformAccountMeta(selectedPlatform, selectedAccount, text, language)
    : escapeHtml(selectedPlatform.description);

  return `
    <div class="workspace-head">
      <div class="workspace-identity ${selectedAccount ? "with-header-stats" : ""}">
        ${
          selectedAccount
            ? renderAccountAvatar(selectedAccount, selectedPlatform, escapeHtml(selectedAccount.nickname.slice(0, 1)), "workspace-avatar")
            : platformLogo(selectedPlatform, "large")
        }
        <div class="workspace-copy">
          <div class="workspace-title-row">
            <h2>${title}</h2>
            ${selectedAccount ? `<span class="account-status-pill ${selectedAccount.status}">${text.accountStatusPrefix}${statusLabel(selectedAccount.status, text)}</span>` : ""}
          </div>
          ${accountHeaderStatsHtml}
          <p>${subtitleHtml}</p>
        </div>
      </div>
      <div class="workspace-actions">
        ${
          selectedAccount
            ? `${accountUnavailable
                ? `<button class="workspace-action" type="button" data-login="${escapeAttribute(selectedAccount.platformId)}"${selectedAccount.platformId === "xiaohongshu" ? ' data-login-target="creator"' : ""} title="${text.reloginAccount}" ${selectedAccountRefreshing ? "disabled" : ""}>${icon("user")}<span>${text.reloginAccount}</span></button>`
                : `<button class="workspace-action" type="button" data-open-homepage="${escapeAttribute(selectedAccount.id)}" title="${text.homepage}" ${selectedAccountOpeningHomepage || selectedAccountRefreshing ? "disabled" : ""}>${icon("home")}<span>${text.homepage}</span></button>`
              }
              <button class="workspace-action ${selectedAccountRefreshing ? "is-loading" : ""}" type="button" data-refresh-account="${escapeAttribute(selectedAccount.id)}" title="${text.refresh}" ${selectedAccountRefreshing ? "disabled" : ""}>${icon("refresh")}<span>${selectedAccountRefreshing ? text.refreshing : text.refresh}</span></button>
              <button class="workspace-action danger" type="button" data-delete-account="${escapeAttribute(selectedAccount.id)}" title="${text.deleteAccount}" ${selectedAccountRefreshing || selectedAccountOpeningHomepage ? "disabled" : ""}>${icon("trash")}<span>${text.deleteAccount}</span></button>
            `
            : `
              <button class="workspace-action" type="button" data-login="${escapeAttribute(selectedPlatform.id)}"${selectedPlatform.id === "xiaohongshu" ? ' data-login-target="creator"' : ""} title="${escapeAttribute(`${text.loginAccount} ${selectedPlatform.name}`)}">${icon("plus")}<span>${text.loginAccount}</span></button>
              <button class="workspace-action ${platformRefreshing ? "is-loading" : ""}" type="button" data-action="refresh-platform" title="${text.refreshPlatform}" ${platformRefreshing ? "disabled" : ""}>${icon("refresh")}<span>${platformRefreshing ? text.refreshing : text.refresh}</span></button>
            `
        }
      </div>
    </div>
  `;
}

function renderAccountHeaderStats(
  account: ChannelAccount,
  platform: PlatformInfo,
  text: CopyText,
  language: LanguageMode,
) {
  const likesLabel = platform.id === "xiaohongshu" ? text.likesAndFavorites : text.likesLabel;
  const likesPending = platform.id === "xiaohongshu" ? text.likesAndFavoritesPending : text.likesPending;
  return `
    <div class="workspace-head-metrics">
      ${renderHeadMetric(text.totalFans, formatFollowers(account.followers, language))}
      ${renderHeadMetric(text.followingLabel, formatOptionalNumber(account.following, text.followingPending, language))}
      ${renderHeadMetric(likesLabel, formatOptionalNumber(account.likes, likesPending, language))}
      ${renderHeadMetric(text.lastSyncTimeLabel, lastSyncValue(account, text, language))}
    </div>
  `;
}

function renderHeadMetric(label: string, value: string) {
  return `
    <span class="workspace-head-metric">
      <em>${label}</em>
      <strong>${value}</strong>
    </span>
  `;
}

function renderWorkspaceTabs(text: CopyText, activeTab: ContentTab, state: WorkspaceTabState) {
  const tabs: Array<{ id: ContentTab; label: string }> = [
    { id: "overview", label: text.overviewTab },
    { id: "works", label: text.worksTab },
  ];
  if (!(state.selectedAccount && ["xiaohongshu", "wechat-channels", "douyin", "bilibili", "kuaishou"].includes(state.selectedPlatform.id))) {
    tabs.push(
      { id: "comments", label: text.commentsTab },
      { id: "data", label: text.dataTab },
    );
  }

  return `
    <div class="workspace-tabs" role="tablist">
      ${tabs
        .map(
          (tab) => `
            <button class="workspace-tab ${tab.id === activeTab ? "active" : ""}" type="button" data-channel-tab="${tab.id}" role="tab" aria-selected="${tab.id === activeTab}">
              ${tab.label}
            </button>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderPlatformWorkspace(state: WorkspaceTabState) {
  switch (state.selectedPlatform.id) {
    default:
      return renderDefaultPlatformWorkspace(state);
  }
}

function renderAccountWorkspace(state: WorkspaceTabState) {
  switch (state.selectedPlatform.id) {
    default:
      return `
        ${renderWorkspaceTabs(state.text, state.activeTab, state)}
        <div class="workspace-body">
          ${renderAccountWorkspaceTab(state)}
        </div>
      `;
  }
}

function renderDefaultPlatformWorkspace(state: WorkspaceTabState) {
  const { text, language, selectedPlatform, selectedAccounts, activeAccountCount, connectedCount, formatFollowersTotal } = state;
  const metrics = [
    renderMetric(text.accountPanel, formatNumber(connectedCount, language)),
    renderMetric(text.availableAccount, formatNumber(activeAccountCount, language)),
    renderMetric(text.totalFans, formatFollowersTotal(selectedAccounts)),
    renderMetric(text.totalFollowing, formatAccountMetricTotal(selectedAccounts, "following", language)),
    renderMetric(
      selectedPlatform.id === "xiaohongshu" ? text.likesAndFavorites : text.totalAccountLikes,
      formatAccountMetricTotal(selectedAccounts, "likes", language),
    ),
  ];
  if (!platformHidesLastSyncMetric(selectedPlatform.id)) {
    metrics.push(renderMetric(text.lastSyncLabel, latestSyncText(selectedAccounts, text, language)));
  }

  return `
    <div class="workspace-body platform-workspace-body">
      <div class="metric-grid">
        ${metrics.join("")}
      </div>
      <section class="content-block platform-account-block">
        <div class="content-block-head">
          <h3>${text.accountPerformance}</h3>
          <span>${formatNumber(selectedAccounts.length, language)}</span>
        </div>
        <div class="account-data-list platform-account-listing">
          ${selectedAccounts.length ? selectedAccounts.map((account) => renderPlatformAccountRow(account, state.selectedPlatform, text, language)).join("") : renderEmpty(text.noAccountDesc)}
        </div>
      </section>
    </div>
  `;
}

function platformHidesLastSyncMetric(platformId: string) {
  return ["xiaohongshu", "wechat-channels", "douyin", "bilibili", "kuaishou"].includes(platformId);
}

function renderAccountWorkspaceTab(state: WorkspaceTabState) {
  if (["xiaohongshu", "wechat-channels", "douyin", "bilibili", "kuaishou"].includes(state.selectedPlatform.id) && state.activeTab !== "works") {
    return renderOverviewView({ ...state, activeTab: "overview" });
  }
  if (state.activeTab === "works") return renderWorksView(state);
  if (state.activeTab === "comments") return renderCommentsView(state);
  if (state.activeTab === "data") return renderDataView(state);
  return renderOverviewView(state);
}

function renderOverviewView(state: WorkspaceTabState) {
  const {
    text,
    language,
    allSelectedAccounts,
    selectedAccount,
    works,
    comments,
  } = state;
  const account = selectedAccount || state.selectedAccounts[0] || null;
  if (!account) return renderEmpty(text.noAccountDesc);
  if (state.selectedPlatform.id === "xiaohongshu" && selectedAccount) {
    return renderXhsOverviewView(state);
  }
  if (state.selectedPlatform.id === "wechat-channels" && selectedAccount) {
    return renderWechatChannelsOverviewView(state);
  }
  if (state.selectedPlatform.id === "douyin" && selectedAccount) {
    return renderDouyinOverviewView(state);
  }
  if (state.selectedPlatform.id === "bilibili" && selectedAccount) {
    return renderBilibiliOverviewView(state);
  }
  if (state.selectedPlatform.id === "kuaishou" && selectedAccount) {
    return renderKuaishouOverviewView(state);
  }
  const showHeaderStats = Boolean(selectedAccount);

  return `
    ${
      showHeaderStats
        ? ""
        : `<div class="metric-grid">
            ${renderMetric(text.totalFans, formatFollowers(account.followers, language))}
            ${renderMetric(text.followingLabel, formatOptionalNumber(account.following, text.followingPending, language))}
            ${renderMetric(text.likesLabel, formatOptionalNumber(account.likes, text.likesPending, language))}
            ${renderMetric(text.lastSyncLabel, accountSyncText(account, text, language))}
          </div>`
    }
    <div class="workspace-columns ${showHeaderStats ? "no-top-metrics" : ""}">
      <section class="content-block">
        <div class="content-block-head">
          <h3>${text.recentWorks}</h3>
          <span>${formatNumber(works.length, language)}</span>
        </div>
        <div class="content-list compact">
          ${works.length ? works.slice(0, 4).map((work) => renderWorkRow(work, allSelectedAccounts, text, language)).join("") : renderEmpty(text.noWorks)}
        </div>
      </section>
      <section class="content-block">
        <div class="content-block-head">
          <h3>${text.recentComments}</h3>
          <span>${formatNumber(comments.length, language)}</span>
        </div>
        <div class="content-list compact">
          ${comments.length ? comments.slice(0, 5).map((comment) => renderCommentRow(comment, allSelectedAccounts, text, language)).join("") : renderEmpty(text.noComments)}
        </div>
      </section>
    </div>
  `;
}

function renderXhsOverviewView(state: WorkspaceTabState) {
  const { text, language, accountContent, overviewPeriod } = state;
  const toolbar = renderXhsOverviewToolbar(text, overviewPeriod);

  if (state.accountContentLoading && !accountContent) {
    return `
      <div class="xhs-overview">
        ${toolbar}
        ${renderAccountContentLoading()}
      </div>
    `;
  }

  const overview = overviewPeriod === 30 ? accountContent?.overviewThirty : accountContent?.overviewSeven;
  const latestWork =
    accountContent?.latestWork ||
    (overviewPeriod === 30
      ? accountContent?.latestWorkThirty || null
      : accountContent?.latestWorkSeven || null);
  const syncError = accountContent?.error || overview?.error || "";

  return `
    <div class="xhs-overview">
      ${toolbar}
      ${syncError ? `<div class="sync-inline-error">${escapeHtml(syncError)}</div>` : ""}
      <div class="xhs-metric-grid">
        ${(overview?.metrics?.length ? overview.metrics : emptyXhsMetrics()).map((metric) => renderXhsMetric(metric)).join("")}
      </div>
      ${overview?.summary ? `<p class="xhs-summary">${escapeHtml(overview.summary)}</p>` : ""}
      <section class="content-block latest-work-block">
        <div class="content-block-head">
          <h3>${text.latestWork}</h3>
          <span>${latestWork ? "1" : "0"}</span>
        </div>
        <div class="content-list compact">
          ${latestWork ? renderXhsLatestWork(latestWork, state.allSelectedAccounts, text, language) : renderEmpty(text.noWorks)}
        </div>
      </section>
    </div>
  `;
}

function renderXhsOverviewToolbar(text: CopyText, overviewPeriod: number) {
  return renderOverviewPeriodToolbar(text, overviewPeriod, {
    title: text.noteOverviewTitle,
    periods: [7, 30],
  });
}

function renderDouyinOverviewView(state: WorkspaceTabState) {
  const { text, language, accountContent, overviewPeriod } = state;
  const toolbar = renderOverviewPeriodToolbar(text, overviewPeriod, {
    title: text.douyinOverviewTitle,
    periods: [1, 7, 30],
  });

  if (state.accountContentLoading && !accountContent) {
    return `
      <div class="xhs-overview douyin-overview">
        ${toolbar}
        ${renderAccountContentLoading()}
      </div>
    `;
  }

  const overview = overviewPeriod === 1
    ? accountContent?.overviewYesterday
    : overviewPeriod === 30
      ? accountContent?.overviewThirty
      : accountContent?.overviewSeven;
  const syncError = accountContent?.error || overview?.error || "";
  const metrics = overview?.metrics?.length ? overview.metrics : emptyDouyinMetrics();
  const latestWork = accountContent?.latestWork || null;

  return `
    <div class="xhs-overview douyin-overview">
      ${toolbar}
      ${syncError ? `<div class="sync-inline-error">${escapeHtml(syncError)}</div>` : ""}
      ${renderAdaptiveMetricGrid(metrics)}
      ${overview?.summary ? `<p class="xhs-summary">${escapeHtml(overview.summary)}</p>` : ""}
      <section class="content-block latest-work-block">
        <div class="content-block-head">
          <h3>${text.latestWork}</h3>
          <span>${latestWork ? "1" : "0"}</span>
        </div>
        <div class="content-list compact">
          ${latestWork ? renderDouyinLatestWork(latestWork, state.allSelectedAccounts, text, language) : renderEmpty(text.noWorks)}
        </div>
      </section>
    </div>
  `;
}

function renderBilibiliOverviewView(state: WorkspaceTabState) {
  const { text, language, accountContent, overviewPeriod } = state;
  const toolbar = renderOverviewPeriodToolbar(text, overviewPeriod, {
    title: text.bilibiliOverviewTitle,
    periods: [1, 7, 30, 90, BILIBILI_TOTAL_PERIOD],
  });

  if (state.accountContentLoading && !accountContent) {
    return `
      <div class="xhs-overview bilibili-overview">
        ${toolbar}
        ${renderAccountContentLoading()}
      </div>
    `;
  }

  const overview = overviewForPeriod(accountContent, overviewPeriod);
  const syncError = accountContent?.error || overview?.error || "";
  const metrics = overview?.metrics?.length ? overview.metrics : emptyBilibiliMetrics();
  const latestVideoWork = accountContent?.latestWork || null;
  const latestArticleWork = accountContent?.latestWorkSeven || null;
  const latestWork = state.selectedWorkType === "article" ? latestArticleWork : latestVideoWork;

  return `
    <div class="xhs-overview bilibili-overview">
      ${toolbar}
      ${syncError ? `<div class="sync-inline-error">${escapeHtml(syncError)}</div>` : ""}
      ${renderBilibiliMetricGrid(metrics)}
      ${overview?.summary ? `<p class="xhs-summary">${escapeHtml(overview.summary)}</p>` : ""}
      <section class="content-block latest-work-block wechat-latest-work-block">
        <div class="content-block-head latest-work-head">
          <h3>${text.latestWork}</h3>
          ${renderWorkTypeSegmented(text, state.selectedWorkType)}
        </div>
        <div class="content-list compact">
          ${latestWork ? renderWechatLatestWork(latestWork, state.allSelectedAccounts, text, language) : renderEmpty(text.noWorks)}
        </div>
      </section>
    </div>
  `;
}

function renderKuaishouOverviewView(state: WorkspaceTabState) {
  const { text, accountContent, overviewPeriod, language } = state;
  const toolbar = renderOverviewPeriodToolbar(text, overviewPeriod, {
    title: text.kuaishouOverviewTitle,
    periods: [7, 30, 90],
  });

  if (state.accountContentLoading && !accountContent) {
    return `
      <div class="xhs-overview kuaishou-overview">
        ${toolbar}
        ${renderAccountContentLoading()}
      </div>
    `;
  }

  const overview = overviewForPeriod(accountContent, overviewPeriod);
  const syncError = accountContent?.error || overview?.error || "";
  const metrics = overview?.metrics?.length ? overview.metrics : emptyKuaishouMetrics();
  const latestWork = accountContent?.latestWork || null;

  return `
    <div class="xhs-overview kuaishou-overview">
      ${toolbar}
      ${syncError ? `<div class="sync-inline-error">${escapeHtml(syncError)}</div>` : ""}
      ${renderBilibiliMetricGrid(metrics)}
      ${overview?.summary ? `<p class="xhs-summary">${escapeHtml(overview.summary)}</p>` : ""}
      <section class="content-block latest-work-block wechat-latest-work-block">
        <div class="content-block-head latest-work-head">
          <h3>${text.latestWork}</h3>
          <span>${latestWork ? "1" : "0"}</span>
        </div>
        <div class="content-list compact">
          ${latestWork ? renderWechatLatestWork(latestWork, state.allSelectedAccounts, text, language) : renderEmpty(text.noWorks)}
        </div>
      </section>
    </div>
  `;
}

function renderOverviewPeriodToolbar(
  text: CopyText,
  overviewPeriod: number,
  options: { title: string; periods: OverviewPeriod[] },
) {
  return `
    <div class="xhs-overview-toolbar">
      <div class="xhs-overview-title">
        <strong>${escapeHtml(options.title)}</strong>
      </div>
      <div class="segmented-control">
        ${options.periods.map((period) => `
          <button class="${overviewPeriod === period ? "active" : ""}" type="button" data-action="overview-period" data-period="${period}">
            ${periodLabel(text, period)}
          </button>
        `).join("")}
      </div>
    </div>
  `;
}

function periodLabel(text: CopyText, period: OverviewPeriod) {
  if (period === 1) return text.yesterday;
  if (period === 7) return text.last7Days;
  if (period === 30) return text.last30Days;
  if (period === 90) return text.last90Days;
  if (period === BILIBILI_HISTORY_PERIOD) return text.history;
  return text.historyTotal;
}

function renderWechatChannelsOverviewView(state: WorkspaceTabState) {
  const { text, accountContent, language } = state;
  if (state.accountContentLoading && !accountContent) {
    return `
      <div class="xhs-overview">
        ${renderWechatOverviewTitle(text)}
        ${renderAccountContentLoading()}
      </div>
    `;
  }

  const overview = accountContent?.overviewSeven || accountContent?.overviewThirty || null;
  const syncError = accountContent?.error || overview?.error || "";
  const metrics = overview?.metrics?.length ? overview.metrics : emptyWechatMetrics(text);
  const latestVideoWork = accountContent?.latestWork || null;
  const latestArticleWork = accountContent?.latestWorkSeven || null;
  const latestWork = state.selectedWorkType === "article" ? latestArticleWork : latestVideoWork;
  return `
    <div class="xhs-overview wechat-overview">
      ${renderWechatOverviewTitle(text)}
      ${syncError ? `<div class="sync-inline-error">${escapeHtml(syncError)}</div>` : ""}
      ${renderAdaptiveMetricGrid(metrics)}
      ${overview?.summary ? `<p class="xhs-summary">${escapeHtml(overview.summary)}</p>` : ""}
      <section class="content-block latest-work-block wechat-latest-work-block">
        <div class="content-block-head latest-work-head">
          <h3>${text.latestWork}</h3>
          ${renderWorkTypeSegmented(text, state.selectedWorkType)}
        </div>
        <div class="content-list compact">
          ${latestWork ? renderWechatLatestWork(latestWork, state.allSelectedAccounts, text, language) : renderEmpty(text.noWorks)}
        </div>
      </section>
    </div>
  `;
}

function renderWechatOverviewTitle(text: CopyText) {
  return `
    <div class="xhs-overview-toolbar">
      <div class="xhs-overview-title">
        <strong>${text.wechatYesterdayOverview}</strong>
      </div>
    </div>
  `;
}

function renderWechatMetric(metric: { label: string; value?: string | null }) {
  const value = metric.value && metric.value.trim() ? metric.value : "-";
  return `
    <div class="xhs-metric-card">
      <span>${escapeHtml(metric.label)}</span>
      <strong>${escapeHtml(value)}</strong>
    </div>
  `;
}

function renderAdaptiveMetricGrid(metrics: Array<{ label: string; value?: string | null }>) {
  const columns = adaptiveMetricColumns(metrics.length);
  const tail = adaptiveMetricTail(metrics.length, columns);
  return `
    <div class="xhs-metric-grid adaptive-metric-grid metric-cols-${columns} metric-tail-${tail}">
      ${metrics.map((metric) => renderWechatMetric(metric)).join("")}
    </div>
  `;
}

function renderBilibiliMetricGrid(metrics: Array<{ label: string; value?: string | null; trend?: string | null; tone?: string | null }>) {
  const columns = adaptiveMetricColumns(metrics.length);
  const tail = adaptiveMetricTail(metrics.length, columns);
  return `
    <div class="xhs-metric-grid adaptive-metric-grid metric-cols-${columns} metric-tail-${tail}">
      ${metrics.map((metric) => renderBilibiliMetric(metric)).join("")}
    </div>
  `;
}

function renderBilibiliMetric(metric: { label: string; value?: string | null; trend?: string | null; tone?: string | null }) {
  const value = metric.value && metric.value.trim() ? metric.value : "-";
  const trend = metric.trend && metric.trend.trim() ? metric.trend : "";
  return `
    <div class="xhs-metric-card bilibili-metric-card">
      <span>${escapeHtml(metric.label)}</span>
      <strong>${escapeHtml(value)}</strong>
      ${trend ? `<em class="${metric.tone === "up" ? "up" : metric.tone === "down" ? "down" : ""}">${escapeHtml(trend)}</em>` : ""}
    </div>
  `;
}

function adaptiveMetricColumns(count: number) {
  return Math.min(Math.max(count, 1), 6);
}

function adaptiveMetricTail(count: number, columns: number) {
  const tail = count % columns;
  return tail === 0 ? columns : tail;
}

function emptyWechatMetrics(text: CopyText) {
  return [text.newFollowers, text.newPlays, text.newLikes, text.newComments]
    .map((label) => ({ label, value: "-" }));
}

function emptyDouyinMetrics() {
  return ["播放量", "主页访问", "作品点赞", "作品分享", "作品评论", "封面点击率", "净增粉丝", "取关粉丝", "总粉丝量"]
    .map((label) => ({ label, value: "-" }));
}

function emptyBilibiliMetrics() {
  return ["播放量", "累计粉丝", "点赞", "收藏", "硬币", "评论", "弹幕", "分享"]
    .map((label) => ({ label, value: "-" }));
}

function emptyKuaishouMetrics() {
  return ["播放量", "点赞量", "净增粉丝量", "完播率", "评论量", "分享量", "作品量"]
    .map((label) => ({ label, value: "-" }));
}

function overviewForPeriod(accountContent: ChannelAccountContent | null, period: OverviewPeriod) {
  if (!accountContent) return null;
  if (period === 1) return accountContent.overviewYesterday || null;
  if (period === 30) return accountContent.overviewThirty || null;
  if (period === 90) return accountContent.overviewNinety || null;
  if (period === BILIBILI_HISTORY_PERIOD) return accountContent.overviewHistory || null;
  if (period === BILIBILI_TOTAL_PERIOD) return accountContent.overviewTotal || null;
  return accountContent.overviewSeven || null;
}

function renderXhsLatestWork(work: ChannelWork, accounts: ChannelAccount[], text: CopyText, language: LanguageMode) {
  const account = accounts.find((item) => item.id === work.accountId);
  const publishedAt = work.publishedAt ? formatDate(work.publishedAt, language) : text.notSynced;
  const labels = latestWorkMetricLabels(language);
  const metrics = [
    { label: labels.impressions, value: formatOptionalContentNumber(work.impressions, language) },
    { label: labels.views, value: formatOptionalContentNumber(work.views, language) },
    { label: labels.coverClickRate, value: work.coverClickRate || "-" },
    { label: labels.avgViewTime, value: work.avgViewTime || "-" },
    { label: labels.gainedFollowers, value: formatOptionalSignedContentNumber(work.gainedFollowers, language) },
    { label: labels.likes, value: formatOptionalContentNumber(work.likes, language) },
    { label: labels.comments, value: formatOptionalContentNumber(work.comments, language) },
    { label: labels.collects, value: formatOptionalContentNumber(work.collects, language) },
    { label: labels.shares, value: formatOptionalContentNumber(work.shares, language) },
  ];

  return `
    <article class="latest-work-card">
      <div class="latest-work-media">
        ${
          work.coverUrl
            ? `<img class="latest-work-cover" src="${escapeAttribute(work.coverUrl)}" alt="" loading="eager" decoding="sync" fetchpriority="high" />`
            : `<div class="latest-work-cover placeholder">${escapeHtml(work.title.slice(0, 1) || "N")}</div>`
        }
      </div>
      <div class="latest-work-main">
        <div class="latest-work-title-row">
          <h3>${escapeHtml(work.title)}</h3>
          <span>${escapeHtml(account?.nickname || text.allAccounts)} · ${publishedAt}</span>
        </div>
        <div class="latest-work-metrics">
          ${metrics.map((metric) => `
            <span>
              <em>${escapeHtml(metric.label)}</em>
              <strong>${escapeHtml(metric.value)}</strong>
            </span>
          `).join("")}
        </div>
      </div>
    </article>
  `;
}

function renderWechatLatestWork(work: ChannelWork, accounts: ChannelAccount[], text: CopyText, language: LanguageMode) {
  const account = accounts.find((item) => item.id === work.accountId);
  const publishedAt = work.publishedAt ? formatDate(work.publishedAt, language) : text.notSynced;
  const metrics = work.metrics?.length ? work.metrics : defaultWorkMetrics(work, text, language);
  const coverUrl = normalizeDisplayImageUrl(work.coverUrl, work.platformId);
  const coverFallback = work.platformId === "bilibili"
    ? ` onerror="this.onerror=null;this.src='${escapeAttribute(BILIBILI_DEFAULT_COVER)}'"`
    : "";

  return `
    <article class="latest-work-card wechat-latest-work-card">
      <div class="latest-work-media">
        ${
          coverUrl
            ? `<img class="latest-work-cover" src="${escapeAttribute(coverUrl)}" alt="" loading="eager" decoding="sync" fetchpriority="high"${coverFallback} />`
            : `<div class="latest-work-cover placeholder">${escapeHtml(work.title.slice(0, 1) || "W")}</div>`
        }
      </div>
      <div class="latest-work-main">
        <div class="latest-work-title-row">
          <h3>${escapeHtml(work.title)}</h3>
          <span>${escapeHtml(account?.nickname || text.allAccounts)} · ${publishedAt}</span>
        </div>
        <div class="latest-work-metrics wechat-latest-metrics">
          ${metrics.map((metric) => `
            <span>
              <em>${escapeHtml(metric.label)}</em>
              <strong>${escapeHtml(metric.value && metric.value.trim() ? metric.value : "-")}</strong>
            </span>
          `).join("")}
        </div>
      </div>
    </article>
  `;
}

function renderDouyinLatestWork(work: ChannelWork, accounts: ChannelAccount[], text: CopyText, language: LanguageMode) {
  const account = accounts.find((item) => item.id === work.accountId);
  const publishedAt = work.publishedAt ? formatDate(work.publishedAt, language) : text.notSynced;
  const metrics = work.metrics?.length ? work.metrics : [
    { label: "播放量", value: formatOptionalContentNumber(work.views, language) },
    { label: "点赞量", value: formatOptionalContentNumber(work.likes, language) },
    { label: "评论量", value: formatOptionalContentNumber(work.comments, language) },
    { label: "分享量", value: formatOptionalContentNumber(work.shares, language) },
    { label: "收藏量", value: formatOptionalContentNumber(work.collects, language) },
    { label: "封面点击率", value: work.coverClickRate || "-" },
  ];

  return `
    <article class="latest-work-card douyin-latest-work-card">
      <div class="latest-work-media">
        ${
          work.coverUrl
            ? `<img class="latest-work-cover" src="${escapeAttribute(work.coverUrl)}" alt="" loading="eager" decoding="sync" fetchpriority="high" />`
            : `<div class="latest-work-cover placeholder">${escapeHtml(work.title.slice(0, 1) || "D")}</div>`
        }
      </div>
      <div class="latest-work-main">
        <div class="latest-work-title-row">
          <h3>${escapeHtml(work.title)}</h3>
          <span>${escapeHtml(account?.nickname || text.allAccounts)} · ${publishedAt}</span>
        </div>
        <div class="latest-work-metrics douyin-latest-metrics">
          ${metrics.map((metric) => `
            <span>
              <em>${escapeHtml(metric.label)}</em>
              <strong>${escapeHtml(metric.value && metric.value.trim() ? metric.value : "-")}</strong>
            </span>
          `).join("")}
        </div>
      </div>
    </article>
  `;
}

function latestWorkMetricLabels(language: LanguageMode) {
  return language === "zh"
    ? {
        impressions: "曝光数",
        views: "观看数",
        coverClickRate: "封面点击率",
        avgViewTime: "平均观看时长",
        gainedFollowers: "涨粉数",
        likes: "点赞数",
        comments: "评论数",
        collects: "收藏数",
        shares: "分享数",
      }
    : {
        impressions: "Impressions",
        views: "Views",
        coverClickRate: "Cover CTR",
        avgViewTime: "Avg view time",
        gainedFollowers: "Followers gained",
        likes: "Likes",
        comments: "Comments",
        collects: "Saves",
        shares: "Shares",
      };
}

function renderXhsMetric(metric: { label: string; value?: string | null; compareLabel?: string | null; trend?: string | null; tone?: string | null }) {
  const value = metric.value && metric.value.trim() ? metric.value : "-";
  const trend = metric.trend && metric.trend.trim() ? metric.trend : "-";
  return `
    <div class="xhs-metric-card">
      <span>${escapeHtml(metric.label)}</span>
      <strong>${escapeHtml(value)}</strong>
      <em class="${metric.tone === "up" ? "up" : metric.tone === "down" ? "down" : ""}">
        ${escapeHtml(metric.compareLabel || "环比")} ${escapeHtml(trend)}
      </em>
    </div>
  `;
}

function emptyXhsMetrics() {
  return [
    "曝光数",
    "观看数",
    "点赞数",
    "评论数",
    "净涨粉",
    "新增关注",
    "封面点击率",
    "视频完播率",
    "收藏数",
    "分享数",
    "取消关注",
    "主页访客",
  ].map((label) => ({ label, value: "-", compareLabel: "环比", trend: "-" }));
}

function renderWorksView(state: WorkspaceTabState) {
  const { text, language, works, allSelectedAccounts, worksPages, worksLoading, selectedPlatform, selectedAccount } = state;
  const isPagedAccountWorks = ["xiaohongshu", "wechat-channels", "douyin", "bilibili", "kuaishou"].includes(selectedPlatform.id) && Boolean(selectedAccount);
  const workTypeTabs = ["wechat-channels", "bilibili"].includes(selectedPlatform.id) && selectedAccount
    ? renderWorkTypeTabs(text, state.selectedWorkType)
    : "";
  if (isPagedAccountWorks && worksLoading && !works.length) {
    return `
      ${workTypeTabs}
      ${renderAccountContentLoading()}
    `;
  }

  const lastPage = worksPages[worksPages.length - 1];
  const showLoadMore = isPagedAccountWorks && lastPage?.hasMore;
  return `
    ${workTypeTabs}
    <div class="content-list">
      ${works.length ? works.map((work) => renderWorkRow(work, allSelectedAccounts, text, language)).join("") : renderEmpty(text.noWorks)}
    </div>
    ${
      isPagedAccountWorks
        ? `<div class="works-pagination">
            ${worksLoading ? `<span class="account-content-loading-spinner works-pagination-spinner" aria-hidden="true"></span>` : ""}
            ${showLoadMore ? `<button class="ghost-btn" type="button" data-action="load-more-works" ${worksLoading ? "disabled" : ""}>${text.loadMore}</button>` : ""}
            ${lastPage?.error ? `<em>${escapeHtml(lastPage.error)}</em>` : ""}
          </div>`
        : ""
    }
  `;
}

function renderWorkTypeTabs(text: CopyText, selectedWorkType: "video" | "article") {
  return `
    <div class="works-toolbar">
      ${renderWorkTypeSegmented(text, selectedWorkType)}
    </div>
  `;
}

function renderWorkTypeSegmented(text: CopyText, selectedWorkType: "video" | "article") {
  const items: Array<{ id: "video" | "article"; label: string }> = [
    { id: "video", label: text.videoWorks },
    { id: "article", label: text.articleWorks },
  ];
  return `
    <div class="segmented-control" role="tablist">
      ${items.map((item) => `
        <button class="${selectedWorkType === item.id ? "active" : ""}" type="button" data-action="work-type" data-work-type="${item.id}" role="tab" aria-selected="${selectedWorkType === item.id}">
          ${item.label}
        </button>
      `).join("")}
    </div>
  `;
}

function renderCommentsView({ text, language, comments, allSelectedAccounts }: WorkspaceTabState) {
  return `
    <div class="content-list">
      ${comments.length ? comments.map((comment) => renderCommentRow(comment, allSelectedAccounts, text, language)).join("") : renderEmpty(text.noComments)}
    </div>
  `;
}

function renderDataView({
  text,
  language,
  selectedPlatform,
  selectedAccounts,
  works,
  comments,
  totalViews,
  totalLikes,
  pendingComments,
  formatFollowersTotal,
}: WorkspaceTabState) {
  return `
    <div class="metric-grid">
      ${renderMetric(text.totalFans, formatFollowersTotal(selectedAccounts))}
      ${renderMetric(text.totalViews, formatNumber(totalViews, language))}
      ${renderMetric(text.totalLikes, formatNumber(totalLikes, language))}
      ${renderMetric(text.contentComments, formatNumber(comments.length, language))}
      ${renderMetric(text.pendingComments, formatNumber(pendingComments, language))}
      ${renderMetric(text.contentWorks, formatNumber(works.length, language))}
    </div>
    <div class="account-data-list">
      ${selectedAccounts.length ? selectedAccounts.map((account) => renderAccountDataRow(account, selectedPlatform, text, language)).join("") : renderEmpty(text.noAccountDesc)}
    </div>
  `;
}

function renderMetric(label: string, value: string) {
  return `
    <div class="metric-card">
      <span>${label}</span>
      <strong>${value}</strong>
    </div>
  `;
}

function renderWorkRow(work: ChannelWork, accounts: ChannelAccount[], text: CopyText, language: LanguageMode) {
  const account = accounts.find((item) => item.id === work.accountId);
  const publishedAt = work.publishedAt ? formatDate(work.publishedAt, language) : text.notSynced;
  const isCompactWork = ["xiaohongshu", "wechat-channels", "douyin", "bilibili", "kuaishou"].includes(work.platformId);
  const workMeta = isCompactWork ? publishedAt : `${account?.nickname || text.allAccounts} · ${publishedAt}`;
  const title = truncateWorkTitle(work.title);
  const coverUrl = normalizeDisplayImageUrl(work.coverUrl, work.platformId);
  const coverFallback = work.platformId === "bilibili"
    ? ` onerror="this.onerror=null;this.src='${escapeAttribute(BILIBILI_DEFAULT_COVER)}'"`
    : "";
  const metrics = work.metrics?.length
    ? work.metrics
    : defaultWorkMetrics(work, text, language);
  return `
    <article class="content-row has-cover${isCompactWork ? " compact-work-row" : ""}">
      ${
        coverUrl
          ? `<img class="content-cover" src="${escapeAttribute(coverUrl)}" alt="" loading="lazy" decoding="async"${coverFallback} />`
          : `<div class="content-cover placeholder">${escapeHtml(work.title.slice(0, 1) || "N")}</div>`
      }
      <div class="content-row-main">
        <div class="content-row-title">
          <h3 title="${escapeAttribute(work.title)}">${escapeHtml(title)}</h3>
          <span class="content-status status-${work.status}">${workStatusLabel(work.status, text)}</span>
          ${renderWorkTypeBadge(work)}
          ${renderWorkBadges(work)}
        </div>
        <p>${escapeHtml(workMeta)}</p>
      </div>
      <div class="content-stats">
        ${metrics.map((metric) => `
          <span><em>${escapeHtml(metric.label)}</em><strong>${escapeHtml(metric.value || "-")}</strong></span>
        `).join("")}
      </div>
    </article>
  `;
}

function normalizeDisplayImageUrl(value?: string | null, platformId?: string | null) {
  const url = value?.trim();
  if (!url && platformId === "bilibili") return BILIBILI_DEFAULT_COVER;
  if (!url) return "";
  if (url.startsWith("//")) return `https:${url}`;
  if (platformId === "bilibili") {
    const normalized = url.startsWith("http://i") && url.includes(".hdslb.com/")
      ? url.replace(/^http:\/\//, "https://")
      : url;
    if (normalized.includes(".hdslb.com/") && !normalized.includes("@")) {
      return `${normalized}@156w_98h_1c.webp`;
    }
    return normalized;
  }
  return url;
}

function renderWorkBadges(work: ChannelWork) {
  const labels = Array.from(new Set((work.badges || []).map((badge) => badge.trim()).filter(Boolean)));
  return labels
    .map((label) => `<span class="content-status ${workBadgeClass(label)}">${escapeHtml(label)}</span>`)
    .join("");
}

function renderWorkTypeBadge(work: ChannelWork) {
  const label = workTypeBadgeLabel(work.workType);
  return label ? `<span class="content-status status-work-type">${escapeHtml(label)}</span>` : "";
}

function workTypeBadgeLabel(workType?: string | null) {
  const value = workType?.trim().toLowerCase();
  if (!value) {
    return "";
  }
  if (["article", "image", "note", "picture", "photo", "pic"].includes(value)) {
    return "图文";
  }
  if (["video", "short_video", "short-video"].includes(value)) {
    return "视频";
  }
  if (value === "live") {
    return "直播";
  }
  return workType?.trim() || "";
}

function workBadgeClass(label: string) {
  if (label.includes("置顶") || label.toLowerCase().includes("top") || label.toLowerCase().includes("pin")) {
    return "status-pinned";
  }
  return "status-visibility";
}

function truncateWorkTitle(title: string) {
  const chars = Array.from(title.trim());
  if (chars.length <= 30) {
    return title.trim();
  }
  return `${chars.slice(0, 29).join("")}…`;
}

function defaultWorkMetrics(work: ChannelWork, text: CopyText, language: LanguageMode) {
  const isDouyin = work.platformId === "douyin";
  const isXhs = work.platformId === "xiaohongshu";
  const viewLabel = isDouyin
    ? (language === "zh" ? "播放" : "Plays")
    : isXhs
      ? (language === "zh" ? "观看" : "Views")
      : text.totalViews;
  return [
    { label: viewLabel, value: formatOptionalContentNumber(work.views, language) },
    { label: text.totalLikes, value: formatOptionalContentNumber(work.likes, language) },
    { label: text.contentComments, value: formatOptionalContentNumber(work.comments, language) },
  ];
}

function renderCommentRow(comment: ChannelComment, accounts: ChannelAccount[], text: CopyText, language: LanguageMode) {
  const account = accounts.find((item) => item.id === comment.accountId);
  return `
    <article class="content-row comment-row">
      <div class="content-row-main">
        <div class="content-row-title">
          <h3>${escapeHtml(comment.author)}</h3>
          <span class="content-status status-${comment.status}">${commentStatusLabel(comment.status, text)}</span>
        </div>
        <p>${escapeHtml(comment.content)}</p>
        <em>${escapeHtml(account?.nickname || text.allAccounts)} · ${formatDate(comment.createdAt, language)} · ${sentimentLabel(comment.sentiment, text)}</em>
      </div>
    </article>
  `;
}

function renderPlatformAccountRow(
  account: ChannelAccount,
  platform: PlatformInfo,
  text: CopyText,
  language: LanguageMode,
) {
  const likesLabel = platform.id === "xiaohongshu" ? text.likesAndFavorites : text.likesLabel;
  const likesPending = platform.id === "xiaohongshu" ? text.likesAndFavoritesPending : text.likesPending;
  return `
    <article class="account-data-row account-data-action" data-account="${escapeAttribute(account.id)}">
      <div class="account-data-main">
        ${renderAccountAvatar(account, undefined, escapeHtml(account.nickname.slice(0, 1)), "account-nav-avatar")}
        <div>
          <div class="account-data-title">
            <strong>${escapeHtml(account.nickname)}</strong>
            <span class="account-status-pill ${account.status}">${statusLabel(account.status, text)}</span>
          </div>
          <span>${platformAccountValue(account, platform, language)}</span>
        </div>
      </div>
      <div class="account-data-metrics">
        <span>${formatFollowerListMetric(account.followers, language)}</span>
        <span>${formatOptionalMetric(account.following, text.followingPending, text.followingLabel, language)}</span>
        <span>${formatOptionalMetric(account.likes, likesPending, likesLabel, language)}</span>
        <span>${accountSyncText(account, text, language)}</span>
      </div>
    </article>
  `;
}

function renderAccountDataRow(
  account: ChannelAccount,
  platform: PlatformInfo,
  text: CopyText,
  language: LanguageMode,
) {
  return `
    <article class="account-data-row">
      <div class="account-data-main">
        ${renderAccountAvatar(account, undefined, escapeHtml(account.nickname.slice(0, 1)), "account-nav-avatar")}
        <div>
          <strong>${escapeHtml(account.nickname)}</strong>
          ${renderPlatformAccountMeta(platform, account, text, language)}
        </div>
      </div>
      <div class="account-data-metrics">
        <span>${formatFollowers(account.followers, language)}</span>
        <span>${formatOptionalMetric(account.following, text.followingPending, text.followingLabel, language)}</span>
        <span>${formatOptionalMetric(account.likes, text.likesPending, text.likesLabel, language)}</span>
        <span>${accountSyncText(account, text, language)}</span>
      </div>
    </article>
  `;
}

function renderEmpty(message: string) {
  return `<div class="content-empty">${message}</div>`;
}

function renderAccountContentLoading() {
  return `
    <div class="account-content-loading" aria-live="polite">
      <span class="account-content-loading-spinner" aria-hidden="true"></span>
    </div>
  `;
}

function workStatusLabel(status: ChannelWork["status"], text: CopyText) {
  if (status === "published") return text.workStatusPublished;
  if (status === "reviewing") return text.workStatusReviewing;
  return text.workStatusDraft;
}

function commentStatusLabel(status: ChannelComment["status"], text: CopyText) {
  if (status === "unread") return text.commentStatusUnread;
  if (status === "replied") return text.commentStatusReplied;
  return text.commentStatusRisk;
}

function sentimentLabel(sentiment: ChannelComment["sentiment"], text: CopyText) {
  if (sentiment === "positive") return text.sentimentPositive;
  if (sentiment === "risk") return text.sentimentRisk;
  return text.sentimentNeutral;
}

function formatOptionalMetric(
  value: number | null | undefined,
  _pendingText: string,
  unit: string,
  language: LanguageMode,
) {
  if (typeof value !== "number") return "-";
  return `${formatNumber(value, language)} ${unit}`;
}

function formatFollowerListMetric(value: number | null | undefined, language: LanguageMode) {
  if (typeof value !== "number") return "-";
  const count = formatNumber(value, language);
  if (language === "zh") return `${count} 粉丝`;
  return `${count} ${value === 1 ? "follower" : "followers"}`;
}

function formatOptionalNumber(value: number | null | undefined, _pendingText: string, language: LanguageMode) {
  if (typeof value !== "number") return "-";
  return formatNumber(value, language);
}

function formatOptionalContentNumber(value: number | null | undefined, language: LanguageMode) {
  if (typeof value !== "number") return "-";
  return formatNumber(value, language);
}

function formatOptionalSignedContentNumber(value: number | null | undefined, language: LanguageMode) {
  if (typeof value !== "number") return "-";
  if (value === 0) return "0";
  return `${value > 0 ? "+" : "-"}${formatNumber(Math.abs(value), language)}`;
}

function formatAccountMetricTotal(
  accounts: ChannelAccount[],
  key: "following" | "likes",
  language: LanguageMode,
) {
  const values = accounts
    .map((account) => account[key])
    .filter((value): value is number => typeof value === "number");
  if (!values.length) return "-";
  return formatNumber(values.reduce((sum, value) => sum + value, 0), language);
}

function accountSyncText(account: ChannelAccount, text: CopyText, language: LanguageMode) {
  return account.lastSyncAt ? `${text.syncedAt} ${formatDate(account.lastSyncAt, language)}` : text.notSynced;
}

function lastSyncValue(account: ChannelAccount, text: CopyText, language: LanguageMode) {
  return account.lastSyncAt ? formatDate(account.lastSyncAt, language) : text.notSynced;
}

function latestSyncText(accounts: ChannelAccount[], text: CopyText, language: LanguageMode) {
  const latest = accounts
    .map((account) => (account.lastSyncAt ? Date.parse(account.lastSyncAt) : NaN))
    .filter((value) => Number.isFinite(value))
    .sort((a, b) => b - a)[0];
  if (!latest) return text.notSynced;
  return `${text.syncedAt} ${formatDate(new Date(latest).toISOString(), language)}`;
}

function renderPlatformAccountMeta(
  platform: PlatformInfo,
  account: ChannelAccount,
  text: CopyText,
  language: LanguageMode,
) {
  const accountValue = account.uid.trim();
  return `
    <span class="platform-account-meta">
      <span>${platformAccountText(platform, account, language)}</span>
      ${
        accountValue
          ? `<button class="account-copy-btn" type="button" data-copy-account="${escapeAttribute(accountValue)}" title="${escapeAttribute(text.copyAccount)}" aria-label="${escapeAttribute(text.copyAccount)}">${icon("copy")}</button>`
          : ""
      }
    </span>
  `;
}

function platformAccountText(platform: PlatformInfo, account: ChannelAccount, language: LanguageMode) {
  const label = platformAccountLabel(platform, language);
  const accountValue = account.uid.trim();
  return escapeHtml(accountValue ? `${label}: ${accountValue}` : label);
}

function platformAccountValue(account: ChannelAccount, platform: PlatformInfo, language: LanguageMode) {
  const accountValue = account.uid.trim();
  return escapeHtml(accountValue || platformAccountLabel(platform, language));
}

function platformAccountLabel(platform: PlatformInfo, language: LanguageMode) {
  if (platform.id === "douyin") {
    return language === "zh" ? "抖音号" : "Douyin ID";
  }
  return language === "zh" ? `${platform.name}账号` : `${platform.name} account`;
}

function formatNumber(value: number, language: LanguageMode) {
  if (language === "zh" && value >= 10000) return `${(value / 10000).toFixed(1)}万`;
  return new Intl.NumberFormat(language === "zh" ? "zh-CN" : "en-US").format(value);
}
