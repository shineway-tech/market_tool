use super::{ChannelPlatform, DomainRule, HomepageKind};

const COOKIE_DOMAINS: &[DomainRule] = &[
    DomainRule {
        host: "kuaishou.com",
        include_subdomains: true,
    },
    DomainRule {
        host: "kwai.com",
        include_subdomains: true,
    },
];

const COOKIE_URLS: &[&str] = &[
    "https://www.kuaishou.com/",
    "https://kuaishou.com/",
    "https://cp.kuaishou.com/",
    "https://id.kuaishou.com/",
    "https://passport.kuaishou.com/",
];

pub(super) const LOGIN_URL: &str = "https://passport.kuaishou.com/pc/account/login/?sid=kuaishou.web.cp.api&indexPage=login-qrcode&callback=https%3A%2F%2Fcp.kuaishou.com%2Frest%2Finfra%2Fsts%3FfollowUrl%3Dhttps%253A%252F%252Fcp.kuaishou.com%252Fprofile%26setRootDomain%3Dtrue";

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "kuaishou",
    name: "快手",
    slug: "KS",
    color: "#ff4906",
    description: "添加并管理多个快手账号。",
    supports_builtin_oauth: true,
    creator_home_url: "https://cp.kuaishou.com/profile",
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: ".kuaishou.com",
    cookie_domains: COOKIE_DOMAINS,
    web_domains: COOKIE_DOMAINS,
    login_cookie_names: &[],
    homepage_kind: HomepageKind::KuaishouProfileOrSearch,
    plugin_auth: true,
    materialize_avatar: true,
    avatar_referer: Some("https://www.kuaishou.com/"),
    avatar_origin: Some("https://www.kuaishou.com"),
};
