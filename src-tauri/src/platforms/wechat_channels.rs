use super::*;

const COOKIE_DOMAINS: &[DomainRule] = &[DomainRule {
    host: "channels.weixin.qq.com",
    include_subdomains: false,
}];

const COOKIE_URLS: &[&str] = &[
    "https://channels.weixin.qq.com/",
    "https://channels.weixin.qq.com/platform",
];

const CREATOR_HOME_URL: &str = "https://channels.weixin.qq.com/platform";
const AUTH_DATA_API: &str = "https://channels.weixin.qq.com/cgi-bin/mmfinderassistant-bin/auth/auth_data";
const AUTH_DATA_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://channels.weixin.qq.com"),
    ("Referer", CREATOR_HOME_URL),
    ("Content-Type", "application/json"),
];

const UID_KEYS: &[&str] = &[
    "uniqId",
    "uniq_id",
    "finderUsername",
    "finderUserName",
    "finder_user_name",
    "finderUserId",
    "finder_user_id",
    "username",
    "userName",
    "wxUsername",
    "wxUserName",
    "openId",
    "open_id",
    "uin",
    "id",
];
const NICKNAME_KEYS: &[&str] = &["nickname", "nickName", "name", "displayName", "finderNickname"];
const AVATAR_KEYS: &[&str] = &[
    "headImgUrl",
    "head_img_url",
    "avatar",
    "avatarUrl",
    "avatar_url",
    "headImg",
    "image",
    "imageUrl",
];
const FOLLOWER_COUNT_KEYS: &[&str] = &[
    "fans_count",
    "fansCount",
    "fans",
    "fan_count",
    "fanCount",
    "followers",
    "followers_count",
    "followersCount",
    "follower_count",
    "followerCount",
];
const LIKE_COUNT_KEYS: &[&str] = &[
    "like_count",
    "likeCount",
    "liked_count",
    "likedCount",
    "likes",
    "liked",
    "total_liked",
    "totalLiked",
];

pub(super) static SPEC: ChannelPlatform = ChannelPlatform {
    id: "wechat-channels",
    name: "视频号",
    slug: "WX",
    color: "#ff9f2e",
    description: "添加并管理多个微信视频号账号。",
    creator_home_url: CREATOR_HOME_URL,
    cookie_urls: COOKIE_URLS,
    default_cookie_domain: "channels.weixin.qq.com",
    cookie_domains: COOKIE_DOMAINS,
    login_cookie_names: &[],
    homepage_kind: HomepageKind::Creator,
    plugin_auth: true,
    materialize_avatar: false,
    avatar_referer: None,
    avatar_origin: None,
};

pub(super) async fn fetch_wx_channels_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let value = request_plugin_json(
        "POST",
        AUTH_DATA_API,
        &cookie_header,
        AUTH_DATA_HEADERS,
    )
    .await
    .map_err(|error| {
        if error.contains("401") || error.contains("403") {
            PluginAuthError::NotLoggedIn("视频号登录已过期，请重新登录。".to_string())
        } else {
            PluginAuthError::Failed(error)
        }
    })?;

    let err_code = first_i64(&value, &["errCode", "errcode", "code"]).unwrap_or(0);
    if err_code == 300333 || err_code == 300334 {
        return Err(PluginAuthError::NotLoggedIn(
            "视频号登录已过期，请重新登录。".to_string(),
        ));
    }
    let finder_user = value
        .get("data")
        .and_then(|data| data.get("finderUser"))
        .ok_or_else(|| {
            PluginAuthError::NotLoggedIn(
                first_string(&value, &["errMsg", "errmsg", "message"])
                    .unwrap_or_else(|| "请先在打开的视频号窗口完成登录。".to_string()),
            )
        })?;
    if err_code != 0 {
        return Err(PluginAuthError::NotLoggedIn(
            first_string(&value, &["errMsg", "errmsg", "message"])
                .unwrap_or_else(|| "视频号登录没有完成。".to_string()),
        ));
    }

    let uid = first_string_deep(finder_user, UID_KEYS)
    .unwrap_or_default();
    let nickname = first_string_deep(finder_user, NICKNAME_KEYS)
        .unwrap_or_else(|| platform_name("wechat-channels").to_string());
    let account = if uid.trim().is_empty() {
        nickname.clone()
    } else {
        uid.clone()
    };
    if account.trim().is_empty() || account == platform_name("wechat-channels") {
        return Err(PluginAuthError::NotLoggedIn(
            "视频号已登录，但没有读取到账号 ID，请进入视频号后台后再检查状态。".to_string(),
        ));
    }

    Ok(PluginAccountInfo {
        uid: account.clone(),
        account,
        nickname,
        avatar: first_profile_image(finder_user, AVATAR_KEYS)
        .map(normalize_image_url)
        .unwrap_or_default(),
        fans_count: first_count(finder_user, FOLLOWER_COUNT_KEYS),
        like_count: first_count(finder_user, LIKE_COUNT_KEYS),
        login_cookie,
    })
}
