use super::*;
use chrono::TimeZone;

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
pub(crate) const STATISTICS_WORKS_URL: &str = "https://cp.kuaishou.com/statistics/works";
pub(crate) const STATISTICS_ARTICLE_URL: &str = "https://cp.kuaishou.com/statistics/article";
pub(crate) const ARTICLE_MANAGE_VIDEO_URL: &str = "https://cp.kuaishou.com/article/manage/video";
pub(crate) const AUTHOR_OVERVIEW_API: &str = "https://cp.kuaishou.com/rest/cp/creator/analysis/pc/author/overview";
pub(crate) const ARTICLE_PHOTO_LIST_API: &str = "https://cp.kuaishou.com/rest/cp/creator/analysis/pc/photo/list";
pub(crate) const ARTICLE_SINGLE_INFO_API: &str = "https://cp.kuaishou.com/rest/cp/creator/analysis/pc/photo/single/info";
pub(crate) const ARTICLE_SINGLE_OVERVIEW_API: &str = "https://cp.kuaishou.com/rest/cp/creator/analysis/pc/photo/single/overview";
pub(crate) const ARTICLE_MANAGE_VIDEO_LIST_API: &str = "https://cp.kuaishou.com/rest/cp/works/v2/video/pc/photo/list";
const KUAISHOU_WORKS_PAGE_SIZE: i64 = 10;
const KUAISHOU_MANAGEMENT_WORKS_PAGE_SIZE: i64 = 30;
const KUAISHOU_MANAGE_CURSOR_START: i64 = 1_893_456_000_000;
const KUAISHOU_MANAGE_TIME_RANGE_ALL: i64 = 5;
const KUAISHOU_MANAGE_ALL_DAYS: i64 = 365;
const MILLIS_PER_DAY: i64 = 86_400_000;
const HOME_INFO_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://cp.kuaishou.com"),
    ("Referer", CREATOR_HOME_URL),
];
const STATISTICS_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://cp.kuaishou.com"),
    ("Referer", STATISTICS_WORKS_URL),
];
const STATISTICS_ARTICLE_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://cp.kuaishou.com"),
    ("Referer", STATISTICS_ARTICLE_URL),
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
const UID_KEYS: &[&str] = &[
    "userKwaiId",
    "kwaiId",
    "kuaishouId",
    "userId",
    "id",
    "uid",
];
const NICKNAME_KEYS: &[&str] = &[
    "userName",
    "user_name",
    "nickname",
    "nickName",
    "name",
    "displayName",
];
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
const FOLLOWING_COUNT_KEYS: &[&str] = &[
    "followCnt",
    "followNum",
    "followCount",
    "followingCount",
    "following",
    "followings",
    "attentionCount",
];
const LIKE_COUNT_KEYS: &[&str] = &["likeCnt", "likeCount", "likes"];
const WORK_COVER_KEYS: &[&str] = &[
    "cover",
    "coverUrl",
    "cover_url",
    "publishCoverUrl",
    "thumbnail",
    "image",
    "images",
];

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

pub(super) async fn fetch_kuaishou_account_content(
    cookie_header: &str,
    login_cookie: String,
    account_id: &str,
) -> Result<ChannelAccountContent, String> {
    fetch_kuaishou_account_content_with_profile(cookie_header, login_cookie, account_id, None).await
}

pub(crate) async fn fetch_kuaishou_account_content_with_profile(
    cookie_header: &str,
    login_cookie: String,
    account_id: &str,
    profile: Option<PluginAccountInfo>,
) -> Result<ChannelAccountContent, String> {
    let now = Utc::now();
    let profile = match profile {
        Some(profile) => profile,
        None => fetch_kuaishou_creator_account_from_cookie(cookie_header, login_cookie).await?,
    };
    let overview_seven = fetch_kuaishou_overview(cookie_header, account_id, 7, 1, now).await?;
    let overview_thirty = fetch_kuaishou_overview(cookie_header, account_id, 30, 2, now).await?;
    let overview_ninety = fetch_kuaishou_overview(cookie_header, account_id, 90, 3, now).await?;
    let latest_work = fetch_kuaishou_latest_work(cookie_header, account_id)
        .await
        .unwrap_or(None);

    Ok(ChannelAccountContent {
        account_id: account_id.to_string(),
        platform_id: "kuaishou".to_string(),
        profile: Some(ChannelAccountProfileSnapshot {
            account_id: account_id.to_string(),
            platform_id: "kuaishou".to_string(),
            followers: profile.fans_count,
            following: profile.following_count,
            likes: profile.like_count,
            last_sync_at: Some(now),
            updated_at: Some(now),
            sync_status: "synced".to_string(),
            error: None,
        }),
        overview_seven: Some(overview_seven),
        overview_thirty: Some(overview_thirty),
        overview_ninety: Some(overview_ninety),
        latest_work: latest_work.clone(),
        latest_work_seven: latest_work.clone(),
        latest_work_thirty: latest_work,
        sync_status: "synced".to_string(),
        ..Default::default()
    })
}

pub(super) async fn fetch_kuaishou_works_page(
    cookie_header: &str,
    account_id: &str,
    page_key: &str,
) -> Result<ChannelWorksPage, String> {
    let page = kuaishou_page_number(page_key);
    let value = request_plugin_json_with_body(
        "POST",
        ARTICLE_PHOTO_LIST_API,
        cookie_header,
        STATISTICS_ARTICLE_HEADERS,
        Some(kuaishou_statistics_works_body(page, KUAISHOU_WORKS_PAGE_SIZE)),
    )
    .await
    .map_err(|error| format!("快手作品列表接口不可用: {error}"))?;
    parse_kuaishou_statistics_works_page(value, account_id, page_key, page)
}

pub(crate) fn kuaishou_statistics_works_body(page: i64, count: i64) -> Value {
    serde_json::json!({
        "page": page,
        "count": count,
        "orderType": 2,
        "sortType": 1,
        "type": 0,
    })
}

pub(crate) fn kuaishou_management_works_body(page_key: &str) -> Value {
    let cursor = page_key
        .trim()
        .parse::<i64>()
        .map(Value::from)
        .unwrap_or_else(|_| {
            page_key
                .trim()
                .is_empty()
                .then(|| Value::from(KUAISHOU_MANAGE_CURSOR_START))
                .unwrap_or_else(|| Value::from(page_key.trim().to_string()))
        });
    let start_time = kuaishou_management_start_time_ms();
    serde_json::json!({
        "queryType": "0",
        "cursor": cursor,
        "startTime": start_time,
        "endTime": KUAISHOU_MANAGE_CURSOR_START,
        "limit": KUAISHOU_MANAGEMENT_WORKS_PAGE_SIZE,
        "timeRangeType": KUAISHOU_MANAGE_TIME_RANGE_ALL,
        "keyword": "",
    })
}

fn kuaishou_management_start_time_ms() -> i64 {
    let Some(tomorrow) = chrono::Local::now()
        .date_naive()
        .checked_add_signed(chrono::Duration::days(1))
    else {
        return 0;
    };
    let Some(midnight) = tomorrow.and_hms_opt(0, 0, 0) else {
        return 0;
    };
    chrono::Local
        .from_local_datetime(&midnight)
        .single()
        .or_else(|| chrono::Local.from_local_datetime(&midnight).earliest())
        .map(|value| value.timestamp_millis() - KUAISHOU_MANAGE_ALL_DAYS * MILLIS_PER_DAY)
        .unwrap_or(0)
}

pub(crate) fn parse_kuaishou_management_works_page(
    value: Value,
    account_id: &str,
    page_key: &str,
) -> Result<ChannelWorksPage, String> {
    let data = kuaishou_management_works_data(&value);
    if data.is_none() && !kuaishou_response_success(&value) {
        eprintln!(
            "[kuaishou:works] management response missing list: {}",
            kuaishou_response_shape(&value)
        );
        return Err(kuaishou_error_message(&value, "快手作品管理列表读取失败"));
    }

    let data = data.unwrap_or_else(|| value.get("data").unwrap_or(&value));
    let works = data
        .get("list")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parse_kuaishou_work(item, account_id))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let next_cursor = data
        .get("nextCursor")
        .and_then(|value| match value {
            Value::String(value) => Some(value.trim().to_string()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.is_empty() && value != "no_more");
    let has_more = next_cursor.is_some();

    Ok(ChannelWorksPage {
        account_id: account_id.to_string(),
        platform_id: "kuaishou".to_string(),
        page_key: page_key.trim().to_string(),
        work_type: Some("video".to_string()),
        next_page_key: next_cursor,
        has_more,
        works,
        updated_at: Some(Utc::now()),
        sync_status: "synced".to_string(),
        error: None,
    })
}

fn kuaishou_management_works_data(value: &Value) -> Option<&Value> {
    if value.get("list").and_then(Value::as_array).is_some() {
        return Some(value);
    }
    if let Some(data) = value
        .get("data")
        .filter(|data| data.get("list").and_then(Value::as_array).is_some())
    {
        return Some(data);
    }
    value
        .get("data")
        .and_then(|data| data.get("data"))
        .filter(|data| data.get("list").and_then(Value::as_array).is_some())
        .or_else(|| {
            value
                .get("result")
                .filter(|data| data.get("list").and_then(Value::as_array).is_some())
        })
}

fn parse_kuaishou_statistics_works_page(
    value: Value,
    account_id: &str,
    page_key: &str,
    page: i64,
) -> Result<ChannelWorksPage, String> {
    if !kuaishou_response_success(&value) {
        return Err(kuaishou_error_message(&value, "快手作品列表读取失败"));
    }

    let photo_list = value
        .get("data")
        .and_then(|data| data.get("photoList"))
        .or_else(|| value.get("photoList"));
    let works = photo_list
        .and_then(|data| data.get("photoItems"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parse_kuaishou_work(item, account_id))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let total_count = photo_list.and_then(|data| number_value(data.get("totalCount")?));
    let next_page = page.saturating_add(1);
    let loaded_count = next_page.saturating_mul(KUAISHOU_WORKS_PAGE_SIZE);
    let has_more = total_count
        .map(|total| (loaded_count as f64) < total)
        .unwrap_or_else(|| works.len() >= KUAISHOU_WORKS_PAGE_SIZE as usize);

    Ok(ChannelWorksPage {
        account_id: account_id.to_string(),
        platform_id: "kuaishou".to_string(),
        page_key: page_key.trim().to_string(),
        work_type: None,
        next_page_key: has_more.then(|| next_page.to_string()),
        has_more,
        works,
        updated_at: Some(Utc::now()),
        sync_status: "synced".to_string(),
        error: None,
    })
}

async fn fetch_kuaishou_latest_work(
    cookie_header: &str,
    account_id: &str,
) -> Result<Option<ChannelContentWork>, String> {
    let value = request_plugin_json_with_body(
        "POST",
        ARTICLE_PHOTO_LIST_API,
        cookie_header,
        STATISTICS_ARTICLE_HEADERS,
        Some(kuaishou_statistics_works_body(0, 1)),
    )
    .await?;
    if !kuaishou_response_success(&value) {
        return Err(kuaishou_error_message(&value, "快手最新作品读取失败"));
    }
    let photo_list = value
        .get("data")
        .and_then(|data| data.get("photoList"))
        .or_else(|| value.get("photoList"));
    let mut work = photo_list
        .and_then(|data| data.get("photoItems"))
        .and_then(Value::as_array)
        .and_then(|items| items.iter().find_map(|item| parse_kuaishou_work(item, account_id)));
    if let Some(work) = work.as_mut() {
        let _ = enrich_kuaishou_latest_work(cookie_header, work).await;
    }
    Ok(work)
}

async fn enrich_kuaishou_latest_work(
    cookie_header: &str,
    work: &mut ChannelContentWork,
) -> Result<(), String> {
    let referer = work
        .link
        .clone()
        .unwrap_or_else(|| STATISTICS_ARTICLE_URL.to_string());
    let detail_headers = [
        ("Origin", "https://cp.kuaishou.com"),
        ("Referer", referer.as_str()),
    ];
    let info = request_plugin_json_with_body(
        "POST",
        ARTICLE_SINGLE_INFO_API,
        cookie_header,
        &detail_headers,
        Some(kuaishou_single_info_body(&work.id)),
    )
    .await?;
    if kuaishou_response_success(&info) {
        apply_kuaishou_single_info(work, &info);
    }

    let play = request_plugin_json_with_body(
        "POST",
        ARTICLE_SINGLE_OVERVIEW_API,
        cookie_header,
        &detail_headers,
        Some(kuaishou_single_overview_body(&work.id, 1)),
    )
    .await?;
    let interact = request_plugin_json_with_body(
        "POST",
        ARTICLE_SINGLE_OVERVIEW_API,
        cookie_header,
        &detail_headers,
        Some(kuaishou_single_overview_body(&work.id, 2)),
    )
    .await?;
    apply_kuaishou_single_overview(work, &[play, interact]);
    Ok(())
}

pub(crate) fn kuaishou_single_info_body(photo_id: &str) -> Value {
    serde_json::json!({ "photoId": photo_id, "workId": photo_id })
}

pub(crate) fn kuaishou_single_overview_body(photo_id: &str, tab_type: i64) -> Value {
    serde_json::json!({
        "photoId": photo_id,
        "tabType": tab_type,
        "dataChangeType": 2,
        "timeGranularity": 2,
    })
}

async fn fetch_kuaishou_overview(
    cookie_header: &str,
    account_id: &str,
    period_days: u16,
    time_type: i64,
    now: DateTime<Utc>,
) -> Result<ChannelAccountOverview, String> {
    let value = request_plugin_json_with_body(
        "POST",
        AUTHOR_OVERVIEW_API,
        cookie_header,
        STATISTICS_HEADERS,
        Some(serde_json::json!({ "timeType": time_type })),
    )
    .await
    .map_err(|error| format!("快手总览接口不可用: {error}"))?;
    parse_kuaishou_overview_response(value, account_id, period_days, now)
}

pub(crate) fn parse_kuaishou_overview_response(
    value: Value,
    account_id: &str,
    period_days: u16,
    now: DateTime<Utc>,
) -> Result<ChannelAccountOverview, String> {
    if !kuaishou_response_success(&value) {
        return Err(kuaishou_error_message(&value, "快手总览读取失败"));
    }

    let data = value.get("data").unwrap_or(&value);
    Ok(ChannelAccountOverview {
        account_id: account_id.to_string(),
        platform_id: "kuaishou".to_string(),
        period_days,
        metrics: kuaishou_overview_metrics(data),
        summary: kuaishou_overview_summary(data),
        updated_at: Some(now),
        sync_status: "synced".to_string(),
        error: None,
    })
}

fn kuaishou_overview_metrics(data: &Value) -> Vec<ChannelOverviewMetric> {
    data
        .get("basicData")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(kuaishou_overview_metric).collect())
        .unwrap_or_default()
}

fn kuaishou_overview_metric(item: &Value) -> ChannelOverviewMetric {
    let label = item
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("数据")
        .to_string();
    let tab = item.get("tab").and_then(Value::as_str).unwrap_or_default();
    let key = kuaishou_overview_key(tab, &label);
    let is_rate = key == "completionRate" || label.contains('率');
    let value = item
        .get("sumCount")
        .and_then(number_value)
        .map(|value| format_kuaishou_metric_value(value, is_rate));
    let yesterday = item.get("endDayCount").and_then(number_value);

    ChannelOverviewMetric {
        key: key.to_string(),
        label,
        value,
        compare_label: Some("昨日".to_string()),
        trend: yesterday.map(|value| {
            if is_rate {
                format!("昨日 {}", format_kuaishou_metric_value(value, true))
            } else {
                format!("昨日 {}", format_signed_kuaishou_count(value))
            }
        }),
        tone: yesterday.map(delta_tone_f64),
    }
}

fn kuaishou_overview_key(tab: &str, label: &str) -> &'static str {
    match tab {
        "PLAY" => "play",
        "LIKE" => "like",
        "PURE_INCREASE_FAN" => "netFollowers",
        "COMPLETE_RATIO" => "completionRate",
        "COMMENT" => "comment",
        "SHARE" => "share",
        "WORKS" => "works",
        _ if label.contains("粉丝") => "netFollowers",
        _ if label.contains("完播") => "completionRate",
        _ if label.contains("评论") => "comment",
        _ if label.contains("点赞") => "like",
        _ if label.contains("分享") => "share",
        _ if label.contains("作品") => "works",
        _ => "metric",
    }
}

fn kuaishou_overview_summary(data: &Value) -> Option<String> {
    data
        .get("dataUpdateTime")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("数据更新至 {}", format_kuaishou_data_date(value)))
}

fn parse_kuaishou_work(item: &Value, account_id: &str) -> Option<ChannelContentWork> {
    let photo_id = first_string(item, &["photoId", "workId", "publishId", "id"])?;
    let title = first_string(item, &["title", "caption", "desc"])
        .unwrap_or_else(|| "快手作品".to_string());
    let cover_url = first_profile_image(item, WORK_COVER_KEYS)
        .map(|value| normalize_platform_image_url("kuaishou", value));
    let views = first_count(item, &["playCount", "play_count", "viewCount", "views"]);
    let likes = first_count(item, &["likeCount", "like_count", "likes"]);
    let comments = first_count(item, &["commentCount", "comment_count", "comments"]);
    let collects = first_count(item, &["collectCount", "collect_count", "collects"]);
    let gained_followers = first_signed_count(item, &["followCount", "follow_count", "fansCount"]);
    let completion_rate = item
        .get("fpr")
        .or_else(|| item.get("finishRate"))
        .and_then(number_value)
        .map(|value| format_kuaishou_metric_value(value, true));
    let work_type = kuaishou_work_type(item);
    let badges = kuaishou_work_badges(item);

    Some(ChannelContentWork {
        id: photo_id.clone(),
        platform_id: "kuaishou".to_string(),
        account_id: account_id.to_string(),
        title,
        cover_url,
        link: Some(format!("https://cp.kuaishou.com/statistics/article/detail/{photo_id}")),
        published_at: item
            .get("publishTime")
            .or_else(|| item.get("uploadTime"))
            .and_then(kuaishou_datetime),
        status: kuaishou_work_status(&badges).to_string(),
        views,
        impressions: None,
        likes,
        collects,
        comments,
        shares: first_count(item, &["shareCount", "share_count", "shares"]),
        cover_click_rate: None,
        avg_view_time: None,
        gained_followers,
        data_updated_at: Some(Utc::now()),
        metrics: kuaishou_work_metrics(
            views,
            likes,
            comments,
            completion_rate.clone(),
            gained_followers,
            collects,
        ),
        badges,
        work_type,
    })
}

fn kuaishou_work_metrics(
    views: Option<u64>,
    likes: Option<u64>,
    comments: Option<u64>,
    completion_rate: Option<String>,
    gained_followers: Option<i64>,
    collects: Option<u64>,
) -> Vec<ChannelWorkMetric> {
    vec![
        kuaishou_work_metric("play", "播放", views.map(|value| value.to_string())),
        kuaishou_work_metric("like", "点赞", likes.map(|value| value.to_string())),
        kuaishou_work_metric("comment", "评论", comments.map(|value| value.to_string())),
        kuaishou_work_metric("completionRate", "完播率", completion_rate),
        kuaishou_work_metric("followers", "涨粉", gained_followers.map(|value| value.to_string())),
        kuaishou_work_metric("collect", "收藏", collects.map(|value| value.to_string())),
    ]
}

fn kuaishou_work_metric(key: &str, label: &str, value: Option<String>) -> ChannelWorkMetric {
    ChannelWorkMetric {
        key: key.to_string(),
        label: label.to_string(),
        value,
    }
}

fn kuaishou_work_type(item: &Value) -> Option<String> {
    if item.get("showAtlasIcon").and_then(kuaishou_value_to_bool) == Some(true) {
        return Some("article".to_string());
    }
    if item.get("video").and_then(kuaishou_value_to_bool) == Some(false) {
        return Some("article".to_string());
    }
    if item.get("video").and_then(kuaishou_value_to_bool) == Some(true) {
        return Some("video".to_string());
    }
    first_i64(item, &["photoType", "workType", "type"]).and_then(|value| match value {
        2 => Some("article".to_string()),
        1 => Some("video".to_string()),
        _ => None,
    })
}

fn kuaishou_work_badges(item: &Value) -> Vec<String> {
    let mut badges = item
        .get("photoStatusTags")
        .and_then(Value::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.get("tagText").and_then(Value::as_str))
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for key in ["promotionDesc", "negativeDesc", "bonusDesc"] {
        if let Some(value) = item.get(key).and_then(Value::as_str).filter(|value| !value.trim().is_empty()) {
            badges.push(value.to_string());
        }
    }
    if item.get("photoTop").and_then(kuaishou_value_to_bool) == Some(true) {
        badges.push("置顶".to_string());
    }
    if let Some(value) = first_i64(item, &["photoStatus"]) {
        match value {
            1 => badges.push("私密".to_string()),
            3 => badges.push("仅好友可见".to_string()),
            _ => {}
        }
    }
    if let Some(value) = first_i64(item, &["publishStatus"]) {
        match value {
            3 | 6 | 9 => badges.push("发布失败".to_string()),
            10 => badges.push("审核中".to_string()),
            11 => badges.push("审核未通过".to_string()),
            15 => badges.push("上传权限封禁".to_string()),
            _ => {}
        }
    }
    badges.sort();
    badges.dedup();
    badges
}

fn kuaishou_work_status(badges: &[String]) -> &'static str {
    if badges.iter().any(|badge| badge.contains("失败") || badge.contains("未通过") || badge.contains("封禁")) {
        "failed"
    } else if badges.iter().any(|badge| badge.contains("审核")) {
        "reviewing"
    } else if badges.iter().any(|badge| badge.contains("草稿")) {
        "draft"
    } else {
        "published"
    }
}

fn apply_kuaishou_single_info(work: &mut ChannelContentWork, value: &Value) {
    let data = value.get("data").unwrap_or(value);
    if let Some(title) = first_string(data, &["title", "caption", "desc"]).filter(|value| !value.trim().is_empty()) {
        work.title = title;
    }
    if let Some(cover) = first_profile_image(data, WORK_COVER_KEYS) {
        work.cover_url = Some(normalize_platform_image_url("kuaishou", cover));
    }
    if let Some(published_at) = data.get("publishTime").and_then(kuaishou_datetime) {
        work.published_at = Some(published_at);
    }
    if let Some(work_type) = kuaishou_work_type(data) {
        work.work_type = Some(work_type);
    }
    let mut badges = work.badges.clone();
    badges.extend(kuaishou_work_badges(data));
    badges.sort();
    badges.dedup();
    work.status = kuaishou_work_status(&badges).to_string();
    work.badges = badges;
}

fn apply_kuaishou_single_overview(work: &mut ChannelContentWork, values: &[Value]) {
    let mut metrics = Vec::new();
    for item in values
        .iter()
        .filter(|value| kuaishou_response_success(value))
        .filter_map(|value| value.get("data").or_else(|| Some(value)))
        .filter_map(|data| data.get("trendList"))
        .filter_map(Value::as_array)
        .flatten()
    {
        let Some(name) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        let en_name = item.get("enName").and_then(Value::as_str).unwrap_or_default();
        let metric_type = first_i64(item, &["type"]).unwrap_or(1);
        let value = item
            .get("sumCount")
            .and_then(number_value)
            .map(|value| format_kuaishou_detail_metric_value(value, metric_type));
        apply_kuaishou_detail_metric_to_work(work, en_name, item.get("sumCount").and_then(number_value));
        metrics.push(ChannelWorkMetric {
            key: kuaishou_detail_metric_key(en_name, name).to_string(),
            label: name.to_string(),
            value,
        });
    }
    if !metrics.is_empty() {
        metrics.sort_by_key(|metric| kuaishou_detail_metric_order(&metric.key));
        metrics.dedup_by(|left, right| left.key == right.key);
        work.metrics = metrics;
    }
}

fn apply_kuaishou_detail_metric_to_work(work: &mut ChannelContentWork, en_name: &str, value: Option<f64>) {
    let Some(value) = value else {
        return;
    };
    match en_name {
        "PLAY_CNT" => work.views = Some(value.round().max(0.0) as u64),
        "LIKE_CNT" => work.likes = Some(value.round().max(0.0) as u64),
        "COMMENT_CNT" => work.comments = Some(value.round().max(0.0) as u64),
        "SHARE_CNT" => work.shares = Some(value.round().max(0.0) as u64),
        "COLLECT_CNT" => work.collects = Some(value.round().max(0.0) as u64),
        "FOLLOW_CNT" | "FAN_CNT" | "PURE_INCREASE_FAN" => work.gained_followers = Some(value.round() as i64),
        "OUTSIDE_CTR" => work.cover_click_rate = Some(format_kuaishou_detail_metric_value(value, 2)),
        "AVG_PLAY_DURATION" => work.avg_view_time = Some(format_kuaishou_detail_metric_value(value, 4)),
        _ => {}
    }
}

fn kuaishou_detail_metric_key(en_name: &str, name: &str) -> &'static str {
    match en_name {
        "PLAY_CNT" => "play",
        "AVG_PLAY_DURATION" => "avgViewTime",
        "OUTSIDE_CTR" => "coverClickRate",
        "TWO_SECONDS_EXIT" => "twoSecondExitRate",
        "FIVE_SECONDS_FPR" => "fiveSecondCompletionRate",
        "FPR" => "completionRate",
        "LIKE_CNT" => "like",
        "COMMENT_CNT" => "comment",
        "SHARE_CNT" => "share",
        "COLLECT_CNT" => "collect",
        "FOLLOW_CNT" | "FAN_CNT" | "PURE_INCREASE_FAN" => "followers",
        _ if name.contains("播放") => "play",
        _ if name.contains("点赞") => "like",
        _ if name.contains("评论") => "comment",
        _ if name.contains("分享") => "share",
        _ if name.contains("收藏") => "collect",
        _ if name.contains("粉") => "followers",
        _ => "metric",
    }
}

fn kuaishou_detail_metric_order(key: &str) -> u8 {
    match key {
        "play" => 0,
        "avgViewTime" => 1,
        "coverClickRate" => 2,
        "completionRate" => 3,
        "fiveSecondCompletionRate" => 4,
        "twoSecondExitRate" => 5,
        "like" => 6,
        "comment" => 7,
        "collect" => 8,
        "share" => 9,
        "followers" => 10,
        _ => 99,
    }
}

fn format_kuaishou_detail_metric_value(value: f64, metric_type: i64) -> String {
    match metric_type {
        2 => {
            let text = format!("{value:.2}");
            format!("{}%", trim_number_text(&text))
        }
        4 => {
            let text = format!("{:.1}", value / 1000.0);
            format!("{}秒", trim_number_text(&text))
        }
        _ => format_kuaishou_count(value),
    }
}

async fn parse_kuaishou_creator_account(
    value: Value,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let payload = value.get("data").filter(|data| !data.is_null()).unwrap_or(&value);
    let user_info = value.get("userInfo").filter(|data| !data.is_null());
    let result = first_i64(&value, RESPONSE_CODE_KEYS).unwrap_or(1);
    let uid = user_info
        .and_then(|data| first_string_deep(data, UID_KEYS))
        .or_else(|| first_string_deep(payload, UID_KEYS))
        .or_else(|| {
            first_count(payload, UID_KEYS)
                .filter(|value| *value > 0)
                .map(|value| value.to_string())
        })
        .unwrap_or_default();
    let nickname = user_info
        .and_then(|data| first_string_deep(data, NICKNAME_KEYS))
        .or_else(|| first_string_deep(payload, NICKNAME_KEYS))
        .unwrap_or_else(|| platform_name("kuaishou").to_string());
    let has_profile = !uid.trim().is_empty() || nickname != platform_name("kuaishou");
    let top_keys = value
        .as_object()
        .map(|object| object.keys().take(8).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    eprintln!(
        "[plugin-auth:kuaishou] result={result} has_profile={has_profile} keys={top_keys:?}"
    );
    if !has_profile {
        if let Some(account) = kuaishou_account_from_login_cookie(&login_cookie) {
            eprintln!("[plugin-auth:kuaishou] using login-cookie fallback account={}", account.uid);
            return Ok(account);
        }
        return Err("请先在打开的快手创作者中心完成登录。".to_string());
    }
    let avatar = user_info
        .and_then(|data| first_profile_image(data, AVATAR_KEYS))
        .or_else(|| first_profile_image(payload, AVATAR_KEYS))
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
        following_count: first_count(payload, FOLLOWING_COUNT_KEYS),
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
        following_count: None,
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

fn kuaishou_response_success(value: &Value) -> bool {
    first_i64(value, &["result", "code", "errCode", "errcode"]).unwrap_or(0) == 1
}

fn kuaishou_error_message(value: &Value, fallback: &str) -> String {
    first_string_deep(
        value,
        &[
            "message",
            "msg",
            "errorMessage",
            "error_msg",
            "errorMsg",
            "reason",
            "tips",
        ],
    )
    .filter(|value| !value.trim().is_empty())
    .unwrap_or_else(|| {
        first_i64(value, RESPONSE_CODE_KEYS)
            .map(|code| format!("{fallback}: {code}"))
            .unwrap_or_else(|| fallback.to_string())
    })
}

fn kuaishou_response_shape(value: &Value) -> String {
    let keys = value
        .as_object()
        .map(|object| object.keys().take(10).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let result = first_i64(value, RESPONSE_CODE_KEYS)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let message = first_string_deep(value, &["message", "msg", "errorMessage", "errorMsg"])
        .unwrap_or_else(|| "-".to_string());
    format!("result={result} keys={keys:?} message={message}")
}

fn kuaishou_page_number(page_key: &str) -> i64 {
    page_key
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value >= 0)
        .unwrap_or(0)
}

fn kuaishou_datetime(value: &Value) -> Option<DateTime<Utc>> {
    let raw = number_value(value)?;
    let timestamp = raw.round() as i64;
    let millis = if timestamp > 10_000_000_000 {
        timestamp
    } else {
        timestamp.saturating_mul(1000)
    };
    Utc.timestamp_millis_opt(millis).single()
}

fn number_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|value| value.is_finite())
}

fn first_signed_count(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(number_value))
        .map(|value| value.round() as i64)
}

fn format_kuaishou_metric_value(value: f64, is_rate: bool) -> String {
    if is_rate {
        return format_kuaishou_percent(value);
    }
    format_kuaishou_count(value)
}

fn format_kuaishou_percent(value: f64) -> String {
    let percent = value * 100.0;
    let text = format!("{percent:.2}");
    format!("{}%", trim_number_text(&text))
}

fn format_kuaishou_count(value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let absolute = value.abs();
    if absolute >= 10_000.0 {
        let text = format!("{:.1}", absolute / 10_000.0);
        format!("{sign}{}万", trim_number_text(&text))
    } else if (absolute.fract()).abs() < f64::EPSILON {
        format!("{sign}{}", absolute.round() as i64)
    } else {
        let text = format!("{absolute:.2}");
        format!("{sign}{}", trim_number_text(&text))
    }
}

fn format_signed_kuaishou_count(value: f64) -> String {
    if value > 0.0 {
        format!("+{}", format_kuaishou_count(value))
    } else {
        format_kuaishou_count(value)
    }
}

fn trim_number_text(value: &str) -> String {
    value
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn delta_tone_f64(value: f64) -> String {
    if value > 0.0 {
        "up".to_string()
    } else if value < 0.0 {
        "down".to_string()
    } else {
        "neutral".to_string()
    }
}

fn kuaishou_value_to_bool(value: &Value) -> Option<bool> {
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

fn format_kuaishou_data_date(value: &str) -> String {
    let value = value.trim();
    if value.len() == 8 {
        format!("{}-{}-{}", &value[0..4], &value[4..6], &value[6..8])
    } else {
        value.to_string()
    }
}
