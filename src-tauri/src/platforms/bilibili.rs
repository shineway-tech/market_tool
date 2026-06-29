use super::*;

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

const CREATOR_HOME_URL: &str = "https://member.bilibili.com/platform/home";
const NAV_API: &str = "https://api.bilibili.com/x/web-interface/nav";
const RELATION_STAT_API_PREFIX: &str = "https://api.bilibili.com/x/relation/stat?vmid=";
const API_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://www.bilibili.com"),
    ("Referer", "https://www.bilibili.com/"),
];

const MID_KEYS: &[&str] = &["mid", "uid", "id"];
const NICKNAME_KEYS: &[&str] = &["uname", "nickname", "name"];
const AVATAR_KEYS: &[&str] = &["face", "avatar", "avatarUrl", "avatar_url"];
const FOLLOWER_COUNT_KEYS: &[&str] = &[
    "follower",
    "fans_count",
    "fansCount",
    "fans",
    "followers",
    "followers_count",
    "followersCount",
];
const LIKE_COUNT_KEYS: &[&str] = &[
    "likes",
    "like_count",
    "likeCount",
    "liked_count",
    "likedCount",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "bilibili",
    name: "哔哩哔哩",
    slug: "BILI",
    color: "#00a1d6",
    description: "添加并管理多个 B 站账号。",
    creator_home_url: CREATOR_HOME_URL,
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: ".bilibili.com",
    cookie_domains: COOKIE_DOMAINS,
    login_cookie_names: &[],
    homepage_kind: HomepageKind::BilibiliSpaceOrSearch,
    plugin_auth: true,
    materialize_avatar: true,
    avatar_referer: Some("https://www.bilibili.com/"),
    avatar_origin: Some("https://www.bilibili.com"),
};

fn first_bilibili_mid(data: &Value) -> Option<String> {
    first_i64(data, MID_KEYS)
        .filter(|value| *value > 0)
        .map(|value| value.to_string())
        .or_else(|| {
            first_string(data, MID_KEYS)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

pub(super) async fn probe_bilibili_creator_session(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let value = request_plugin_json(
        "GET",
        NAV_API,
        cookie_header,
        API_HEADERS,
    )
    .await
    .map_err(|error| format!("B 站登录已失效，请重新登录后再打开创作中心。{error}"))?;
    let data = value.get("data");
    let is_login = data
        .and_then(|data| data.get("isLogin"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if first_i64(&value, &["code"]).unwrap_or(-1) != 0 || !is_login {
        return Err("B 站登录已失效，请重新登录后再打开创作中心。".to_string());
    }

    let uid = data
        .and_then(first_bilibili_mid)
        .unwrap_or_default();
    let nickname = data
        .and_then(|data| first_string(data, NICKNAME_KEYS))
        .unwrap_or_else(|| platform_name("bilibili").to_string());
    let avatar = data
        .and_then(|data| first_profile_image(data, AVATAR_KEYS))
        .unwrap_or_default();
    let avatar = materialize_account_avatar("bilibili", avatar).await;
    let account = if uid.trim().is_empty() {
        nickname.clone()
    } else {
        uid.clone()
    };
    let mut fans_count = data.and_then(|data| first_count(data, FOLLOWER_COUNT_KEYS));
    if fans_count.is_none() {
        fans_count = fetch_bilibili_fans_count(cookie_header, &uid).await;
    }
    Ok(PluginAccountInfo {
        uid: account.clone(),
        account,
        nickname,
        avatar,
        fans_count,
        like_count: data.and_then(|data| first_count(data, LIKE_COUNT_KEYS)),
        login_cookie,
    })
}

async fn fetch_bilibili_fans_count(cookie_header: &str, uid: &str) -> Option<u64> {
    let uid = uid.trim();
    if uid.is_empty() || !uid.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let value = request_plugin_json(
        "GET",
        &format!("{RELATION_STAT_API_PREFIX}{uid}"),
        cookie_header,
        API_HEADERS,
    )
    .await
    .ok()?;
    if first_i64(&value, &["code"]).unwrap_or(-1) != 0 {
        return None;
    }
    value
        .get("data")
        .and_then(|data| first_count(data, FOLLOWER_COUNT_KEYS))
}
