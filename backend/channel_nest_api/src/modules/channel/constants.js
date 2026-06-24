const PlatformIds = {
  Xiaohongshu: 'xiaohongshu',
  WechatChannels: 'wechat-channels',
  Douyin: 'douyin',
  Bilibili: 'bilibili',
  Kuaishou: 'kuaishou',
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
  Creator: 'creator',
  OAuth: 'oauth',
};

const DefaultPlatforms = [
  {
    platform_id: PlatformIds.Xiaohongshu,
    name: '小红书',
    slug: 'XHS',
    color: '#ff2442',
    description: '添加并管理多个小红书账号。',
    supports_builtin_oauth: false,
    auth_mode: AuthMode.Creator,
    sort_no: 10,
  },
  {
    platform_id: PlatformIds.WechatChannels,
    name: '视频号',
    slug: 'WX',
    color: '#ff9f2e',
    description: '添加并管理多个微信视频号账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Creator,
    sort_no: 20,
  },
  {
    platform_id: PlatformIds.Douyin,
    name: '抖音',
    slug: 'DY',
    color: '#111111',
    description: '添加并管理多个抖音账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Creator,
    sort_no: 30,
  },
  {
    platform_id: PlatformIds.Bilibili,
    name: '哔哩哔哩',
    slug: 'BILI',
    color: '#00a1d6',
    description: '添加并管理多个 B 站账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Creator,
    sort_no: 40,
  },
  {
    platform_id: PlatformIds.Kuaishou,
    name: '快手',
    slug: 'KS',
    color: '#ff4906',
    description: '添加并管理多个快手账号。',
    supports_builtin_oauth: true,
    auth_mode: AuthMode.Creator,
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

const LikeCountKeys = [
  'liked_count',
  'likedCount',
  'like_count',
  'likeCount',
  'likes',
  'liked',
  'faved_count',
  'favedCount',
  'faved_num',
  'favedNum',
  'liked_num',
  'likedNum',
  'like_num',
  'likeNum',
  'like_collect_count',
  'likeCollectCount',
  'liked_collect_count',
  'likedCollectCount',
  'like_collect_num',
  'likeCollectNum',
  'liked_collect_num',
  'likedCollectNum',
  'like_collect_number',
  'likeCollectNumber',
  'liked_collect_number',
  'likedCollectNumber',
  'like_and_collect',
  'likeAndCollect',
  'like_and_collect_count',
  'likeAndCollectCount',
  'liked_and_collected',
  'likedAndCollected',
  'liked_and_collected_count',
  'likedAndCollectedCount',
  'total_liked',
  'totalLiked',
  'total_like',
  'totalLike',
];

module.exports = {
  AccountStatus,
  AuthMode,
  AuthTaskStatus,
  CreatorHomeUrls,
  DefaultPlatforms,
  FollowerCountKeys,
  LikeCountKeys,
  LocalLoginUrls,
  PlatformIds,
};
