use super::*;
use chrono::{Duration, FixedOffset, NaiveDateTime, TimeZone};

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
const CREATOR_DATA_OVERVIEW_URL: &str = "https://creator.douyin.com/creator-micro/data-center/operation";
const CREATOR_DATA_WORKS_URL: &str = "https://creator.douyin.com/creator-micro/content/manage";
const PC_USER_INFO_API: &str = "https://creator.douyin.com/aweme/v1/creator/pc/user/info/";
const USER_INFO_API: &str = "https://creator.douyin.com/aweme/v1/creator/user/info/";
const OVERVIEW_DASHBOARD_API: &str = "https://creator.douyin.com/janus/douyin/creator/data/overview/dashboard";
const HOMEPAGE_LATEST_WORKS_API: &str = "https://creator.douyin.com/web/api/creator/item/list";
const WORKS_LIST_API: &str = "https://creator.douyin.com/janus/douyin/creator/pc/work_list";
const WORK_DETAIL_API: &str = "https://creator.douyin.com/janus/douyin/creator/pc/work_detail";
const CREATOR_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://creator.douyin.com"),
    ("Referer", CREATOR_HOME_URL),
];
const DATA_OVERVIEW_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://creator.douyin.com"),
    ("Referer", CREATOR_DATA_OVERVIEW_URL),
];
const DATA_WORKS_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://creator.douyin.com"),
    ("Referer", CREATOR_DATA_WORKS_URL),
];
const WORK_DETAIL_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://creator.douyin.com"),
    (
        "Referer",
        "https://creator.douyin.com/creator-micro/work-management/work-detail/",
    ),
    ("Agw-Js-Conv", "str"),
];

const PC_USER_ID_KEYS: &[&str] = &["uid", "user_id", "userId", "sec_uid", "secUid"];
const CREATOR_USER_ID_KEYS: &[&str] = &[
    "douyin_unique_id",
    "unique_id",
    "uniqueId",
    "uid",
    "user_id",
    "userId",
    "sec_uid",
    "secUid",
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
    "total_favorited",
    "totalFavorited",
    "favorited_count",
    "favoritedCount",
    "liked_count",
    "likedCount",
    "like_count",
    "likeCount",
    "digg_count",
    "diggCount",
    "total_like_count",
    "totalLikeCount",
];
const WORK_ID_KEYS: &[&str] = &["item_id_plain", "aweme_id", "awemeId", "item_id", "itemId", "id"];
const WORK_TITLE_KEYS: &[&str] = &["title", "desc", "description"];
const WORK_COVER_KEYS: &[&str] = &[
    "Cover",
    "cover_image_url",
    "cover_url",
    "coverUrl",
    "cover",
    "image",
    "images",
    "url_list",
];
const WORK_LINK_KEYS: &[&str] = &["item_link", "share_url", "shareUrl", "url", "link"];
const WORK_TIME_KEYS: &[&str] = &["create_time", "publish_time", "publishTime"];
const WORK_VIEW_KEYS: &[&str] = &["play_count", "playCount", "play_cnt", "playCnt", "view_count", "viewCount"];
const WORK_LIKE_KEYS: &[&str] = &["like_count", "likeCount", "like_cnt", "likeCnt", "digg_count", "diggCount"];
const WORK_COMMENT_KEYS: &[&str] = &["comment_count", "commentCount", "comment_cnt", "commentCnt"];
const WORK_SHARE_KEYS: &[&str] = &["share_count", "shareCount", "share_cnt", "shareCnt"];
const WORK_COLLECT_KEYS: &[&str] = &["collect_count", "collectCount", "favorite_count", "favoriteCount"];

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
    let uid = verify_info
        .and_then(|value| first_string_deep(value, CREATOR_USER_ID_KEYS))
        .or_else(|| first_string_deep(&user, CREATOR_USER_ID_KEYS))
        .or_else(|| first_string_deep(&pc_user, PC_USER_ID_KEYS))
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
    let following_count = verify_info
        .and_then(|value| first_count(value, FOLLOWING_COUNT_KEYS))
        .or_else(|| first_count(&user, FOLLOWING_COUNT_KEYS));
    let like_count = verify_info
        .and_then(|value| first_count(value, LIKE_COUNT_KEYS))
        .or_else(|| first_count(&user, LIKE_COUNT_KEYS));

    Ok(PluginAccountInfo {
        uid: uid.clone(),
        account: uid,
        nickname,
        avatar,
        fans_count,
        following_count,
        like_count,
        login_cookie,
    })
}

pub(super) async fn fetch_douyin_account_content(
    cookie_header: &str,
    login_cookie: String,
    account_id: &str,
) -> Result<ChannelAccountContent, String> {
    let now = Utc::now();
    let profile = fetch_douyin_creator_account_from_cookie(cookie_header, login_cookie).await?;
    let overview_yesterday = fetch_douyin_overview(cookie_header, account_id, 1, now).await?;
    let overview_seven = fetch_douyin_overview(cookie_header, account_id, 7, now).await?;
    let overview_thirty = fetch_douyin_overview(cookie_header, account_id, 30, now).await?;
    let latest_work = fetch_douyin_latest_work(cookie_header, account_id)
        .await
        .unwrap_or(None);

    Ok(ChannelAccountContent {
        account_id: account_id.to_string(),
        platform_id: "douyin".to_string(),
        profile: Some(ChannelAccountProfileSnapshot {
            account_id: account_id.to_string(),
            platform_id: "douyin".to_string(),
            followers: profile.fans_count,
            following: profile.following_count,
            likes: profile.like_count,
            last_sync_at: Some(now),
            updated_at: Some(now),
            sync_status: "synced".to_string(),
            error: None,
        }),
        overview_yesterday: Some(overview_yesterday),
        overview_seven: Some(overview_seven),
        overview_thirty: Some(overview_thirty),
        latest_work: latest_work.clone(),
        latest_work_seven: latest_work.clone(),
        latest_work_thirty: latest_work,
        sync_status: "synced".to_string(),
        ..Default::default()
    })
}

async fn fetch_douyin_latest_work(
    cookie_header: &str,
    account_id: &str,
) -> Result<Option<ChannelContentWork>, String> {
    let (start_time, end_time) = douyin_latest_work_window_millis();
    let params = vec![
        ("count", "10".to_string()),
        ("fields", "visibility,metrics,review".to_string()),
        ("status_list[]", "102".to_string()),
        ("status_list[]", "143".to_string()),
        ("start_time", start_time.to_string()),
        ("end_time", end_time.to_string()),
        ("need_long_article", "true".to_string()),
    ];
    let url = Url::parse_with_params(HOMEPAGE_LATEST_WORKS_API, params)
        .map_err(|error| format!("抖音最新作品地址无效: {error}"))?;
    let value = request_plugin_json(
        "GET",
        url.as_str(),
        cookie_header,
        CREATOR_HEADERS,
    )
    .await
    .map_err(|error| format!("抖音最新作品接口不可用: {error}"))?;
    if !douyin_response_success(&value) {
        return Err(douyin_error_message(&value, "抖音最新作品读取失败"));
    }

    let Some(item) = value
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
    else {
        return Ok(None);
    };
    let Some(mut work) = parse_douyin_work(item, account_id) else {
        return Ok(None);
    };
    apply_douyin_list_metrics(&mut work, Some(item));
    let _ = apply_douyin_work_detail(cookie_header, &mut work).await;
    Ok(Some(work))
}

fn douyin_latest_work_window_millis() -> (i64, i64) {
    let timezone = FixedOffset::east_opt(8 * 3600).expect("valid timezone");
    let today = Utc::now().with_timezone(&timezone).date_naive();
    let start_day = today.checked_sub_signed(Duration::days(30)).unwrap_or(today);
    let start_time = timezone
        .from_local_datetime(&start_day.and_hms_opt(0, 0, 0).expect("valid midnight"))
        .single()
        .expect("valid local time")
        .timestamp_millis();
    (start_time, Utc::now().timestamp_millis())
}

pub(super) async fn fetch_douyin_works_page(
    cookie_header: &str,
    account_id: &str,
    page_key: &str,
) -> Result<ChannelWorksPage, String> {
    let cursor = page_key.trim();
    let cursor = if cursor.is_empty() { "0" } else { cursor };
    let url = Url::parse_with_params(
        WORKS_LIST_API,
        [
            ("status", "0"),
            ("count", "12"),
            ("max_cursor", cursor),
            ("scene", "star_atlas"),
            ("device_platform", "android"),
            ("aid", "1128"),
        ],
    )
    .map_err(|error| format!("抖音作品列表地址无效: {error}"))?;
    let value = request_plugin_json(
        "GET",
        url.as_str(),
        cookie_header,
        DATA_WORKS_HEADERS,
    )
    .await
    .map_err(|error| format!("抖音作品列表接口不可用: {error}"))?;
    if !douyin_response_success(&value) {
        return Err(douyin_error_message(&value, "抖音作品列表读取失败"));
    }

    let detail_items = value.get("items").and_then(Value::as_array);
    let works = value
        .get("aweme_list")
        .or_else(|| value.get("items"))
        .or_else(|| value.get("item_list"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    let mut work = parse_douyin_work(item, account_id)?;
                    apply_douyin_list_metrics(
                        &mut work,
                        detail_items.and_then(|items| items.get(index)),
                    );
                    Some(work)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let has_more = value
        .get("has_more")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let next_page_key = if has_more {
        value
            .get("max_cursor")
            .and_then(value_to_text)
            .or_else(|| first_string_deep(&value, &["cursor"]))
            .filter(|value| !value.trim().is_empty() && value != "0")
    } else {
        None
    };

    Ok(ChannelWorksPage {
        account_id: account_id.to_string(),
        platform_id: "douyin".to_string(),
        page_key: page_key.trim().to_string(),
        work_type: None,
        next_page_key,
        has_more,
        works,
        updated_at: Some(Utc::now()),
        sync_status: "synced".to_string(),
        error: None,
    })
}

async fn fetch_douyin_overview(
    cookie_header: &str,
    account_id: &str,
    period_days: u16,
    now: DateTime<Utc>,
) -> Result<ChannelAccountOverview, String> {
    let value = request_plugin_json_with_body(
        "POST",
        OVERVIEW_DASHBOARD_API,
        cookie_header,
        DATA_OVERVIEW_HEADERS,
        Some(serde_json::json!({ "recent_days": period_days })),
    )
    .await
    .map_err(|error| format!("抖音总览接口不可用: {error}"))?;
    if !douyin_response_success(&value) {
        return Err(douyin_error_message(&value, "抖音总览读取失败"));
    }

    let data = value.get("data").unwrap_or(&value);
    Ok(ChannelAccountOverview {
        account_id: account_id.to_string(),
        platform_id: "douyin".to_string(),
        period_days,
        metrics: douyin_overview_metrics(data),
        summary: None,
        updated_at: Some(now),
        sync_status: "synced".to_string(),
        error: None,
    })
}

fn douyin_overview_metrics(data: &Value) -> Vec<ChannelOverviewMetric> {
    let metrics = data
        .get("metrics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    [
        ("play_cnt", "播放量"),
        ("homepage_view_cnt", "主页访问"),
        ("digg_cnt", "作品点赞"),
        ("share_count", "作品分享"),
        ("comment_cnt", "作品评论"),
        ("cover_click_ratio", "封面点击率"),
        ("net_fans_cnt", "净增粉丝"),
        ("cancel_fans_cnt", "取关粉丝"),
        ("total_fans_cnt", "总粉丝量"),
    ]
    .into_iter()
    .map(|(key, label)| ChannelOverviewMetric {
        key: key.to_string(),
        label: metrics
            .iter()
            .find(|item| item.get("english_metric_name").and_then(Value::as_str) == Some(key))
            .and_then(|item| item.get("metric_name").and_then(Value::as_str))
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(label)
            .to_string(),
        value: metrics
            .iter()
            .find(|item| item.get("english_metric_name").and_then(Value::as_str) == Some(key))
            .and_then(|item| douyin_dashboard_metric_value(item, key)),
        compare_label: None,
        trend: None,
        tone: None,
    })
    .collect()
}

fn douyin_dashboard_metric_value(metric: &Value, key: &str) -> Option<String> {
    let value = metric.get("metric_value")?;
    if key == "cover_click_ratio" {
        return value.as_f64().map(format_douyin_percent);
    }
    value_to_text(value)
}

fn format_douyin_percent(value: f64) -> String {
    if !value.is_finite() || value.abs() < f64::EPSILON {
        return "0".to_string();
    }
    let text = format!("{:.2}", value * 100.0);
    format!("{}%", text.trim_end_matches('0').trim_end_matches('.'))
}

async fn apply_douyin_work_detail(
    cookie_header: &str,
    work: &mut ChannelContentWork,
) -> Result<(), String> {
    let item_id = work.id.trim();
    if item_id.is_empty() {
        return Ok(());
    }
    let url = Url::parse_with_params(WORK_DETAIL_API, [("item_id", item_id)])
        .map_err(|error| format!("抖音作品详情地址无效: {error}"))?;
    let value = request_plugin_json(
        "GET",
        url.as_str(),
        cookie_header,
        WORK_DETAIL_HEADERS,
    )
    .await
    .map_err(|error| format!("抖音作品详情接口不可用: {error}"))?;
    if !douyin_response_success(&value) {
        return Err(douyin_error_message(&value, "抖音作品详情读取失败"));
    }

    let detail = value
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first());
    let legacy_detail = value
        .get("item_list")
        .and_then(Value::as_array)
        .and_then(|items| items.first());
    let metrics = detail.and_then(|value| value.get("metrics"));
    let statistics = legacy_detail.and_then(|value| value.get("statistics"));
    let summarize_data = legacy_detail.and_then(|value| value.get("summarize_data"));

    work.views = douyin_metric_count(metrics, "view_count")
        .or_else(|| first_count_optional(statistics, &["play_count"]))
        .or(work.views);
    work.likes = douyin_metric_count(metrics, "like_count")
        .or_else(|| first_count_optional(statistics, &["digg_count"]))
        .or(work.likes);
    work.comments = douyin_metric_count(metrics, "comment_count")
        .or_else(|| first_count_optional(statistics, &["comment_count"]))
        .or(work.comments);
    work.shares = douyin_metric_count(metrics, "share_count")
        .or_else(|| first_count_optional(statistics, &["share_count"]))
        .or(work.shares);
    work.collects = douyin_metric_count(metrics, "favorite_count")
        .or_else(|| first_count_optional(statistics, &["collect_count"]))
        .or(work.collects);
    work.cover_click_rate = douyin_metric_percent(metrics, "cover_click_rate")
        .or_else(|| work.cover_click_rate.clone());
    work.avg_view_time = douyin_seconds_value(metrics, "avg_view_second")
        .or_else(|| douyin_seconds_value(summarize_data, "play_avg_time"))
        .or_else(|| work.avg_view_time.clone());
    let gained_followers = douyin_metric_count(metrics, "subscribe_count")
        .or_else(|| first_count_optional(summarize_data, &["new_fans_count"]))
        .and_then(|value| i64::try_from(value).ok());
    work.gained_followers = gained_followers.or(work.gained_followers);
    work.data_updated_at = Some(Utc::now());
    if work.work_type.is_none() {
        work.work_type = detail
            .and_then(douyin_work_type)
            .or_else(|| legacy_detail.and_then(douyin_work_type));
    }
    work.metrics = douyin_latest_work_detail_metrics(
        work.work_type.as_deref(),
        metrics,
        statistics,
        summarize_data,
    );
    Ok(())
}

fn douyin_latest_work_detail_metrics(
    work_type: Option<&str>,
    metrics: Option<&Value>,
    statistics: Option<&Value>,
    summarize_data: Option<&Value>,
) -> Vec<ChannelWorkMetric> {
    let mut items = douyin_metrics_from_specs(metrics, statistics, DOUYIN_LATEST_WORK_BASE_METRICS);
    match work_type {
        Some("video") => items.extend(douyin_metrics_from_specs(
            metrics,
            summarize_data,
            DOUYIN_LATEST_VIDEO_METRICS,
        )),
        Some("article") | Some("image") | Some("note") => {
            items.extend(douyin_metrics_from_specs(
                metrics,
                summarize_data,
                DOUYIN_LATEST_ARTICLE_METRICS,
            ));
        }
        _ => items.extend(douyin_metrics_from_specs(
            metrics,
            None,
            DOUYIN_LATEST_FALLBACK_METRICS,
        )),
    }
    items
}

#[derive(Clone, Copy)]
enum DouyinMetricValueKind {
    Count,
    Percent,
    Number,
    Seconds,
}

#[derive(Clone, Copy)]
struct DouyinMetricSpec {
    key: &'static str,
    label: &'static str,
    metric_key: &'static str,
    value_kind: DouyinMetricValueKind,
    fallback_key: Option<&'static str>,
}

impl DouyinMetricSpec {
    const fn count(
        key: &'static str,
        label: &'static str,
        metric_key: &'static str,
        fallback_key: Option<&'static str>,
    ) -> Self {
        Self {
            key,
            label,
            metric_key,
            value_kind: DouyinMetricValueKind::Count,
            fallback_key,
        }
    }

    const fn percent(
        key: &'static str,
        label: &'static str,
        metric_key: &'static str,
        fallback_key: Option<&'static str>,
    ) -> Self {
        Self {
            key,
            label,
            metric_key,
            value_kind: DouyinMetricValueKind::Percent,
            fallback_key,
        }
    }

    const fn number(
        key: &'static str,
        label: &'static str,
        metric_key: &'static str,
    ) -> Self {
        Self {
            key,
            label,
            metric_key,
            value_kind: DouyinMetricValueKind::Number,
            fallback_key: None,
        }
    }

    const fn seconds(
        key: &'static str,
        label: &'static str,
        metric_key: &'static str,
        fallback_key: Option<&'static str>,
    ) -> Self {
        Self {
            key,
            label,
            metric_key,
            value_kind: DouyinMetricValueKind::Seconds,
            fallback_key,
        }
    }
}

const DOUYIN_LATEST_WORK_BASE_METRICS: &[DouyinMetricSpec] = &[
    DouyinMetricSpec::count("play", "播放量", "view_count", Some("play_count")),
    DouyinMetricSpec::count("like", "点赞量", "like_count", Some("digg_count")),
    DouyinMetricSpec::count("comment", "评论量", "comment_count", Some("comment_count")),
    DouyinMetricSpec::count("share", "分享量", "share_count", Some("share_count")),
    DouyinMetricSpec::count("favorite", "收藏量", "favorite_count", Some("collect_count")),
];

const DOUYIN_LATEST_VIDEO_METRICS: &[DouyinMetricSpec] = &[
    DouyinMetricSpec::count("danmaku", "弹幕量", "danmaku_count", None),
    DouyinMetricSpec::percent("completionRate", "完播率", "completion_rate", Some("play_finish_ratio")),
    DouyinMetricSpec::percent("bounceRate", "2s跳出率", "bounce_rate_2s", None),
    DouyinMetricSpec::seconds("avgViewSecond", "平均播放时长", "avg_view_second", Some("play_avg_time")),
    DouyinMetricSpec::percent("completionRate5s", "5s完播率", "completion_rate_5s", None),
    DouyinMetricSpec::percent("avgViewProportion", "平均播放占比", "avg_view_proportion", None),
    DouyinMetricSpec::count("subscribe", "涨粉量", "subscribe_count", Some("new_fans_count")),
    DouyinMetricSpec::percent("subscribeRate", "涨粉率", "subscribe_rate", None),
    DouyinMetricSpec::count("unsubscribe", "脱粉量", "unsubscribe_count", None),
    DouyinMetricSpec::percent("unsubscribeRate", "脱粉率", "unsubscribe_rate", None),
    DouyinMetricSpec::count("dislike", "不感兴趣量", "dislike_count", None),
    DouyinMetricSpec::percent("dislikeRate", "不感兴趣率", "dislike_rate", None),
];

const DOUYIN_LATEST_ARTICLE_METRICS: &[DouyinMetricSpec] = &[
    DouyinMetricSpec::percent("descriptionSpreadRate", "文案展开率", "description_spread_rate", None),
    DouyinMetricSpec::number("imageAvgViewCount", "平均浏览图片数", "image_avg_view_count"),
    DouyinMetricSpec::percent("coverClickRate", "封面点击率", "cover_click_rate", None),
    DouyinMetricSpec::percent(
        "descriptionCompletionRate",
        "文案完读率",
        "description_completion_rate",
        None,
    ),
    DouyinMetricSpec::percent("commentEntryRate", "评论进入率", "comment_entry_rate", None),
    DouyinMetricSpec::percent("likeRate", "点赞率", "like_rate", None),
    DouyinMetricSpec::percent("commentRate", "评论率", "comment_rate", None),
    DouyinMetricSpec::count("download", "下载量", "download_count", None),
    DouyinMetricSpec::percent("favoriteRate", "收藏率", "favorite_rate", None),
    DouyinMetricSpec::percent("shareRate", "分享率", "share_rate", None),
    DouyinMetricSpec::percent("dislikeRate", "不感兴趣率", "dislike_rate", None),
    DouyinMetricSpec::count("subscribe", "涨粉量", "subscribe_count", Some("new_fans_count")),
];

const DOUYIN_LATEST_FALLBACK_METRICS: &[DouyinMetricSpec] = &[
    DouyinMetricSpec::percent("descriptionSpreadRate", "文案展开率", "description_spread_rate", None),
    DouyinMetricSpec::percent("bounceRate", "划走率", "bounce_rate_2s", None),
];

const DOUYIN_WORK_ARTICLE_METRICS: &[DouyinMetricSpec] = &[
    DouyinMetricSpec::percent("descriptionSpreadRate", "文案展开率", "description_spread_rate", None),
    DouyinMetricSpec::number("imageAvgViewCount", "平均浏览图片", "image_avg_view_count"),
];

const DOUYIN_WORK_VIDEO_METRICS: &[DouyinMetricSpec] = &[
    DouyinMetricSpec::seconds("avgViewSecond", "平均播放时长", "avg_view_second", None),
    DouyinMetricSpec::percent("completionRate", "完播率", "completion_rate", None),
];

const DOUYIN_WORK_FALLBACK_METRICS: &[DouyinMetricSpec] = &[
    DouyinMetricSpec::percent("descriptionSpreadRate", "文案展开率", "description_spread_rate", None),
];

fn douyin_metrics_from_specs(
    metrics: Option<&Value>,
    fallback: Option<&Value>,
    specs: &[DouyinMetricSpec],
) -> Vec<ChannelWorkMetric> {
    specs
        .iter()
        .map(|spec| douyin_metric_from_spec(metrics, fallback, spec))
        .collect()
}

fn douyin_metric_from_spec(
    metrics: Option<&Value>,
    fallback: Option<&Value>,
    spec: &DouyinMetricSpec,
) -> ChannelWorkMetric {
    let value = match spec.value_kind {
        DouyinMetricValueKind::Count => douyin_metric_count(metrics, spec.metric_key)
            .or_else(|| spec.fallback_key.and_then(|key| first_count_optional(fallback, &[key])))
            .map(|value| value.to_string()),
        DouyinMetricValueKind::Percent => douyin_metric_percent(metrics, spec.metric_key)
            .or_else(|| spec.fallback_key.and_then(|key| douyin_metric_percent(fallback, key))),
        DouyinMetricValueKind::Number => douyin_metric_number_text(metrics, spec.metric_key)
            .or_else(|| spec.fallback_key.and_then(|key| douyin_metric_number_text(fallback, key))),
        DouyinMetricValueKind::Seconds => douyin_seconds_value(metrics, spec.metric_key)
            .or_else(|| spec.fallback_key.and_then(|key| douyin_seconds_value(fallback, key))),
    };
    douyin_work_metric(spec.key, spec.label, value)
}

fn douyin_work_metric(key: &str, label: &str, value: Option<String>) -> ChannelWorkMetric {
    ChannelWorkMetric {
        key: key.to_string(),
        label: label.to_string(),
        value,
    }
}

fn douyin_metric_count(metrics: Option<&Value>, key: &str) -> Option<u64> {
    metrics
        .and_then(|value| value.get(key))
        .and_then(douyin_metric_number)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value.round() as u64)
}

fn douyin_metric_percent(metrics: Option<&Value>, key: &str) -> Option<String> {
    metrics
        .and_then(|value| value.get(key))
        .and_then(douyin_metric_number)
        .map(format_douyin_detail_percent)
}

fn douyin_metric_number_text(metrics: Option<&Value>, key: &str) -> Option<String> {
    metrics
        .and_then(|value| value.get(key))
        .and_then(douyin_metric_number)
        .map(format_douyin_number)
}

fn douyin_seconds_value(metrics: Option<&Value>, key: &str) -> Option<String> {
    metrics
        .and_then(|value| value.get(key))
        .and_then(douyin_metric_number)
        .map(format_douyin_seconds)
}

fn douyin_metric_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse::<f64>().ok())
}

fn first_count_optional(value: Option<&Value>, keys: &[&str]) -> Option<u64> {
    value.and_then(|value| first_count(value, keys))
}

fn format_douyin_detail_percent(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    format!("{:.2}%", value * 100.0)
}

fn format_douyin_seconds(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let text = if value >= 1.0 {
        format_douyin_number(value.round())
    } else {
        format_douyin_number(value)
    };
    format!("{text}秒")
}

fn format_douyin_number(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    if (value.fract()).abs() < 0.000_001 {
        return format!("{}", value.round() as i64);
    }
    let text = format!("{:.2}", value);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn parse_douyin_work(value: &Value, account_id: &str) -> Option<ChannelContentWork> {
    let id = first_string_or_number_deep(value, WORK_ID_KEYS)?;
    let title = first_string_deep(value, WORK_TITLE_KEYS)
        .unwrap_or_else(|| "未命名作品".to_string());
    let views = first_count(value, WORK_VIEW_KEYS);
    let likes = first_count(value, WORK_LIKE_KEYS);
    let comments = first_count(value, WORK_COMMENT_KEYS);
    let shares = first_count(value, WORK_SHARE_KEYS);
    let collects = first_count(value, WORK_COLLECT_KEYS);
    let work_type = douyin_work_type(value);

    Some(ChannelContentWork {
        id: id.clone(),
        platform_id: "douyin".to_string(),
        account_id: account_id.to_string(),
        title,
        cover_url: first_profile_image(value, WORK_COVER_KEYS)
            .map(|url| normalize_platform_image_url("douyin", url)),
        link: first_string_deep(value, WORK_LINK_KEYS),
        published_at: first_time_deep(value, WORK_TIME_KEYS),
        status: "published".to_string(),
        views,
        impressions: None,
        likes,
        collects,
        comments,
        shares,
        cover_click_rate: None,
        avg_view_time: None,
        gained_followers: None,
        data_updated_at: None,
        metrics: douyin_work_metrics(
            work_type.as_deref(),
            views,
            likes,
            comments,
            collects,
            shares,
            None,
        ),
        badges: douyin_work_badges(value),
        work_type,
    })
}

fn apply_douyin_list_metrics(work: &mut ChannelContentWork, detail: Option<&Value>) {
    if work.work_type.is_none() {
        work.work_type = detail.and_then(douyin_work_type);
    }
    let metrics = detail.and_then(|value| value.get("metrics"));
    work.views = douyin_metric_count(metrics, "view_count").or(work.views);
    work.likes = douyin_metric_count(metrics, "like_count").or(work.likes);
    work.comments = douyin_metric_count(metrics, "comment_count").or(work.comments);
    work.shares = douyin_metric_count(metrics, "share_count").or(work.shares);
    work.collects = douyin_metric_count(metrics, "favorite_count").or(work.collects);
    work.cover_click_rate = douyin_metric_percent(metrics, "cover_click_rate")
        .or_else(|| work.cover_click_rate.clone());
    work.metrics = douyin_work_metrics(
        work.work_type.as_deref(),
        work.views,
        work.likes,
        work.comments,
        work.collects,
        work.shares,
        metrics,
    );
    if let Some(detail) = detail {
        for badge in douyin_work_badges(detail) {
            push_unique_badge(&mut work.badges, badge);
        }
    }
}

fn douyin_work_badges(value: &Value) -> Vec<String> {
    let mut badges = Vec::new();
    if first_bool_deep(value, &["is_top", "isTop", "is_pinned", "isPinned", "is_stick", "isStick", "top"])
        .unwrap_or(false)
    {
        push_unique_badge(&mut badges, "置顶");
    }
    if let Some(label) = first_string_deep(
        value,
        &["label_top_text", "labelTopText", "top_label", "topLabel", "tag_text", "tagText"],
    )
    .filter(|label| label.contains("置顶"))
    {
        push_unique_badge(&mut badges, label);
    }
    if let Some(label) = douyin_visibility_badge(value) {
        push_unique_badge(&mut badges, label);
    }
    badges
}

fn douyin_visibility_badge(value: &Value) -> Option<String> {
    first_string_or_number_deep(
        value,
        &[
            "visibility",
            "visible_status",
            "visibleStatus",
            "private_status",
            "privateStatus",
            "private_type",
            "privateType",
        ],
    )
    .and_then(|value| visibility_label(&value))
}

fn first_bool_deep(value: &Value, keys: &[&str]) -> Option<bool> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key).and_then(value_to_bool) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| first_bool_deep(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_bool_deep(value, keys)),
        _ => None,
    }
}

fn value_to_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(number) => number
            .as_i64()
            .map(|value| value != 0)
            .or_else(|| number.as_u64().map(|value| value != 0))
            .or_else(|| number.as_f64().map(|value| value.abs() > f64::EPSILON)),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => Some(true),
            "false" | "0" | "no" | "n" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn visibility_label(value: &str) -> Option<String> {
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
    if value.contains("好友") || lower.contains("friend") {
        return Some("好友可见".to_string());
    }
    match value {
        "0" => Some("公开".to_string()),
        "1" => Some("仅自己可见".to_string()),
        "2" => Some("好友可见".to_string()),
        "3" => Some("部分可见".to_string()),
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

fn first_string_or_number_deep(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(value) = first_string_deep(value, keys).filter(|value| !value.trim().is_empty()) {
        return Some(value);
    }
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(text) = map.get(*key).and_then(value_to_text) {
                    return Some(text);
                }
            }
            map.values()
                .find_map(|value| first_string_or_number_deep(value, keys))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|value| first_string_or_number_deep(value, keys)),
        _ => None,
    }
}

fn douyin_work_metrics(
    work_type: Option<&str>,
    views: Option<u64>,
    likes: Option<u64>,
    comments: Option<u64>,
    collects: Option<u64>,
    shares: Option<u64>,
    detail_metrics: Option<&Value>,
) -> Vec<ChannelWorkMetric> {
    let mut metrics = vec![
        douyin_count_value_metric("play", "播放", views),
        douyin_count_value_metric("like", "点赞", likes),
        douyin_count_value_metric("comment", "评论", comments),
        douyin_count_value_metric("collect", "收藏", collects),
    ];

    match work_type {
        Some("article") | Some("image") | Some("note") => {
            metrics.extend(douyin_metrics_from_specs(
                detail_metrics,
                None,
                DOUYIN_WORK_ARTICLE_METRICS,
            ));
        }
        Some("video") => {
            metrics.extend(douyin_metrics_from_specs(
                detail_metrics,
                None,
                DOUYIN_WORK_VIDEO_METRICS,
            ));
        }
        _ => {
            metrics.push(douyin_count_value_metric("share", "分享", shares));
            metrics.extend(douyin_metrics_from_specs(
                detail_metrics,
                None,
                DOUYIN_WORK_FALLBACK_METRICS,
            ));
        }
    }

    metrics
}

fn douyin_count_value_metric(key: &str, label: &str, value: Option<u64>) -> ChannelWorkMetric {
    douyin_work_metric(key, label, value.map(|value| value.to_string()))
}

fn douyin_work_type(value: &Value) -> Option<String> {
    if first_bool_deep(value, &["is_pic_word", "isPicWord", "is_slides", "isSlides"])
        .unwrap_or(false)
    {
        return Some("article".to_string());
    }

    if let Some(media_type) = first_i64(value, &["media_type", "mediaType"]) {
        return match media_type {
            2 => Some("article".to_string()),
            0 | 1 | 4 => Some("video".to_string()),
            _ => None,
        };
    }

    if let Some(content_type) = first_i64(
        value,
        &["content_type", "contentType", "item_type", "itemType", "type"],
    ) {
        return match content_type {
            1 | 68 => Some("article".to_string()),
            0 | 2 | 4 => Some("video".to_string()),
            _ => None,
        };
    }

    if let Some(aweme_type) = first_i64(value, &["aweme_type", "awemeType"]) {
        return match aweme_type {
            2 => Some("article".to_string()),
            0 | 4 => Some("video".to_string()),
            _ => None,
        };
    }

    if first_count(value, &["picture_count", "pictureCount"])
        .or_else(|| value.get("picture_info").and_then(|value| first_count(value, &["count"])))
        .unwrap_or(0)
        > 0
    {
        return Some("article".to_string());
    }

    if value.get("video").is_some() || value.get("video_info").is_some() {
        return Some("video".to_string());
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
        _ => None,
    }
}

fn first_time_deep(value: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(time) = map.get(*key).and_then(time_from_value) {
                    return Some(time);
                }
            }
            map.values().find_map(|value| first_time_deep(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_time_deep(value, keys)),
        _ => None,
    }
}

fn time_from_value(value: &Value) -> Option<DateTime<Utc>> {
    if let Some(seconds) = value.as_i64() {
        let seconds = if seconds > 10_000_000_000 { seconds / 1000 } else { seconds };
        return DateTime::from_timestamp(seconds, 0);
    }

    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    let text = text.strip_prefix("发布于").unwrap_or(text).trim();
    if let Ok(value) = NaiveDateTime::parse_from_str(text, "%Y年%m月%d日 %H:%M") {
        return FixedOffset::east_opt(8 * 3600)?
            .from_local_datetime(&value)
            .single()
            .map(|value| value.with_timezone(&Utc));
    }
    if let Ok(value) = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M"))
    {
        return FixedOffset::east_opt(8 * 3600)?
            .from_local_datetime(&value)
            .single()
            .map(|value| value.with_timezone(&Utc));
    }
    DateTime::parse_from_rfc3339(text)
        .map(|value| value.with_timezone(&Utc))
        .ok()
}

fn douyin_error_message(value: &Value, fallback: &str) -> String {
    first_string_deep(value, &["status_msg", "status_message", "message", "msg"])
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
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
