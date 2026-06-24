const axios = require('axios');
const crypto = require('crypto');
const { BadArgumentError, NotFoundError } = require('@honeykid/ml/errors');
const repository = require('./repository');
const channelConfig = require('./config');
const {
  AccountStatus,
  AuthMode,
  AuthTaskStatus,
  CreatorHomeUrls,
  FollowerCountKeys,
  LocalLoginUrls,
  PlatformIds,
  RelayPlatformIds,
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

function relayResponseData(value) {
  return value && value.data !== undefined ? value.data : value;
}

function relayErrorMessage(value) {
  return firstString(value, ['message', 'msg', 'error', 'err_msg']) || 'Relay 请求失败';
}

function ensureRelaySuccess(value) {
  if (value && Number.isInteger(value.code) && value.code !== 0) {
    throw new BadArgumentError(relayErrorMessage(value));
  }
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

function normalizeKuaishouAuthUrl(url) {
  if (!url) {
    return url;
  }

  try {
    const parsed = new URL(url);

    if (parsed.hostname === 'open.kuaishou.com' && parsed.pathname === '/oauth2/authorize') {
      parsed.searchParams.set('ua', 'pc');
      return parsed.toString();
    }
  } catch (error) {
    return url;
  }

  return url;
}

function encodePathSegment(value) {
  return encodeURIComponent(String(value));
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
    relayPlatformId: platform.relay_platform_id,
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
    status: account.status,
    relayAccountRef: account.relay_account_ref || null,
    homepageUrl: accountHomeUrl(account),
    createdAt: account.created_at,
    updatedAt: account.updated_at,
    lastSyncAt: account.last_sync_at,
  };
}

function relayAccountFromValue(value) {
  const platformId = normalizePlatformId(firstString(value, ['type', 'platform', 'platformId', 'platform_id']));
  const relayAccountRef = firstString(value, ['id', 'accountId', 'account_id']);
  const platformUid = firstString(value, ['uid', 'platformUid', 'platform_uid', 'openId', 'open_id'])
    || relayAccountRef;
  const nickname = firstString(value, ['nickname', 'name', 'displayName', 'display_name'])
    || platformId;

  if (!platformId || !platformUid) {
    return null;
  }

  return {
    platform_id: platformId,
    platform_uid: platformUid,
    nickname,
    avatar: firstString(value, [
      'avatar',
      'avatarUrl',
      'avatar_url',
      'headImg',
      'headImgUrl',
      'head_img',
      'profileImageUrl',
      'profile_image_url',
      'image',
      'imageUrl',
      'image_url',
    ]) || '',
    followers: firstCount(value, FollowerCountKeys),
    status: Number(value.status) === 0 ? AccountStatus.Expired : AccountStatus.Active,
    relay_account_ref: relayAccountRef,
    token_payload: value,
    last_sync_at: new Date(),
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
      relay: {
        enabled: Boolean(channelConfig.relay.enabled),
        serverUrl: channelConfig.relay.server_url,
      },
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
      relay_account_ref: entries.relay_account_ref,
    }, {
      id: crypto.randomUUID(),
      user_id: userId,
      platform_id: entries.platform_id,
      platform_uid: entries.platform_uid,
      nickname: entries.nickname,
      avatar: entries.avatar || '',
      followers: entries.followers,
      status: entries.status || AccountStatus.Active,
      relay_account_ref: entries.relay_account_ref || null,
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
    const mode = platform.auth_mode || AuthMode.Relay;
    let session = {
      url: LocalLoginUrls[platform.platform_id] || '',
      auth_type: 'webview',
      session_id: null,
      expires_at: new Date(Date.now() + 10 * 60 * 1000),
      instructions: null,
      payload: null,
    };

    if (mode === AuthMode.Relay) {
      session = await this.createRelayAuthSession(platform);
    }

    await repository.createAuthTask({
      id: taskId,
      user_id: userId,
      platform_id: platform.platform_id,
      mode,
      auth_type: session.auth_type,
      relay_session_id: session.session_id,
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

    if (task.mode !== AuthMode.Relay || !task.relay_session_id) {
      return {
        taskId,
        status: AuthTaskStatus.Pending,
        account: null,
        message: task.message || '请在打开的官方页面完成登录。',
      };
    }

    const statusValue = await this.relayGet(
      `v2/channels/accounts/auth/${encodePathSegment(RelayPlatformIds[task.platform_id])}/status/${encodePathSegment(task.relay_session_id)}`,
    );
    const data = relayResponseData(statusValue);
    const remoteStatus = firstString(data, ['status']) || firstString(statusValue, ['status']);

    if (!['completed', 'success'].includes(remoteStatus)) {
      return {
        taskId,
        status: AuthTaskStatus.Pending,
        account: null,
        message: '还没有收到平台授权结果。',
      };
    }

    const account = await this.upsertRelayAuthorizedAccount(userId, task.platform_id, data);
    const updatedTask = await repository.updateAuthTask(userId, taskId, {
      status: AuthTaskStatus.Success,
      account_id: account.id,
      message: '授权成功',
      payload_json: statusValue,
    });

    return {
      taskId,
      status: updatedTask.status,
      account: toClientAccount(account),
      message: updatedTask.message,
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

    let { followers } = account;
    let { status } = account;

    if (account.relay_account_ref && channelConfig.relay.enabled && channelConfig.relay.api_key) {
      try {
        const value = await this.relayGet(
          `v2/channels/accounts/${encodePathSegment(account.relay_account_ref)}/analytics`,
        );
        const data = relayResponseData(value);

        followers = firstCount(data, FollowerCountKeys)
          || firstCount(value, FollowerCountKeys)
          || followers;
        status = AccountStatus.Active;
      } catch (error) {
        await repository.createSyncLog({
          user_id: userId,
          account_id: accountId,
          action: 'refresh',
          status: 'failed',
          message: error.message,
        });
      }
    }

    const updated = await repository.updateAccount(userId, accountId, {
      followers,
      status,
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

  async syncRelayAccounts(userId) {
    const value = await this.relayGet('v2/channels/accounts');
    const data = relayResponseData(value);
    const list = Array.isArray(data) ? data : data.list || [];

    const accounts = await Promise.all(list.map(async (item) => {
      const remote = relayAccountFromValue(item);

      if (!remote) {
        return null;
      }

      const account = await repository.upsertAccount({
        platform_uid: remote.platform_uid,
        relay_account_ref: remote.relay_account_ref,
      }, {
        id: crypto.randomUUID(),
        user_id: userId,
        ...remote,
      });

      return toClientAccount(account);
    }));

    return accounts.filter(Boolean);
  }

  async createRelayAuthSession(platform) {
    if (!channelConfig.relay.enabled || !channelConfig.relay.api_key) {
      throw new BadArgumentError('Relay 授权服务未配置');
    }

    const relayPlatformId = platform.relay_platform_id || RelayPlatformIds[platform.platform_id];
    const value = await this.relayGet(`v2/channels/accounts/auth/${encodePathSegment(relayPlatformId)}`);
    const data = relayResponseData(value);
    let url = firstString(data, ['url', 'uri']);

    if (!url) {
      throw new BadArgumentError('授权响应缺少 URL');
    }

    if (platform.platform_id === PlatformIds.Kuaishou) {
      url = normalizeKuaishouAuthUrl(url);
    }

    return {
      url,
      auth_type: url.startsWith('data:image') ? 'qrcode' : 'oauth',
      session_id: firstString(data, ['sessionId', 'session_id']),
      expires_at: parseDate(firstString(data, ['expiresAt', 'expires_at'])) || new Date(Date.now() + 5 * 60 * 1000),
      instructions: data.authInstructions && (
        data.authInstructions['zh-CN']
        || data.authInstructions.zh
        || data.authInstructions['en-US']
        || data.authInstructions.en
      ),
      payload: value,
    };
  }

  async upsertRelayAuthorizedAccount(userId, platformId, authStatusData) {
    const source = authStatusData.account
      || authStatusData.channelAccount
      || authStatusData;
    const remote = relayAccountFromValue(source);

    if (!remote || remote.platform_id !== platformId) {
      const accounts = await this.syncRelayAccounts(userId);
      const latest = accounts
        .filter((item) => item.platformId === platformId)
        .sort((a, b) => new Date(b.updatedAt) - new Date(a.updatedAt))[0];

      if (!latest) {
        throw new BadArgumentError('授权已完成，但没有同步到账号信息');
      }

      return {
        id: latest.id,
        platform_id: latest.platformId,
        platform_uid: latest.uid,
        nickname: latest.nickname,
        avatar: latest.avatar,
        followers: latest.followers,
        status: latest.status,
        relay_account_ref: latest.relayAccountRef,
        homepage_url: latest.homepageUrl,
        created_at: latest.createdAt,
        updated_at: latest.updatedAt,
        last_sync_at: latest.lastSyncAt,
      };
    }

    const account = await repository.upsertAccount({
      platform_uid: remote.platform_uid,
      relay_account_ref: remote.relay_account_ref,
    }, {
      id: crypto.randomUUID(),
      user_id: userId,
      ...remote,
    });

    return account;
  }

  async relayGet(path, params = {}) {
    if (!channelConfig.relay.enabled || !channelConfig.relay.api_key) {
      throw new BadArgumentError('Relay 授权服务未配置');
    }

    const base = channelConfig.relay.server_url.replace(/\/+$/, '');
    const url = `${base}/${path.replace(/^\/+/, '')}`;
    const response = await axios.get(url, {
      params,
      timeout: channelConfig.relay.timeout,
      headers: {
        'x-api-key': channelConfig.relay.api_key,
        'Accept-Language': 'zh-CN',
      },
    });

    ensureRelaySuccess(response.data);

    return response.data;
  }
}

module.exports = new ChannelService();
