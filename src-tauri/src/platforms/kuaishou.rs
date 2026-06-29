use super::*;

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
pub(super) const HOME_INFO_API: &str = "https://cp.kuaishou.com/rest/cp/creator/pc/home/infoV2";
const CREATOR_HOME_URL: &str = "https://cp.kuaishou.com/profile";
const HOME_INFO_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://cp.kuaishou.com"),
    ("Referer", CREATOR_HOME_URL),
];

const LOGIN_COOKIE_NAMES: &[&str] = &[
    "kuaishou.web.cp.api_st",
    "kuaishou.web.cp.api_ph",
    "passToken",
    "userId",
    "bUserId",
];
const LOGIN_REQUIRED_COOKIE_NAME: &str = "kuaishou.web.cp.api_st";
const RESPONSE_CODE_KEYS: &[&str] = &["result", "code", "errCode", "errcode"];
const UID_KEYS: &[&str] = &["userKwaiId", "kwaiId", "userId", "id", "uid"];
const NICKNAME_KEYS: &[&str] = &["userName", "nickname", "name", "displayName"];
const AVATAR_KEYS: &[&str] = &[
    "userAvatar",
    "avatar",
    "avatarUrl",
    "avatar_url",
    "headUrl",
    "head_url",
    "headurl",
];
const FOLLOWER_COUNT_KEYS: &[&str] = &["fansCnt", "fansNum", "fansCount", "fans", "followers"];
const LIKE_COUNT_KEYS: &[&str] = &["likeCnt", "likeCount", "likes"];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "kuaishou",
    name: "快手",
    slug: "KS",
    color: "#ff4906",
    description: "添加并管理多个快手账号。",
    creator_home_url: CREATOR_HOME_URL,
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: ".kuaishou.com",
    cookie_domains: COOKIE_DOMAINS,
    login_cookie_names: &[],
    homepage_kind: HomepageKind::KuaishouProfileOrSearch,
    plugin_auth: true,
    materialize_avatar: true,
    avatar_referer: Some("https://www.kuaishou.com/"),
    avatar_origin: Some("https://www.kuaishou.com"),
};

pub(crate) async fn collect_kuaishou_plugin_account_from_browser_context(
    value: Value,
    login_cookie: String,
) -> Result<PluginAccountInfo, PluginAuthError> {
    parse_kuaishou_creator_account(value, login_cookie)
        .await
        .map_err(PluginAuthError::NotLoggedIn)
}

pub(crate) fn has_kuaishou_creator_login_cookie_header(cookie_header: &str) -> bool {
    cookie_header.split(';').any(|pair| {
        let Some((name, value)) = pair.trim().split_once('=') else {
            return false;
        };
        let name = name.trim();
        !value.trim().is_empty() && LOGIN_COOKIE_NAMES.iter().any(|item| item == &name)
    }) && cookie_header
        .split(';')
        .any(|pair| pair.trim().starts_with(&format!("{LOGIN_REQUIRED_COOKIE_NAME}=")))
}

pub(super) async fn fetch_kuaishou_creator_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let value = request_plugin_json(
        "POST",
        HOME_INFO_API,
        cookie_header,
        HOME_INFO_HEADERS,
    )
    .await
    .map_err(|error| format!("快手创作者中心账号接口不可用: {error}"))?;
    parse_kuaishou_creator_account(value, login_cookie).await
}

async fn parse_kuaishou_creator_account(
    value: Value,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let payload = value.get("data").filter(|data| !data.is_null()).unwrap_or(&value);
    let result = first_i64(&value, RESPONSE_CODE_KEYS).unwrap_or(1);
    let uid = first_string_deep(payload, UID_KEYS)
        .or_else(|| {
            first_count(payload, UID_KEYS)
                .filter(|value| *value > 0)
                .map(|value| value.to_string())
        })
        .unwrap_or_default();
    let nickname = first_string_deep(payload, NICKNAME_KEYS)
        .unwrap_or_else(|| platform_name("kuaishou").to_string());
    let has_profile = !uid.trim().is_empty() || nickname != platform_name("kuaishou");
    let top_keys = value
        .as_object()
        .map(|object| object.keys().take(8).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    eprintln!(
        "[plugin-auth:kuaishou] result={result} has_profile={has_profile} keys={top_keys:?}"
    );
    if result == 500002 || !has_profile {
        if let Some(account) = kuaishou_account_from_login_cookie(&login_cookie) {
            eprintln!("[plugin-auth:kuaishou] using login-cookie fallback account={}", account.uid);
            return Ok(account);
        }
        return Err("请先在打开的快手创作者中心完成登录。".to_string());
    }
    let avatar = first_profile_image(payload, AVATAR_KEYS)
    .unwrap_or_default();
    let avatar = materialize_account_avatar("kuaishou", avatar).await;
    let account = if uid.trim().is_empty() {
        nickname.clone()
    } else {
        uid.clone()
    };
    Ok(PluginAccountInfo {
        uid: account.clone(),
        account,
        nickname,
        avatar,
        fans_count: first_count(payload, FOLLOWER_COUNT_KEYS),
        like_count: first_count(payload, LIKE_COUNT_KEYS),
        login_cookie,
    })
}

fn kuaishou_account_from_login_cookie(login_cookie: &str) -> Option<PluginAccountInfo> {
    if !login_cookie_has_required_cookie(login_cookie) {
        return None;
    }
    let uid = login_cookie_value(login_cookie, "userId")
        .or_else(|| login_cookie_value(login_cookie, "bUserId"))?;
    let uid = uid.trim().to_string();
    if uid.is_empty() {
        return None;
    }
    let suffix = uid.chars().rev().take(4).collect::<String>().chars().rev().collect::<String>();
    let nickname = if suffix.is_empty() {
        "快手账号".to_string()
    } else {
        format!("快手账号 {suffix}")
    };
    Some(PluginAccountInfo {
        uid: uid.clone(),
        account: uid,
        nickname,
        avatar: String::new(),
        fans_count: None,
        like_count: None,
        login_cookie: login_cookie.to_string(),
    })
}

fn login_cookie_has_required_cookie(login_cookie: &str) -> bool {
    login_cookie_value(login_cookie, LOGIN_REQUIRED_COOKIE_NAME)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn login_cookie_value(login_cookie: &str, expected_name: &str) -> Option<String> {
    let trimmed = login_cookie.trim();
    if trimmed.starts_with('[') {
        if let Ok(Value::Array(cookies)) = serde_json::from_str::<Value>(trimmed) {
            return cookies.iter().find_map(|cookie| {
                let name = cookie.get("name").and_then(Value::as_str)?;
                if !name.eq_ignore_ascii_case(expected_name) {
                    return None;
                }
                cookie
                    .get("value")
                    .and_then(Value::as_str)
                    .map(|value| value.to_string())
            });
        }
    }
    trimmed.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        if name.trim().eq_ignore_ascii_case(expected_name) {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}
