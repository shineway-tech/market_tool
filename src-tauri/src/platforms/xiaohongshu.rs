use super::*;

const COOKIE_DOMAINS: &[DomainRule] = &[DomainRule {
    host: "xiaohongshu.com",
    include_subdomains: true,
}];

const COOKIE_URLS: &[&str] = &[
    "https://www.xiaohongshu.com/",
    "https://creator.xiaohongshu.com/",
    "https://edith.xiaohongshu.com/",
];

const CREATOR_HOME_URL: &str = "https://creator.xiaohongshu.com/new/home";
const CREATOR_USER_INFO_API: &str = "https://creator.xiaohongshu.com/api/galaxy/user/info";
const CREATOR_PERSONAL_INFO_API: &str =
    "https://creator.xiaohongshu.com/api/galaxy/creator/home/personal_info";
const EDITH_USER_ME_API: &str = "https://edith.xiaohongshu.com/api/sns/web/v2/user/me";

const CREATOR_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://creator.xiaohongshu.com"),
    ("Referer", CREATOR_HOME_URL),
];
const EDITH_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://www.xiaohongshu.com"),
    ("Referer", "https://www.xiaohongshu.com/"),
];

const USER_UID_KEYS: &[&str] = &["user_id", "red_id", "userId", "id", "redId"];
const EDITH_UID_KEYS: &[&str] = &["user_id", "red_id", "userId", "id"];
const CREATOR_UID_KEYS: &[&str] = &[
    "user_id",
    "red_id",
    "userId",
    "id",
    "creator_id",
    "creatorId",
    "author_id",
    "authorId",
];
const USER_NICKNAME_KEYS: &[&str] = &[
    "userName",
    "user_name",
    "nickname",
    "nickName",
    "name",
    "red_id",
    "redId",
];
const CREATOR_NICKNAME_KEYS: &[&str] = &[
    "name",
    "nickname",
    "nickName",
    "user_name",
    "userName",
    "creator_name",
    "creatorName",
];
const EDITH_NICKNAME_KEYS: &[&str] = &["name", "nickname", "nickName", "user_name", "userName", "red_id"];
const USER_AVATAR_KEYS: &[&str] = &[
    "userAvatar",
    "user_avatar",
    "avatar",
    "avatar_url",
    "avatarUrl",
    "head_img",
    "headImg",
    "headImgUrl",
];
const CREATOR_AVATAR_KEYS: &[&str] = &[
    "avatar",
    "avatar_url",
    "avatarUrl",
    "head_img",
    "headImg",
    "headImgUrl",
    "image",
    "image_url",
    "imageUrl",
    "profile_image_url",
    "profilePicture",
];
const EDITH_AVATAR_KEYS: &[&str] = CREATOR_AVATAR_KEYS;
const FOLLOWER_COUNT_KEYS: &[&str] = &[
    "fans_count",
    "fansCount",
    "fans",
    "fan_count",
    "fanCount",
    "fans_num",
    "fansNum",
    "followers",
    "followers_count",
    "followersCount",
    "fans_count_show",
    "fansCountShow",
    "fansNumShow",
];
const LIKE_COUNT_KEYS: &[&str] = &[
    "liked_count",
    "likedCount",
    "like_count",
    "likeCount",
    "likes",
    "liked",
    "faved_count",
    "favedCount",
    "like_collect_count",
    "likeCollectCount",
    "liked_collect_count",
    "likedCollectCount",
    "liked_num_show",
    "likedNumShow",
    "total_liked",
    "totalLiked",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "xiaohongshu",
    name: "小红书",
    slug: "XHS",
    color: "#ff2442",
    description: "添加并管理多个小红书账号。",
    creator_home_url: CREATOR_HOME_URL,
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

pub(super) async fn refresh_xhs_account_profile(
    saved_login_cookie: Option<&str>,
) -> Result<Option<PluginAccountInfo>, String> {
    if let Some(login_cookie) = saved_login_cookie {
        let cookie_header = login_cookie_to_header(login_cookie);
        if !cookie_header.trim().is_empty() {
            match fetch_xhs_plugin_account_from_cookie(
                &cookie_header,
                login_cookie.to_string(),
                Some("creator"),
            )
            .await
            {
                Ok(profile) => return Ok(Some(profile)),
                Err(error) => eprintln!("[refresh:xhs] saved cookie refresh failed: {}", plugin_error_message(&error)),
            }
        }
    }

    Err("小红书登录已失效，请重新登录后再打开创作中心。".to_string())
}

pub(super) fn xhs_profile_matches_account(profile: &PluginAccountInfo, account: &ChannelAccount) -> bool {
    let profile_values = [&profile.uid, &profile.account, &profile.nickname]
        .into_iter()
        .map(|value| normalize_match_key(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    xhs_account_match_values(account)
        .into_iter()
        .any(|value| profile_values.iter().any(|profile_value| profile_value == &value))
}

fn xhs_account_match_values(account: &ChannelAccount) -> Vec<String> {
    let mut values = vec![
        account.uid.clone(),
        account.nickname.clone(),
        account.id.clone(),
    ];
    values.extend(values.clone().into_iter().map(|value| {
        value
            .strip_prefix("xhs_")
            .unwrap_or(&value)
            .strip_suffix("_web")
            .unwrap_or_else(|| value.strip_prefix("xhs_").unwrap_or(&value))
            .to_string()
    }));
    values
        .into_iter()
        .map(|value| normalize_match_key(&value))
        .filter(|value| !value.is_empty())
        .collect()
}

pub(super) async fn fetch_xhs_plugin_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
    login_target: Option<&str>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let user_result = request_plugin_json(
        "GET",
        CREATOR_USER_INFO_API,
        cookie_header,
        CREATOR_HEADERS,
    )
    .await;
    if let Err(error) = &user_result {
        eprintln!("[plugin-auth:xhs] creator user request failed: {error}");
    }
    let edith_result = request_plugin_json(
        "GET",
        EDITH_USER_ME_API,
        cookie_header,
        EDITH_HEADERS,
    )
    .await;
    if let Err(error) = &edith_result {
        eprintln!("[plugin-auth:xhs] edith profile request failed: {error}");
    }
    let creator_result = request_plugin_json(
        "GET",
        CREATOR_PERSONAL_INFO_API,
        cookie_header,
        CREATOR_HEADERS,
    )
    .await;
    if let Err(error) = &creator_result {
        eprintln!("[plugin-auth:xhs] creator profile request failed: {error}");
    }
    let user = user_result.ok();
    let edith = edith_result.ok();
    let creator = creator_result.ok();
    let user_data = user.as_ref().and_then(xhs_response_payload);
    let edith_data = edith.as_ref().and_then(xhs_response_payload);
    let creator_data = creator.as_ref().and_then(xhs_response_payload);
    let user_uid = user_data.and_then(|data| first_string_deep(data, USER_UID_KEYS));
    let edith_uid = edith_data.and_then(|data| first_string_deep(data, EDITH_UID_KEYS));
    let creator_uid = creator_data.and_then(|data| first_string_deep(data, CREATOR_UID_KEYS));
    let user_nickname = user_data.and_then(|data| first_string_deep(data, USER_NICKNAME_KEYS));
    let creator_nickname = creator_data.and_then(|data| first_string_deep(data, CREATOR_NICKNAME_KEYS));
    let user_avatar = user_data.and_then(|data| first_profile_image(data, USER_AVATAR_KEYS));
    let creator_avatar = creator_data.and_then(|data| first_profile_image(data, CREATOR_AVATAR_KEYS));
    let creator_fans_count = creator_data.and_then(|data| first_count(data, FOLLOWER_COUNT_KEYS));
    let creator_like_count = creator_data.and_then(|data| first_count(data, LIKE_COUNT_KEYS));
    let user_ok = user
        .as_ref()
        .map(|value| response_success(value) && user_uid.is_some())
        .unwrap_or(false);
    let creator_has_profile = creator_uid.is_some()
        || creator_nickname.is_some()
        || creator_avatar.is_some()
        || creator_fans_count.is_some()
        || creator_like_count.is_some();
    let creator_ok = creator
        .as_ref()
        .map(|value| response_success(value) && creator_has_profile)
        .unwrap_or(false);
    eprintln!("[plugin-auth:xhs] user_ok={user_ok} creator_ok={creator_ok}");
    if !user_ok || !creator_ok {
        return Err(PluginAuthError::NotLoggedIn(match login_target {
            Some("home") => "请先在打开的小红书主页完成登录。".to_string(),
            _ => "请先在打开的小红书创作中心完成登录。".to_string(),
        }));
    }

    let uid = creator_uid.or(user_uid).or(edith_uid).unwrap_or_default();
    let nickname = creator_nickname
        .or(user_nickname)
        .or_else(|| {
            edith_data.and_then(|data| first_string_deep(data, EDITH_NICKNAME_KEYS))
        })
        .unwrap_or_default();
    let avatar = creator_avatar
        .or(user_avatar)
        .or_else(|| {
            edith_data.and_then(|data| first_profile_image(data, EDITH_AVATAR_KEYS))
        })
        .map(normalize_image_url)
        .unwrap_or_default();
    let avatar = materialize_account_avatar("xiaohongshu", avatar).await;
    let account = if uid.trim().is_empty() {
        nickname.clone()
    } else {
        uid.clone()
    };
    if account.trim().is_empty() || account == platform_name("xiaohongshu") {
        return Err(PluginAuthError::NotLoggedIn(
            "小红书已登录，但没有读取到账号 ID，请进入创作者中心后再检查状态。".to_string(),
        ));
    }

    Ok(PluginAccountInfo {
        uid: account.clone(),
        account,
        nickname,
        avatar,
        fans_count: creator_fans_count
            .or_else(|| first_count_from_values(&[user_data, edith_data], FOLLOWER_COUNT_KEYS)),
        like_count: creator_like_count
            .or_else(|| first_count_from_values(&[user_data, edith_data], LIKE_COUNT_KEYS)),
        login_cookie,
    })
}

fn xhs_response_payload(value: &Value) -> Option<&Value> {
    value
        .get("data")
        .filter(|data| !data.is_null())
        .or(Some(value))
}
