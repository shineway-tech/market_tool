import type { ChannelAccount, PlatformInfo } from "../domain/types";
import type { CopyText } from "../i18n/copy";
import { icon } from "../ui/icons";
import { platformLogo } from "../ui/platform-logo";

export interface ChannelsPageState {
  text: CopyText;
  selectedPlatform: PlatformInfo;
  selectedAccounts: ChannelAccount[];
  connectedCount: number;
  activeAccountCount: number;
  platformRefreshing: boolean;
  platforms: PlatformInfo[];
  platformItem: (platform: PlatformInfo) => string;
  accountItem: (account: ChannelAccount) => string;
  emptyAccounts: (platform: PlatformInfo) => string;
  formatFollowersTotal: (items: ChannelAccount[]) => string;
}

export function renderChannelsPage({
  text,
  selectedPlatform,
  selectedAccounts,
  connectedCount,
  activeAccountCount,
  platformRefreshing,
  platforms,
  platformItem,
  accountItem,
  emptyAccounts,
  formatFollowersTotal,
}: ChannelsPageState) {
  return `
    <section class="channel-head">
      <div>
        <h1>${text.channelsTitle}</h1>
      </div>
    </section>

    <section class="channel-layout">
      <aside class="platform-panel">
        <div class="panel-title">
          <span>${text.platformPanel}</span>
          <strong>${connectedCount}</strong>
        </div>
        <div class="platform-list">
          ${platforms.map((platform) => platformItem(platform)).join("")}
        </div>
      </aside>

      <section class="account-panel">
        <div class="account-panel-head">
          <div class="selected-platform">
            ${platformLogo(selectedPlatform, "large")}
            <div>
              <h2>${selectedPlatform.name}</h2>
            </div>
          </div>
          <div class="account-actions">
            <button class="ghost-btn ${platformRefreshing ? "is-loading" : ""}" type="button" data-action="refresh-platform" ${platformRefreshing ? "disabled" : ""}>${icon("refresh")}${platformRefreshing ? text.refreshing : text.refresh}</button>
          </div>
        </div>
        <div class="account-summary">
          <div><span>${text.accountPanel}</span><strong>${selectedAccounts.length}</strong></div>
          <div><span>${text.availableAccount}</span><strong>${activeAccountCount}</strong></div>
          <div><span>${text.totalFans}</span><strong>${formatFollowersTotal(selectedAccounts)}</strong></div>
        </div>
        <div class="account-list">
          ${
            selectedAccounts.length
              ? selectedAccounts.map((account) => accountItem(account)).join("")
              : emptyAccounts(selectedPlatform)
          }
        </div>
      </section>
    </section>
  `;
}
