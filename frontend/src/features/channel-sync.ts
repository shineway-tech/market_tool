import type { ChannelAccount } from "../domain/types";

export function accountBackendPayload(account: ChannelAccount) {
  return {
    platform_id: account.platformId,
    platform_uid: account.uid,
    nickname: account.nickname,
    avatar: account.avatar,
    followers: account.followers ?? null,
    likes: account.likes ?? null,
    status: account.status,
    homepage_url: "",
  };
}

export function upsertAccount(
  accounts: ChannelAccount[],
  updated: ChannelAccount,
  currentUserId?: string,
) {
  if (currentUserId && updated.userId && updated.userId !== currentUserId) {
    return accounts;
  }

  let found = false;
  const nextAccounts = accounts.map((item) => {
    if (item.id !== updated.id) return item;
    found = true;
    return updated;
  });

  if (found || (updated.userId && updated.userId !== currentUserId)) {
    return nextAccounts;
  }

  return [updated, ...nextAccounts];
}

export async function mirrorAccounts(
  accounts: ChannelAccount[],
  syncAccount: (account: ChannelAccount) => Promise<unknown>,
) {
  if (!accounts.length) return;
  await Promise.allSettled(accounts.map(syncAccount));
}
