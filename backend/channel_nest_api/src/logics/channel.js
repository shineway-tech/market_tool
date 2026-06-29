const crypto = require('crypto');
const { BadArgumentError, NotFoundError } = require('@honeykid/ml/errors');
const {
  AuthMode,
  AuthTaskStatus,
  CreatorHomeUrls,
  DefaultPlatforms,
  PlatformIds,
} = require('../utils/channel_constants');

function normalizePlatformId(value) {
  const platform = String(value || '').trim();

  if (['xhs', 'rednote', PlatformIds.Xiaohongshu].includes(platform)) {
    return PlatformIds.Xiaohongshu;
  }

  if (['wxSph', 'wxsph', 'wechat', PlatformIds.WechatChannels].includes(platform)) {
    return PlatformIds.WechatChannels;
  }

  if (['KWAI', 'kwai', PlatformIds.Kuaishou].includes(platform)) {
    return PlatformIds.Kuaishou;
  }

  return platform;
}

function platformById(platformId) {
  const normalized = normalizePlatformId(platformId);

  return DefaultPlatforms.find((item) => item.platform_id === normalized);
}

function toClientPlatform(platform) {
  return {
    id: platform.platform_id,
    name: platform.name,
    slug: platform.slug,
    color: platform.color,
    description: platform.description,
    authMode: platform.auth_mode,
    sortNo: platform.sort_no,
  };
}

function stableAccountId(userId, entries) {
  const raw = [
    userId,
    entries.platform_id,
    entries.platform_uid,
  ].join(':');

  return `local-${crypto.createHash('sha1').update(raw).digest('hex').slice(0, 24)}`;
}

function toClientAccount(userId, entries) {
  const platformId = normalizePlatformId(entries.platform_id);
  const now = new Date();

  return {
    id: entries.id || stableAccountId(userId, {
      ...entries,
      platform_id: platformId,
    }),
    userId,
    platformId,
    uid: entries.platform_uid,
    nickname: entries.nickname,
    avatar: entries.avatar || '',
    followers: entries.followers === undefined ? null : entries.followers,
    likes: entries.likes === undefined ? null : entries.likes,
    status: entries.status || 'active',
    homepageUrl: entries.homepage_url || CreatorHomeUrls[platformId] || '',
    createdAt: entries.created_at || now,
    updatedAt: entries.updated_at || now,
    lastSyncAt: entries.last_sync_at || now,
  };
}

class ChannelLogic {
  static async bootstrap() {
    return {
      platforms: DefaultPlatforms.map(toClientPlatform),
      accounts: [],
    };
  }

  static async listPlatforms() {
    return DefaultPlatforms.map(toClientPlatform);
  }

  static async listAccounts() {
    return [];
  }

  static async upsertAccount(userId, entries) {
    const platform = platformById(entries.platform_id);

    if (!platform) {
      throw new BadArgumentError('平台不存在');
    }

    return toClientAccount(userId, {
      ...entries,
      platform_id: platform.platform_id,
    });
  }

  static async startAuth(userId, entries) {
    const platform = platformById(entries.platform_id);

    if (!platform) {
      throw new BadArgumentError('平台不存在');
    }

    const taskId = crypto.randomUUID();

    return {
      taskId,
      url: CreatorHomeUrls[platform.platform_id] || '',
      callbackUrl: entries.callback_url || null,
      mode: AuthMode.Creator,
      authType: 'local-client',
      sessionId: null,
      expiresAt: new Date(Date.now() + 10 * 60 * 1000),
      instructions: '平台账号授权由桌面客户端本地完成，服务端不保存平台账号或 Cookie。',
      userId,
    };
  }

  static async getAuthStatus(userId, taskId) {
    if (!taskId) {
      throw new NotFoundError('授权任务不存在');
    }

    return {
      taskId,
      status: AuthTaskStatus.Failed,
      account: null,
      message: '平台账号授权由桌面客户端本地完成。',
      userId,
    };
  }

  static async completeAuth() {
    throw new BadArgumentError('平台账号授权由桌面客户端本地完成，服务端不保存授权结果。');
  }

  static async refreshAccount() {
    throw new BadArgumentError('平台账号刷新由桌面客户端本地完成。');
  }

  static async deleteAccount() {
    return null;
  }

  static async getAccountHomepage(userId, accountId) {
    if (!accountId) {
      throw new NotFoundError('账号不存在');
    }

    return {
      url: '',
      userId,
    };
  }
}

module.exports = ChannelLogic;
