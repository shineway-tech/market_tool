const PlatformIds = {
  Xiaohongshu: 'xiaohongshu',
  WechatChannels: 'wechat-channels',
  Douyin: 'douyin',
  Bilibili: 'bilibili',
  Kuaishou: 'kuaishou',
};

const RelayPlatformIds = {
  [PlatformIds.Xiaohongshu]: 'xhs',
  [PlatformIds.WechatChannels]: 'wxSph',
  [PlatformIds.Douyin]: 'douyin',
  [PlatformIds.Bilibili]: 'bilibili',
  [PlatformIds.Kuaishou]: 'KWAI',
};

const AccountStatus = {
  Active: 'active',
  Expired: 'expired',
  Pending: 'pending',
};

const AuthTaskStatus = {
  Pending: 'pending',
  Success: 'success',
  Failed: 'failed',
};

const AuthMode = {
  Local: 'local',
  Relay: 'relay',
  OAuth: 'oauth',
};

const DefaultPlatforms = [
  {
    platform_id: PlatformIds.Xiaohongshu,
    relay_platform_id: RelayPlatformIds[PlatformIds.Xiaohongshu],
    name: '小红书',
    slug: 'XHS',
    color: '#ff2442',
    description: '添加并管理多个小红书账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Local,
    sort_no: 10,
  },
  {
    platform_id: PlatformIds.WechatChannels,
    relay_platform_id: RelayPlatformIds[PlatformIds.WechatChannels],
    name: '视频号',
    slug: 'WX',
    color: '#ff9f2e',
    description: '添加并管理多个微信视频号账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Local,
    sort_no: 20,
  },
  {
    platform_id: PlatformIds.Douyin,
    relay_platform_id: RelayPlatformIds[PlatformIds.Douyin],
    name: '抖音',
    slug: 'DY',
    color: '#111111',
    description: '添加并管理多个抖音账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Relay,
    sort_no: 30,
  },
  {
    platform_id: PlatformIds.Bilibili,
    relay_platform_id: RelayPlatformIds[PlatformIds.Bilibili],
    name: '哔哩哔哩',
    slug: 'BILI',
    color: '#00a1d6',
    description: '添加并管理多个 B 站账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Relay,
    sort_no: 40,
  },
  {
    platform_id: PlatformIds.Kuaishou,
    relay_platform_id: RelayPlatformIds[PlatformIds.Kuaishou],
    name: '快手',
    slug: 'KS',
    color: '#ff4906',
    description: '添加并管理多个快手账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Relay,
    sort_no: 50,
  },
];

const LocalLoginUrls = {
  [PlatformIds.Xiaohongshu]: 'https://creator.xiaohongshu.com/',
  [PlatformIds.WechatChannels]: 'https://channels.weixin.qq.com/platform',
};

const CreatorHomeUrls = {
  [PlatformIds.Xiaohongshu]: 'https://creator.xiaohongshu.com/',
  [PlatformIds.WechatChannels]: 'https://channels.weixin.qq.com/platform',
  [PlatformIds.Douyin]: 'https://creator.douyin.com/creator-micro/home?enter_from=dou_web',
  [PlatformIds.Bilibili]: 'https://member.bilibili.com/platform/home',
  [PlatformIds.Kuaishou]: 'https://cp.kuaishou.com/',
};

const FollowerCountKeys = [
  'fans_count',
  'fansCount',
  'fans',
  'fan_count',
  'fanCount',
  'followers',
  'followers_count',
  'followersCount',
];

module.exports = {
  AccountStatus,
  AuthMode,
  AuthTaskStatus,
  CreatorHomeUrls,
  DefaultPlatforms,
  FollowerCountKeys,
  LocalLoginUrls,
  PlatformIds,
  RelayPlatformIds,
};
