use super::*;

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

const CREATOR_HOME_URL: &str = "https://creator.douyin.com/creator-micro/home?enter_from=dou_web";
const PC_USER_INFO_API: &str = "https://creator.douyin.com/aweme/v1/creator/pc/user/info/";
const USER_INFO_API: &str = "https://creator.douyin.com/aweme/v1/creator/user/info/";
const CREATOR_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://creator.douyin.com"),
    ("Referer", CREATOR_HOME_URL),
];

const PC_USER_ID_KEYS: &[&str] = &["uid", "user_id", "userId", "sec_uid", "secUid"];
const CREATOR_USER_ID_KEYS: &[&str] = &[
    "uid",
    "user_id",
    "userId",
    "sec_uid",
    "secUid",
    "douyin_unique_id",
    "unique_id",
    "uniqueId",
];
const NICKNAME_KEYS: &[&str] = &[
    "nick_name",
    "nickName",
    "nickname",
    "name",
    "display_name",
    "displayName",
];
const AVATAR_KEYS: &[&str] = &[
    "avatar_url",
    "avatarUrl",
    "avatar",
    "avatar_thumb",
    "avatarThumb",
    "head_img",
    "headImg",
];
const FOLLOWER_COUNT_KEYS: &[&str] = &[
    "fans_count",
    "fansCount",
    "fans",
    "fan_count",
    "fanCount",
    "follower_count",
    "followerCount",
    "followers",
    "followers_count",
    "followersCount",
    "displayFans",
    "fansDisplay",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "douyin",
    name: "抖音",
    slug: "DY",
    color: "#111111",
    description: "添加并管理多个抖音账号。",
    creator_home_url: CREATOR_HOME_URL,
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: ".douyin.com",
    cookie_domains: COOKIE_DOMAINS,
    login_cookie_names: LOGIN_COOKIE_NAMES,
    homepage_kind: HomepageKind::Creator,
    plugin_auth: true,
    materialize_avatar: false,
    avatar_referer: None,
    avatar_origin: None,
};

pub(super) async fn fetch_douyin_creator_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let pc_user = request_plugin_json(
        "GET",
        PC_USER_INFO_API,
        cookie_header,
        CREATOR_HEADERS,
    )
    .await
    .map_err(|error| format!("抖音创作者中心账号接口不可用: {error}"))?;
    if !douyin_response_success(&pc_user) {
        return Err("抖音网页登录态已失效，请重新登录后再打开创作中心。".to_string());
    }

    let user = request_plugin_json(
        "GET",
        USER_INFO_API,
        cookie_header,
        CREATOR_HEADERS,
    )
    .await
    .map_err(|error| format!("抖音创作者中心资料接口不可用: {error}"))?;
    if !douyin_response_success(&user) {
        return Err("抖音创作者中心资料读取失败，请重新登录后再同步。".to_string());
    }

    let verify_info = user
        .get("douyin_user_verify_info")
        .or_else(|| user.get("user_profile"));
    let uid = first_string_deep(&pc_user, PC_USER_ID_KEYS)
    .or_else(|| {
        verify_info.and_then(|value| first_string_deep(value, CREATOR_USER_ID_KEYS))
    })
    .or_else(|| {
        first_string_deep(&user, CREATOR_USER_ID_KEYS)
    })
    .unwrap_or_else(|| stable_label_fragment(cookie_header));
    let nickname = verify_info
        .and_then(|value| first_string_deep(value, NICKNAME_KEYS))
        .or_else(|| {
            first_string_deep(&user, NICKNAME_KEYS)
        })
        .unwrap_or_else(|| platform_name("douyin").to_string());
    let avatar = verify_info
        .and_then(|value| first_profile_image(value, AVATAR_KEYS))
        .or_else(|| {
            first_profile_image(&user, AVATAR_KEYS)
        })
        .unwrap_or_default();
    let fans_count = verify_info
        .and_then(|value| first_count(value, FOLLOWER_COUNT_KEYS))
        .or_else(|| first_count(&user, FOLLOWER_COUNT_KEYS));

    Ok(PluginAccountInfo {
        uid: uid.clone(),
        account: uid,
        nickname,
        avatar,
        fans_count,
        like_count: None,
        login_cookie,
    })
}

fn douyin_response_success(value: &Value) -> bool {
    first_i64(value, &["status_code", "code", "errCode", "errcode"])
        .map(|code| code == 0)
        .unwrap_or(true)
}

pub(super) fn has_douyin_login_cookie(login_cookie: &str) -> bool {
    let Some(platform) = crate::platforms::platform("douyin") else {
        return false;
    };
    login_cookie_to_header(login_cookie).split(';').any(|pair| {
        let Some((name, value)) = pair.trim().split_once('=') else {
            return false;
        };
        !value.trim().is_empty() && platform.is_login_cookie_name(name)
    })
}
