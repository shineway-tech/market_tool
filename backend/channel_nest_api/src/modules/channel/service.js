const crypto = require('crypto');
const { BadArgumentError, NotFoundError } = require('@honeykid/ml/errors');
const repository = require('./repository');
const {
  AccountStatus,
  AuthMode,
  AuthTaskStatus,
  CreatorHomeUrls,
  LocalLoginUrls,
  PlatformIds,
} = require('./constants');

function firstString(value, keys) {
  if (!value || typeof value !== 'object') {
    return null;
  }

  for (const key of keys) {
    if (typeof value[key] === 'string' && value[key].trim()) {
      return value[key].trim();
    }
  }

  return null;
}

function firstCount(value, keys) {
  if (!value || typeof value !== 'object') {
    return null;
  }

  for (const key of keys) {
    const count = Number(value[key]);

    if (Number.isFinite(count) && count >= 0) {
      return Math.trunc(count);
    }
  }

  return null;
}

function parseDate(value) {
  if (!value) {
    return null;
  }

  const date = new Date(value);

  return Number.isNaN(date.getTime()) ? null : date;
}

function normalizePlatformId(value) {
  const platform = String(value || '').trim();

  if (['xhs', 'rednote', 'xiaohongshu'].includes(platform)) {
    return PlatformIds.Xiaohongshu;
  }

  if (['wxSph', 'wxsph', 'wechat-channels'].includes(platform)) {
    return PlatformIds.WechatChannels;
  }

  if (['KWAI', 'kwai', 'kuaishou'].includes(platform)) {
    return PlatformIds.Kuaishou;
  }

  return platform;
}

function accountHomeUrl(account) {
  if (account.homepage_url) {
    return account.homepage_url;
  }

  if (account.platform_id === PlatformIds.Douyin) {
    return CreatorHomeUrls[PlatformIds.Douyin];
  }

  if (account.platform_id === PlatformIds.Kuaishou) {
    return CreatorHomeUrls[PlatformIds.Kuaishou];
  }

  if (account.platform_id === PlatformIds.Bilibili) {
    return CreatorHomeUrls[PlatformIds.Bilibili];
  }

  if (account.platform_id === PlatformIds.Xiaohongshu) {
    return CreatorHomeUrls[PlatformIds.Xiaohongshu];
  }

  if (account.platform_id === PlatformIds.WechatChannels) {
    return CreatorHomeUrls[PlatformIds.WechatChannels];
  }

  return '';
}

function toClientPlatform(platform) {
  return {
    id: platform.platform_id,
    name: platform.name,
    slug: platform.slug,
    color: platform.color,
    description: platform.description,
    supportsBuiltinOauth: Boolean(platform.supports_builtin_oauth),
    authMode: platform.auth_mode,
    sortNo: platform.sort_no,
  };
}

function toClientAccount(account) {
  if (!account) {
    return null;
  }

  return {
    id: account.id,
    platformId: account.platform_id,
    uid: account.platform_uid,
    nickname: account.nickname,
    avatar: account.avatar || '',
    followers: account.followers === null || account.followers === undefined
      ? null
      : Number(account.followers),
    likes: account.likes === null || account.likes === undefined
      ? null
      : Number(account.likes),
    status: account.status,
    homepageUrl: accountHomeUrl(account),
    createdAt: account.created_at,
    updatedAt: account.updated_at,
    lastSyncAt: account.last_sync_at,
  };
}

class ChannelService {
  async bootstrap(userId) {
    const [platforms, accounts] = await Promise.all([
      repository.listPlatforms(),
      repository.listAccounts(userId),
    ]);

    return {
      platforms: platforms.map(toClientPlatform),
      accounts: accounts.map(toClientAccount),
    };
  }

  async listPlatforms() {
    const platforms = await repository.listPlatforms();

    return platforms.map(toClientPlatform);
  }

  async listAccounts(userId, entries) {
    const accounts = await repository.listAccounts(userId, entries);

    return accounts.map(toClientAccount);
  }

  async upsertAccount(userId, entries) {
    const platform = await repository.getPlatform(entries.platform_id);

    if (!platform) {
      throw new BadArgumentError('平台不存在');
    }

    const account = await repository.upsertAccount({
      platform_uid: entries.platform_uid,
    }, {
      id: crypto.randomUUID(),
      user_id: userId,
      platform_id: entries.platform_id,
      platform_uid: entries.platform_uid,
      nickname: entries.nickname,
      avatar: entries.avatar || '',
      followers: entries.followers,
      likes: entries.likes,
      status: entries.status || AccountStatus.Active,
      access_token: entries.access_token || null,
      refresh_token: entries.refresh_token || null,
      token_expires_at: parseDate(entries.token_expires_at),
      token_payload: entries.token_payload || null,
      login_cookie: entries.login_cookie || null,
      webview_session_id: entries.webview_session_id || null,
      homepage_url: entries.homepage_url || null,
      last_sync_at: new Date(),
    });

    return toClientAccount(account);
  }

  async startAuth(userId, entries) {
    const platform = await repository.getPlatform(entries.platform_id);

    if (!platform) {
      throw new BadArgumentError('平台不存在');
    }

    const taskId = crypto.randomUUID();
    const mode = platform.auth_mode || AuthMode.Creator;
    let session = {
      url: LocalLoginUrls[platform.platform_id] || '',
      auth_type: 'webview',
      session_id: null,
      expires_at: new Date(Date.now() + 10 * 60 * 1000),
      instructions: null,
      payload: null,
    };

    await repository.createAuthTask({
      id: taskId,
      user_id: userId,
      platform_id: platform.platform_id,
      mode,
      auth_type: session.auth_type,
      url: session.url,
      callback_url: entries.callback_url || null,
      status: AuthTaskStatus.Pending,
      expires_at: session.expires_at,
      payload_json: session.payload || null,
    });

    return {
      taskId,
      url: session.url,
      callbackUrl: entries.callback_url || null,
      mode,
      authType: session.auth_type,
      sessionId: session.session_id,
      expiresAt: session.expires_at,
      instructions: session.instructions,
    };
  }

  async getAuthStatus(userId, taskId) {
    const task = await repository.getAuthTask(userId, taskId);

    if (!task) {
      throw new NotFoundError('授权任务不存在');
    }

    if (task.status !== AuthTaskStatus.Pending) {
      const account = task.account_id ? await repository.getAccount(userId, task.account_id) : null;

      return {
        taskId,
        status: task.status,
        account: toClientAccount(account),
        message: task.message,
      };
    }

    return {
      taskId,
      status: AuthTaskStatus.Pending,
      account: null,
      message: task.message || '请在打开的官方页面完成登录。',
    };
  }

  async completeAuth(userId, taskId, entries) {
    const task = await repository.getAuthTask(userId, taskId);

    if (!task) {
      throw new NotFoundError('授权任务不存在');
    }

    const account = await this.upsertAccount(userId, {
      ...entries,
      platform_id: task.platform_id,
    });

    await repository.updateAuthTask(userId, taskId, {
      status: AuthTaskStatus.Success,
      account_id: account.id,
      message: '授权成功',
      payload_json: entries.token_payload || null,
    });

    return account;
  }

  async refreshAccount(userId, accountId) {
    const account = await repository.getAccount(userId, accountId);

    if (!account) {
      throw new NotFoundError('账号不存在');
    }

    const updated = await repository.updateAccount(userId, accountId, {
      followers: account.followers,
      likes: account.likes,
      status: account.status,
      last_sync_at: new Date(),
    });

    await repository.createSyncLog({
      user_id: userId,
      account_id: accountId,
      action: 'refresh',
      status: 'success',
      message: '账号状态已刷新',
    });

    return toClientAccount(updated);
  }

  async deleteAccount(userId, accountId) {
    const account = await repository.getAccount(userId, accountId);

    if (!account) {
      throw new NotFoundError('账号不存在');
    }

    await repository.deleteAccount(userId, accountId);
  }

  async getAccountHomepage(userId, accountId) {
    const account = await repository.getAccount(userId, accountId);

    if (!account) {
      throw new NotFoundError('账号不存在');
    }

    return {
      url: accountHomeUrl(account),
    };
  }

}

module.exports = new ChannelService();
