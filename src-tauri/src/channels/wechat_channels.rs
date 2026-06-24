use super::{ChannelPlatform, DomainRule, HomepageKind};

const COOKIE_DOMAINS: &[DomainRule] = &[DomainRule {
    host: "channels.weixin.qq.com",
    include_subdomains: false,
}];

const COOKIE_URLS: &[&str] = &[
    "https://channels.weixin.qq.com/",
    "https://channels.weixin.qq.com/platform",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "wechat-channels",
    name: "视频号",
    slug: "WX",
    color: "#ff9f2e",
    description: "添加并管理多个微信视频号账号。",
    supports_builtin_oauth: true,
    creator_home_url: "https://channels.weixin.qq.com/platform",
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: "channels.weixin.qq.com",
    cookie_domains: COOKIE_DOMAINS,
    web_domains: COOKIE_DOMAINS,
    login_cookie_names: &[],
    homepage_kind: HomepageKind::Creator,
    plugin_auth: true,
    materialize_avatar: false,
    avatar_referer: None,
    avatar_origin: None,
};
