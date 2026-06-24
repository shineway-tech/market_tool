use super::{ChannelPlatform, DomainRule, HomepageKind};

const COOKIE_DOMAINS: &[DomainRule] = &[DomainRule {
    host: "bilibili.com",
    include_subdomains: true,
}];

const COOKIE_URLS: &[&str] = &[
    "https://www.bilibili.com/",
    "https://bilibili.com/",
    "https://passport.bilibili.com/",
    "https://member.bilibili.com/",
    "https://space.bilibili.com/",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "bilibili",
    name: "哔哩哔哩",
    slug: "BILI",
    color: "#00a1d6",
    description: "添加并管理多个 B 站账号。",
    supports_builtin_oauth: true,
    creator_home_url: "https://member.bilibili.com/platform/home",
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: ".bilibili.com",
    cookie_domains: COOKIE_DOMAINS,
    web_domains: COOKIE_DOMAINS,
    login_cookie_names: &[],
    homepage_kind: HomepageKind::BilibiliSpaceOrSearch,
    plugin_auth: true,
    materialize_avatar: true,
    avatar_referer: Some("https://www.bilibili.com/"),
    avatar_origin: Some("https://www.bilibili.com"),
};
