use super::{ChannelPlatform, DomainRule, HomepageKind};

const COOKIE_DOMAINS: &[DomainRule] = &[DomainRule {
    host: "xiaohongshu.com",
    include_subdomains: true,
}];

const COOKIE_URLS: &[&str] = &[
    "https://www.xiaohongshu.com/",
    "https://creator.xiaohongshu.com/",
    "https://edith.xiaohongshu.com/",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "xiaohongshu",
    name: "小红书",
    slug: "XHS",
    color: "#ff2442",
    description: "添加并管理多个小红书账号。",
    supports_builtin_oauth: false,
    creator_home_url: "https://creator.xiaohongshu.com/new/home",
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: ".xiaohongshu.com",
    cookie_domains: COOKIE_DOMAINS,
    login_cookie_names: &[],
    homepage_kind: HomepageKind::Creator,
    plugin_auth: true,
    materialize_avatar: true,
    avatar_referer: Some("https://creator.xiaohongshu.com/"),
    avatar_origin: Some("https://creator.xiaohongshu.com"),
};
