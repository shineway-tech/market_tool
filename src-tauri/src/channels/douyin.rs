use super::{ChannelPlatform, DomainRule, HomepageKind};

const COOKIE_DOMAINS: &[DomainRule] = &[DomainRule {
    host: "douyin.com",
    include_subdomains: true,
}];

const COOKIE_URLS: &[&str] = &[
    "https://www.douyin.com/",
    "https://douyin.com/",
    "https://creator.douyin.com/",
    "https://passport.douyin.com/",
    "https://sso.douyin.com/",
];

const LOGIN_COOKIE_NAMES: &[&str] = &[
    "sessionid",
    "sessionid_ss",
    "sid_guard",
    "sid_tt",
    "uid_tt",
    "uid_tt_ss",
    "sso_uid_tt",
    "sso_uid_tt_ss",
    "passport_auth_status",
    "passport_auth_status_ss",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "douyin",
    name: "抖音",
    slug: "DY",
    color: "#111111",
    description: "添加并管理多个抖音账号。",
    supports_builtin_oauth: true,
    creator_home_url: "https://creator.douyin.com/creator-micro/home?enter_from=dou_web",
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: ".douyin.com",
    cookie_domains: COOKIE_DOMAINS,
    web_domains: COOKIE_DOMAINS,
    login_cookie_names: LOGIN_COOKIE_NAMES,
    homepage_kind: HomepageKind::Creator,
    plugin_auth: true,
    materialize_avatar: false,
    avatar_referer: None,
    avatar_origin: None,
};
