import type { ChannelAccount } from "../domain/types";

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
