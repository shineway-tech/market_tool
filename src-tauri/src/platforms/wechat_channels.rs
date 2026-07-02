use super::*;
use chrono::{FixedOffset, TimeZone};
use serde_json::json;

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
const STATISTIC_FANS_TREND_API: &str =
    "https://channels.weixin.qq.com/cgi-bin/mmfinderassistant-bin/statistic/fans_trend";
const STATISTIC_POST_TOTAL_API: &str =
    "https://channels.weixin.qq.com/cgi-bin/mmfinderassistant-bin/statistic/new_post_total_data";
const POST_LIST_API: &str =
    "https://channels.weixin.qq.com/micro/content/cgi-bin/mmfinderassistant-bin/post/post_list";
const WX_VIDEO_LIST_PAGE_URL: &str = "https://channels.weixin.qq.com/micro/content/post/list";
const WX_VIDEO_LIST_PAGE_URL_PARAM: &str =
    "https:%2F%2Fchannels.weixin.qq.com%2Fmicro%2Fcontent%2Fpost%2Flist";
const WX_ARTICLE_LIST_PAGE_URL: &str =
    "https://channels.weixin.qq.com/micro/content/post/finderNewLifePostList";
const WX_ARTICLE_LIST_PAGE_URL_PARAM: &str =
    "https:%2F%2Fchannels.weixin.qq.com%2Fmicro%2Fcontent%2Fpost%2FfinderNewLifePostList";
const WX_POST_CARD_PAGE_URL: &str =
    "https://channels.weixin.qq.com/micro/content/iframe/post-card.html";
const WX_POST_CARD_PAGE_URL_PARAM: &str =
    "https:%2F%2Fchannels.weixin.qq.com%2Fmicro%2Fcontent%2Fiframe%2Fpost-card.html";
const AUTH_DATA_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://channels.weixin.qq.com"),
    ("Referer", CREATOR_HOME_URL),
    ("Content-Type", "application/json"),
];
const STATISTIC_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://channels.weixin.qq.com"),
    ("Referer", "https://channels.weixin.qq.com/platform"),
    ("Content-Type", "application/json"),
];

const WX_INTERVAL_DAY: i64 = 3;
const WX_USERPAGE_TYPE_PHOTO: i64 = 10;
const WX_USERPAGE_TYPE_VIDEO: i64 = 11;
const WX_WORKS_TAB_PAGE_SIZE: i64 = 20;
const WX_LATEST_CARD_PAGE_SIZE: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WxWorkKind {
    Video,
    Article,
}

impl WxWorkKind {
    fn from_option(value: Option<&str>) -> Self {
        match value.unwrap_or_default().trim() {
            "article" | "photo" | "image" | "new-life" | "newlife" => WxWorkKind::Article,
            _ => WxWorkKind::Video,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            WxWorkKind::Video => "video",
            WxWorkKind::Article => "article",
        }
    }

    fn userpage_type(self) -> i64 {
        match self {
            WxWorkKind::Video => WX_USERPAGE_TYPE_VIDEO,
            WxWorkKind::Article => WX_USERPAGE_TYPE_PHOTO,
        }
    }

    fn views_label(self) -> &'static str {
        match self {
            WxWorkKind::Video => "播放",
            WxWorkKind::Article => "阅读",
        }
    }

    fn includes_play_metrics(self) -> bool {
        matches!(self, WxWorkKind::Video)
    }
}

#[derive(Debug, Clone, Copy)]
enum WxPostListSource {
    WorksTab,
    LatestCard,
}

#[derive(Debug, Clone, Copy)]
struct WxPostListRequestConfig {
    referer: &'static str,
    page_url_param: &'static str,
    page_size: i64,
    sticky_order: bool,
}

impl WxPostListSource {
    fn request_config(self, work_kind: WxWorkKind) -> WxPostListRequestConfig {
        match self {
            WxPostListSource::LatestCard => WxPostListRequestConfig {
                referer: WX_POST_CARD_PAGE_URL,
                page_url_param: WX_POST_CARD_PAGE_URL_PARAM,
                page_size: WX_LATEST_CARD_PAGE_SIZE,
                sticky_order: false,
            },
            WxPostListSource::WorksTab if work_kind == WxWorkKind::Article => {
                WxPostListRequestConfig {
                    referer: WX_ARTICLE_LIST_PAGE_URL,
                    page_url_param: WX_ARTICLE_LIST_PAGE_URL_PARAM,
                    page_size: WX_WORKS_TAB_PAGE_SIZE,
                    sticky_order: true,
                }
            }
            WxPostListSource::WorksTab => WxPostListRequestConfig {
                referer: WX_VIDEO_LIST_PAGE_URL,
                page_url_param: WX_VIDEO_LIST_PAGE_URL_PARAM,
                page_size: WX_WORKS_TAB_PAGE_SIZE,
                sticky_order: true,
            },
        }
    }
}

impl WxPostListRequestConfig {
    fn url(self) -> String {
        format!("{POST_LIST_API}?_pageUrl={}", self.page_url_param)
    }

    fn body(self, current_page: i64, userpage_type: i64) -> Value {
        json!({
            "pageSize": self.page_size,
            "currentPage": current_page,
            "userpageType": userpage_type,
            "stickyOrder": self.sticky_order,
            "timestamp": Utc::now().timestamp_millis().to_string(),
            "scene": 7,
            "reqScene": 7,
        })
    }
}

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
const FOLLOWING_COUNT_KEYS: &[&str] = &[
    "following_count",
    "followingCount",
    "follow_count",
    "followCount",
    "follow_num",
    "followNum",
    "following",
    "followings",
    "attention_count",
    "attentionCount",
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
const WORK_ID_KEYS: &[&str] = &[
    "objectId",
    "object_id",
    "exportId",
    "export_id",
    "feedId",
    "feed_id",
    "id",
    "nonceId",
    "nonce_id",
];
const WORK_TITLE_KEYS: &[&str] = &[
    "richTextTitle",
    "title",
    "shortTitle",
    "description",
    "content",
    "desc",
];
const WORK_COVER_KEYS: &[&str] = &[
    "thumbUrl",
    "thumb_url",
    "coverUrl",
    "cover_url",
    "cover",
    "url",
    "imageUrl",
    "image_url",
];
const WORK_LINK_KEYS: &[&str] = &["url", "link", "detailUrl", "detail_url", "shortLink", "short_link"];
const WORK_TIME_KEYS: &[&str] = &[
    "createTime",
    "create_time",
    "createTimestamp",
    "create_timestamp",
    "createtime",
    "publishTime",
    "publish_time",
    "publishTimestamp",
    "publish_timestamp",
    "timestamp",
];
const WORK_VIEW_KEYS: &[&str] = &["browse", "browseCount", "readCount", "read_count", "playCount", "play_count", "views"];
const WORK_LIKE_KEYS: &[&str] = &["like", "likeCount", "like_count", "likes"];
const WORK_COMMENT_KEYS: &[&str] = &["comment", "commentCount", "comment_count", "comments"];
const WORK_COLLECT_KEYS: &[&str] = &["favCount", "fav_count", "favoriteCount", "favorite_count", "collectCount", "collect_count"];
const WORK_SHARE_KEYS: &[&str] = &["forwardCount", "forward_count", "share", "shareCount", "share_count", "shares"];
const WORK_FULL_PLAY_RATE_KEYS: &[&str] = &["fullPlayRate", "full_play_rate", "completePlayRate", "complete_play_rate"];
const WORK_AVG_PLAY_TIME_KEYS: &[&str] = &["avgPlayTimeSec", "avg_play_time_sec", "avgPlayTime", "avg_play_time"];

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

    let uid = first_string_deep(finder_user, UID_KEYS).unwrap_or_default();
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
        following_count: first_count(finder_user, FOLLOWING_COUNT_KEYS),
        like_count: first_count(finder_user, LIKE_COUNT_KEYS),
        login_cookie,
    })
}

pub(super) async fn fetch_wx_channels_account_content(
    cookie_header: &str,
    login_cookie: String,
    account_id: &str,
) -> Result<ChannelAccountContent, String> {
    let profile = fetch_wx_channels_account_from_cookie(cookie_header, login_cookie)
        .await
        .map_err(|error| plugin_error_message(&error))?;
    let now = Utc::now();
    let (previous_day, yesterday) = wx_yesterday_keypoint_window_seconds();
    let statistic_body = json!({
        "interval": WX_INTERVAL_DAY,
        "startTs": previous_day,
        "endTs": yesterday,
    });
    let fans_value = request_plugin_json_with_body(
        "POST",
        STATISTIC_FANS_TREND_API,
        cookie_header,
        STATISTIC_HEADERS,
        Some(statistic_body.clone()),
    )
    .await
    .map_err(|error| format!("视频号关注数据同步失败: {error}"))?;
    ensure_wx_success(&fans_value, "视频号关注数据同步失败")?;

    let post_value = request_plugin_json_with_body(
        "POST",
        STATISTIC_POST_TOTAL_API,
        cookie_header,
        STATISTIC_HEADERS,
        Some(statistic_body),
    )
    .await
    .map_err(|error| format!("视频号作品数据同步失败: {error}"))?;
    ensure_wx_success(&post_value, "视频号作品数据同步失败")?;

    let overview = ChannelAccountOverview {
        account_id: account_id.to_string(),
        platform_id: "wechat-channels".to_string(),
        period_days: 1,
        metrics: vec![
            overview_metric("netAdd", "净增关注", keypoint_sum(&fans_value, &["netAdd", "net_add", "newFollow", "newFollowCount"])),
            overview_metric("browse", "新增播放", keypoint_sum(&post_value, &["browse", "play", "playCount", "readCount"])),
            overview_metric("like", "新增点赞", keypoint_sum(&post_value, &["like", "likeCount", "likedCount"])),
            overview_metric("comment", "新增评论", keypoint_sum(&post_value, &["comment", "commentCount", "comments"])),
        ],
        summary: Some("数据统计时间为昨日 00:00 至 23:59".to_string()),
        updated_at: Some(now),
        sync_status: "synced".to_string(),
        error: None,
    };
    let latest_video_work = fetch_wx_channels_latest_work(cookie_header, account_id, "video")
        .await
        .unwrap_or(None);
    let latest_article_work = fetch_wx_channels_latest_work(cookie_header, account_id, "article")
        .await
        .unwrap_or(None);

    Ok(ChannelAccountContent {
        account_id: account_id.to_string(),
        platform_id: "wechat-channels".to_string(),
        profile: Some(ChannelAccountProfileSnapshot {
            account_id: account_id.to_string(),
            platform_id: "wechat-channels".to_string(),
            followers: profile.fans_count,
            following: profile.following_count,
            likes: profile.like_count,
            last_sync_at: Some(now),
            updated_at: Some(now),
            sync_status: "synced".to_string(),
            error: None,
        }),
        overview_yesterday: Some(overview.clone()),
        overview_seven: Some(overview),
        overview_thirty: None,
        latest_work: latest_video_work,
        latest_work_seven: latest_article_work,
        latest_work_thirty: None,
        sync_status: "synced".to_string(),
        error: None,
        ..Default::default()
    })
}

pub(super) async fn fetch_wx_channels_works_page(
    cookie_header: &str,
    account_id: &str,
    page_key: &str,
    work_type: Option<&str>,
) -> Result<ChannelWorksPage, String> {
    fetch_wx_channels_works_page_for_source(
        cookie_header,
        account_id,
        page_key,
        work_type,
        WxPostListSource::WorksTab,
    )
    .await
}

async fn fetch_wx_channels_works_page_for_source(
    cookie_header: &str,
    account_id: &str,
    page_key: &str,
    work_type: Option<&str>,
    source: WxPostListSource,
) -> Result<ChannelWorksPage, String> {
    let work_kind = WxWorkKind::from_option(work_type);
    let work_type = work_kind.as_str();
    let current_page = wx_page_number(page_key);
    let request_config = source.request_config(work_kind);
    let url = request_config.url();
    let headers = [
        ("Origin", "https://channels.weixin.qq.com"),
        ("Referer", request_config.referer),
        ("Content-Type", "application/json"),
    ];
    let value = request_plugin_json_with_body(
        "POST",
        &url,
        cookie_header,
        &headers,
        Some(request_config.body(current_page, work_kind.userpage_type())),
    )
    .await
    .map_err(|error| format!("视频号作品列表同步失败: {error}"))?;
    ensure_wx_success(&value, "视频号作品列表同步失败")?;

    let data = response_data(&value);
    let works = first_list(
        data.unwrap_or(&value),
        &["list", "feedList", "objectList", "items"],
    )
    .into_iter()
    .filter_map(|item| parse_wx_work(item, account_id, work_kind))
    .collect::<Vec<_>>();
    let total_count =
        data.and_then(|value| first_count(value, &["totalCount", "total_count", "total"]));
    let has_more = total_count
        .map(|total| {
            (current_page * request_config.page_size) < i64::try_from(total).unwrap_or(i64::MAX)
        })
        .unwrap_or_else(|| works.len() >= request_config.page_size as usize);

    Ok(ChannelWorksPage {
        account_id: account_id.to_string(),
        platform_id: "wechat-channels".to_string(),
        page_key: page_key.to_string(),
        work_type: Some(work_type.to_string()),
        next_page_key: if has_more {
            Some((current_page + 1).to_string())
        } else {
            None
        },
        has_more,
        works,
        updated_at: Some(Utc::now()),
        sync_status: "synced".to_string(),
        error: None,
    })
}

async fn fetch_wx_channels_latest_work(
    cookie_header: &str,
    account_id: &str,
    work_type: &str,
) -> Result<Option<ChannelContentWork>, String> {
    fetch_wx_channels_works_page_for_source(
        cookie_header,
        account_id,
        "1",
        Some(work_type),
        WxPostListSource::LatestCard,
    )
    .await
    .map(|page| page.works.into_iter().next())
}

fn ensure_wx_success(value: &Value, fallback: &str) -> Result<(), String> {
    let err_code = first_i64(value, &["errCode", "errcode", "code"]).unwrap_or(0);
    if err_code == 0 {
        return Ok(());
    }
    let message = first_string(value, &["errMsg", "errmsg", "message", "msg"])
        .unwrap_or_else(|| fallback.to_string());
    if err_code == 300333 || err_code == 300334 || message.contains("登录") {
        return Err("视频号登录已过期，请重新登录。".to_string());
    }
    Err(format!("{message} ({err_code})"))
}

fn response_data(value: &Value) -> Option<&Value> {
    value
        .get("data")
        .or_else(|| value.get("resp"))
        .or_else(|| value.get("result"))
        .filter(|value| !value.is_null())
}

fn wx_yesterday_keypoint_window_seconds() -> (i64, i64) {
    let timezone = FixedOffset::east_opt(8 * 3600).expect("valid timezone");
    let today = Utc::now().with_timezone(&timezone).date_naive();
    let yesterday = today.pred_opt().unwrap_or(today);
    let previous_day = yesterday.pred_opt().unwrap_or(yesterday);
    let yesterday_start = timezone
        .from_local_datetime(&yesterday.and_hms_opt(0, 0, 0).expect("valid midnight"))
        .single()
        .expect("valid local time")
        .timestamp();
    let previous_start = timezone
        .from_local_datetime(&previous_day.and_hms_opt(0, 0, 0).expect("valid midnight"))
        .single()
        .expect("valid local time")
        .timestamp();
    (previous_start, yesterday_start)
}

fn overview_metric(key: &str, label: &str, value: Option<i64>) -> ChannelOverviewMetric {
    ChannelOverviewMetric {
        key: key.to_string(),
        label: label.to_string(),
        value: value.map(|value| value.to_string()).or_else(|| Some("-".to_string())),
        compare_label: None,
        trend: None,
        tone: None,
    }
}

fn keypoint_sum(value: &Value, keys: &[&str]) -> Option<i64> {
    let data = response_data(value).unwrap_or(value);
    let total_data = data.get("totalData").or_else(|| data.get("total_data")).unwrap_or(data);
    keys.iter()
        .find_map(|key| find_value_by_key(total_data, key))
        .and_then(sum_keypoint_now)
}

fn sum_keypoint_now(value: &Value) -> Option<i64> {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                return None;
            }
            let start = if items.len() > 1 { items.len() / 2 } else { 0 };
            Some(items[start..].iter().filter_map(signed_number).sum())
        }
        Value::Object(map) => map
            .get("now")
            .or_else(|| map.get("value"))
            .and_then(signed_number),
        _ => signed_number(value),
    }
}

fn first_list<'a>(value: &'a Value, keys: &[&str]) -> Vec<&'a Value> {
    if let Value::Array(items) = value {
        return items.iter().collect();
    }
    if let Value::Object(map) = value {
        for key in keys {
            if let Some(Value::Array(items)) = map.get(*key) {
                return items.iter().collect();
            }
        }
        for child in map.values() {
            let nested = first_list(child, keys);
            if !nested.is_empty() {
                return nested;
            }
        }
    }
    Vec::new()
}

fn parse_wx_work(
    value: &Value,
    account_id: &str,
    work_kind: WxWorkKind,
) -> Option<ChannelContentWork> {
    let work_type = work_kind.as_str();
    let id = first_string_deep(value, WORK_ID_KEYS)?;
    let title = first_string_deep(value, WORK_TITLE_KEYS)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "未命名作品".to_string());
    let cover_url = first_profile_image(value, WORK_COVER_KEYS).map(normalize_image_url);
    let views = first_count(value, WORK_VIEW_KEYS);
    let likes = first_count(value, WORK_LIKE_KEYS);
    let comments = first_count(value, WORK_COMMENT_KEYS);
    let collects = first_count(value, WORK_COLLECT_KEYS);
    let shares = first_count(value, WORK_SHARE_KEYS);
    let full_play_rate = first_decimal_deep(value, WORK_FULL_PLAY_RATE_KEYS).map(percent_text);
    let avg_play_time = first_decimal_deep(value, WORK_AVG_PLAY_TIME_KEYS).map(duration_text);
    let metrics = wx_work_metrics(
        work_kind,
        views,
        likes,
        comments,
        collects,
        shares,
        full_play_rate.clone(),
        avg_play_time.clone(),
    );

    Some(ChannelContentWork {
        id: format!("wechat-channels-{work_type}-{id}"),
        platform_id: "wechat-channels".to_string(),
        account_id: account_id.to_string(),
        title,
        cover_url,
        link: first_string_deep(value, WORK_LINK_KEYS),
        published_at: first_time_deep(value, WORK_TIME_KEYS),
        status: "published".to_string(),
        views,
        impressions: None,
        likes,
        collects,
        comments,
        shares,
        cover_click_rate: full_play_rate,
        avg_view_time: avg_play_time,
        gained_followers: None,
        data_updated_at: None,
        metrics,
        badges: wx_work_badges(value),
        work_type: Some(work_type.to_string()),
    })
}

fn wx_work_badges(value: &Value) -> Vec<String> {
    let mut badges = Vec::new();
    if wx_sticky_op_status(value) == Some(2) {
        push_unique_badge(&mut badges, "置顶");
    }
    if let Some(label) = wx_visibility_badge(value) {
        push_unique_badge(&mut badges, label);
    }
    badges
}

fn wx_sticky_op_status(value: &Value) -> Option<i64> {
    find_value_by_key(value, "stickyOpStatus")
        .or_else(|| find_value_by_key(value, "sticky_op_status"))
        .and_then(signed_number)
}

fn wx_visibility_badge(value: &Value) -> Option<String> {
    for key in ["visibleType", "visible_type", "visibleRange", "visible_range", "visibility"] {
        if let Some(label) = find_value_by_key(value, key)
            .and_then(value_to_text)
            .and_then(|value| wx_visible_type_label(&value))
        {
            return Some(label);
        }
    }
    for key in ["privateStatus", "private_status", "privateType", "private_type"] {
        if let Some(label) = find_value_by_key(value, key)
            .and_then(value_to_text)
            .and_then(|value| wx_private_status_label(&value))
        {
            return Some(label);
        }
    }
    None
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let text = text.trim();
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn wx_visible_type_label(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if value.contains("公开") || lower.contains("public") {
        return Some("公开".to_string());
    }
    if value.contains("仅自己") || value.contains("私密") || lower.contains("private") {
        return Some("仅自己可见".to_string());
    }
    if value.contains("关注") || lower.contains("follow") {
        return Some("关注可见".to_string());
    }
    if value.contains("好友") || lower.contains("friend") {
        return Some("好友可见".to_string());
    }
    match value {
        "0" => Some("公开".to_string()),
        "1" => Some("公开".to_string()),
        "2" => Some("关注可见".to_string()),
        "3" => Some("仅自己可见".to_string()),
        _ if value.chars().count() <= 12 => Some(value.to_string()),
        _ => None,
    }
}

fn wx_private_status_label(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if value.contains("仅自己") || value.contains("私密") || lower.contains("private") {
        return Some("仅自己可见".to_string());
    }
    if value.contains("公开") || lower.contains("public") {
        return Some("公开".to_string());
    }
    match value {
        "0" | "false" => Some("公开".to_string()),
        "1" | "true" => Some("仅自己可见".to_string()),
        _ if value.chars().count() <= 12 => Some(value.to_string()),
        _ => None,
    }
}

fn push_unique_badge(badges: &mut Vec<String>, label: impl Into<String>) {
    let label = label.into().trim().to_string();
    if !label.is_empty() && !badges.iter().any(|item| item == &label) {
        badges.push(label);
    }
}

fn wx_work_metrics(
    work_kind: WxWorkKind,
    views: Option<u64>,
    likes: Option<u64>,
    comments: Option<u64>,
    collects: Option<u64>,
    shares: Option<u64>,
    full_play_rate: Option<String>,
    avg_play_time: Option<String>,
) -> Vec<ChannelWorkMetric> {
    let mut metrics = [
        (
            "views",
            work_kind.views_label(),
            views.map(|value| value.to_string()),
        ),
        ("likes", "点赞", likes.map(|value| value.to_string())),
        ("comments", "评论", comments.map(|value| value.to_string())),
        ("collects", "收藏", collects.map(|value| value.to_string())),
        ("shares", "转发", shares.map(|value| value.to_string())),
    ]
    .into_iter()
    .map(|(key, label, value)| ChannelWorkMetric {
        key: key.to_string(),
        label: label.to_string(),
        value: value.or_else(|| Some("-".to_string())),
    })
    .collect::<Vec<_>>();
    if work_kind.includes_play_metrics() {
        metrics.push(ChannelWorkMetric {
            key: "fullPlayRate".to_string(),
            label: "完播率".to_string(),
            value: full_play_rate.or_else(|| Some("-".to_string())),
        });
        metrics.push(ChannelWorkMetric {
            key: "avgPlayTime".to_string(),
            label: "平均播放时长".to_string(),
            value: avg_play_time.or_else(|| Some("-".to_string())),
        });
    }
    metrics
}

fn wx_page_number(page_key: &str) -> i64 {
    page_key
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn first_time_deep(value: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key).and_then(time_from_value) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| first_time_deep(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_time_deep(value, keys)),
        _ => None,
    }
}

fn first_decimal_deep(value: &Value, keys: &[&str]) -> Option<f64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key).and_then(decimal_from_value) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| first_decimal_deep(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_decimal_deep(value, keys)),
        _ => None,
    }
}

fn time_from_value(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(number) = value.as_i64() {
        let seconds = if number > 9_999_999_999 { number / 1000 } else { number };
        return DateTime::from_timestamp(seconds, 0);
    }
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(number) = text.parse::<i64>() {
        return time_from_value(&Value::from(number));
    }
    DateTime::parse_from_rfc3339(text)
        .map(|value| value.with_timezone(&Utc))
        .ok()
}

fn find_value_by_key<'a>(value: &'a Value, target: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            if let Some(value) = map.get(target) {
                return Some(value);
            }
            map.values().find_map(|value| find_value_by_key(value, target))
        }
        Value::Array(items) => items.iter().find_map(|value| find_value_by_key(value, target)),
        _ => None,
    }
}

fn decimal_from_value(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))
        .or_else(|| value.as_u64().map(|value| value as f64))
        .or_else(|| value.as_str()?.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
}

fn signed_number(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| decimal_from_value(value).map(|value| value.round() as i64))
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
}

fn percent_text(value: f64) -> String {
    let percent = if value.abs() <= 1.0 { value * 100.0 } else { value };
    format!("{}%", compact_decimal(percent))
}

fn duration_text(value: f64) -> String {
    let seconds = if value > 600.0 { value / 1000.0 } else { value };
    format!("{}秒", compact_decimal(seconds))
}

fn compact_decimal(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    if (rounded.fract()).abs() < f64::EPSILON {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded:.1}")
    }
}
