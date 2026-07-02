use super::*;
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};

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
const UPSTAT_API_PREFIX: &str = "https://api.bilibili.com/x/space/upstat?mid=";
const DATA_CENTER_OVERVIEW_API: &str = "https://member.bilibili.com/x/web/data/v2/overview/stat/num";
const VIDEO_WORKS_API: &str = "https://member.bilibili.com/x/web/archives";
const ARTICLE_WORKS_API: &str = "https://api.bilibili.com/x/polymer/web-dynamic/v1/opus/creationlist";
const BILI_WORKS_PAGE_SIZE: i64 = 10;
const BILI_PERIOD_HISTORY: u16 = 36500;
const BILI_PERIOD_TOTAL: u16 = 65535;
const BILI_DATA_CENTER_HISTORY_PERIOD: i8 = 3;
const API_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://www.bilibili.com"),
    ("Referer", "https://www.bilibili.com/"),
];
const DATA_CENTER_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://member.bilibili.com"),
    (
        "Referer",
        "https://member.bilibili.com/platform/data-up/video/",
    ),
];
const VIDEO_WORKS_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://member.bilibili.com"),
    (
        "Referer",
        "https://member.bilibili.com/platform/upload-manager/article",
    ),
];
const ARTICLE_WORKS_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://member.bilibili.com"),
    ("Referer", "https://member.bilibili.com/opus/management/"),
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
const FOLLOWING_COUNT_KEYS: &[&str] = &[
    "following",
    "following_count",
    "followingCount",
    "follow_count",
    "followCount",
    "followings",
    "attention_count",
    "attentionCount",
];
const LIKE_COUNT_KEYS: &[&str] = &[
    "likes",
    "like_count",
    "likeCount",
    "liked_count",
    "likedCount",
];
const WORK_COVER_KEYS: &[&str] = &[
    "cover",
    "pic",
    "cover_url",
    "coverUrl",
    "image",
    "images",
    "thumbnail",
    "thumbnail_url",
    "thumbnailUrl",
];

#[derive(Clone, Copy)]
struct BiliOverviewMetricSpec {
    key: &'static str,
    label: &'static str,
    cumulative_label: &'static str,
    data_key: &'static str,
}

const BILI_OVERVIEW_METRICS: &[BiliOverviewMetricSpec] = &[
    BiliOverviewMetricSpec {
        key: "views",
        label: "播放量",
        cumulative_label: "播放量",
        data_key: "play",
    },
    BiliOverviewMetricSpec {
        key: "followers",
        label: "净增粉丝",
        cumulative_label: "累计粉丝",
        data_key: "fan",
    },
    BiliOverviewMetricSpec {
        key: "likes",
        label: "点赞",
        cumulative_label: "点赞",
        data_key: "like",
    },
    BiliOverviewMetricSpec {
        key: "collects",
        label: "收藏",
        cumulative_label: "收藏",
        data_key: "fav",
    },
    BiliOverviewMetricSpec {
        key: "coins",
        label: "硬币",
        cumulative_label: "硬币",
        data_key: "coin",
    },
    BiliOverviewMetricSpec {
        key: "comments",
        label: "评论",
        cumulative_label: "评论",
        data_key: "comment",
    },
    BiliOverviewMetricSpec {
        key: "danmaku",
        label: "弹幕",
        cumulative_label: "弹幕",
        data_key: "dm",
    },
    BiliOverviewMetricSpec {
        key: "shares",
        label: "分享",
        cumulative_label: "分享",
        data_key: "share",
    },
];

const BILI_OVERVIEW_TAB_KEYS: &[&[&str]] = &[
    &["play", "fan"],
    &["like", "fav", "coin"],
    &["comment", "dm", "share"],
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum BiliWorkKind {
    Video,
    Article,
}

impl BiliWorkKind {
    fn from_option(value: Option<&str>) -> Self {
        match value.unwrap_or_default().trim() {
            "article" | "image" | "photo" | "opus" => BiliWorkKind::Article,
            _ => BiliWorkKind::Video,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            BiliWorkKind::Video => "video",
            BiliWorkKind::Article => "article",
        }
    }
}

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
    let mut following_count = data.and_then(|data| first_count(data, FOLLOWING_COUNT_KEYS));
    let mut like_count = data.and_then(|data| first_count(data, LIKE_COUNT_KEYS));
    if fans_count.is_none() || following_count.is_none() {
        if let Some((relation_fans_count, relation_following_count)) =
            fetch_bilibili_relation_counts(cookie_header, &uid).await
        {
            if fans_count.is_none() {
                fans_count = relation_fans_count;
            }
            if following_count.is_none() {
                following_count = relation_following_count;
            }
        }
    }
    if like_count.is_none() {
        like_count = fetch_bilibili_like_count(cookie_header, &uid).await;
    }
    Ok(PluginAccountInfo {
        uid: account.clone(),
        account,
        nickname,
        avatar,
        fans_count,
        following_count,
        like_count,
        login_cookie,
    })
}

async fn fetch_bilibili_relation_counts(cookie_header: &str, uid: &str) -> Option<(Option<u64>, Option<u64>)> {
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
    let data = value.get("data").or(Some(&value));
    Some((
        data.and_then(|data| first_count(data, FOLLOWER_COUNT_KEYS)),
        data.and_then(|data| first_count(data, FOLLOWING_COUNT_KEYS)),
    ))
}

async fn fetch_bilibili_like_count(cookie_header: &str, uid: &str) -> Option<u64> {
    let uid = uid.trim();
    if uid.is_empty() || !uid.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let value = request_plugin_json(
        "GET",
        &format!("{UPSTAT_API_PREFIX}{uid}"),
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
        .or(Some(&value))
        .and_then(|data| first_count(data, LIKE_COUNT_KEYS))
}

pub(super) async fn fetch_bilibili_account_content(
    cookie_header: &str,
    login_cookie: String,
    account_id: &str,
) -> Result<ChannelAccountContent, String> {
    let now = Utc::now();
    let profile = probe_bilibili_creator_session(cookie_header, login_cookie).await?;
    let overview_yesterday_data =
        fetch_bilibili_data_center_overview(cookie_header, &profile.uid, -1).await?;
    let overview_seven_data =
        fetch_bilibili_data_center_overview(cookie_header, &profile.uid, 0).await?;
    let overview_thirty_data =
        fetch_bilibili_data_center_overview(cookie_header, &profile.uid, 1).await?;
    let overview_ninety_data =
        fetch_bilibili_data_center_overview(cookie_header, &profile.uid, 2).await?;
    let overview_total_data = fetch_bilibili_data_center_overview(
        cookie_header,
        &profile.uid,
        BILI_DATA_CENTER_HISTORY_PERIOD,
    )
    .await?;
    let latest_video_work = fetch_bilibili_latest_work(cookie_header, account_id, BiliWorkKind::Video)
        .await
        .unwrap_or(None);
    let latest_article_work = fetch_bilibili_latest_work(cookie_header, account_id, BiliWorkKind::Article)
        .await
        .unwrap_or(None);

    Ok(ChannelAccountContent {
        account_id: account_id.to_string(),
        platform_id: "bilibili".to_string(),
        profile: Some(ChannelAccountProfileSnapshot {
            account_id: account_id.to_string(),
            platform_id: "bilibili".to_string(),
            followers: count_key(&overview_total_data, "fan").or(profile.fans_count),
            following: profile.following_count,
            likes: count_key(&overview_total_data, "like").or(profile.like_count),
            last_sync_at: Some(now),
            updated_at: Some(now),
            sync_status: "synced".to_string(),
            error: None,
        }),
        overview_yesterday: Some(build_bilibili_overview(
            account_id,
            1,
            &overview_yesterday_data,
            false,
            now,
        )),
        overview_seven: Some(build_bilibili_overview(
            account_id,
            7,
            &overview_seven_data,
            false,
            now,
        )),
        overview_thirty: Some(build_bilibili_overview(
            account_id,
            30,
            &overview_thirty_data,
            false,
            now,
        )),
        overview_ninety: Some(build_bilibili_overview(
            account_id,
            90,
            &overview_ninety_data,
            false,
            now,
        )),
        overview_history: Some(build_bilibili_overview(
            account_id,
            BILI_PERIOD_HISTORY,
            &overview_total_data,
            true,
            now,
        )),
        overview_total: Some(build_bilibili_overview(
            account_id,
            BILI_PERIOD_TOTAL,
            &overview_total_data,
            true,
            now,
        )),
        latest_work: latest_video_work,
        latest_work_seven: latest_article_work,
        latest_work_thirty: None,
        sync_status: "synced".to_string(),
        ..Default::default()
    })
}

pub(super) async fn fetch_bilibili_works_page(
    cookie_header: &str,
    account_id: &str,
    page_key: &str,
    work_type: Option<&str>,
) -> Result<ChannelWorksPage, String> {
    match BiliWorkKind::from_option(work_type) {
        BiliWorkKind::Article => {
            fetch_bilibili_article_works_page(cookie_header, account_id, page_key).await
        }
        BiliWorkKind::Video => {
            fetch_bilibili_video_works_page(cookie_header, account_id, page_key).await
        }
    }
}

async fn fetch_bilibili_latest_work(
    cookie_header: &str,
    account_id: &str,
    kind: BiliWorkKind,
) -> Result<Option<ChannelContentWork>, String> {
    let page = match kind {
        BiliWorkKind::Video => fetch_bilibili_video_works_page(cookie_header, account_id, "").await?,
        BiliWorkKind::Article => fetch_bilibili_article_works_page(cookie_header, account_id, "").await?,
    };
    Ok(page.works.into_iter().next())
}

async fn fetch_bilibili_data_center_overview(
    cookie_header: &str,
    uid: &str,
    period: i8,
) -> Result<Value, String> {
    let uid = uid.trim();
    if uid.is_empty() || !uid.chars().all(|ch| ch.is_ascii_digit()) {
        return Err("B 站账号缺少创作中心 UID，无法读取核心数据。".to_string());
    }
    let mut merged = serde_json::Map::new();
    for (tab, keys) in BILI_OVERVIEW_TAB_KEYS.iter().enumerate() {
        let params = vec![
            ("period", period.to_string()),
            ("tab", tab.to_string()),
            ("tmid", uid.to_string()),
            ("t", Utc::now().timestamp_millis().to_string()),
        ];
        let url = Url::parse_with_params(DATA_CENTER_OVERVIEW_API, params)
            .map_err(|error| format!("B 站数据中心地址无效: {error}"))?;
        let value = request_plugin_json("GET", url.as_str(), cookie_header, DATA_CENTER_HEADERS)
            .await
            .map_err(|error| format!("B 站数据中心核心数据接口不可用: {error}"))?;
        if !bilibili_response_success(&value) {
            return Err(bilibili_error_message(&value, "B 站数据中心核心数据读取失败"));
        }
        let data = value.get("data").unwrap_or(&value);
        merge_bilibili_overview_tab(&mut merged, data, keys);
    }
    Ok(Value::Object(merged))
}

fn merge_bilibili_overview_tab(
    merged: &mut serde_json::Map<String, Value>,
    data: &Value,
    keys: &[&str],
) {
    if let Some(log_date) = data.get("log_date") {
        merged.insert("log_date".to_string(), log_date.clone());
    }
    for key in keys {
        if let Some(value) = data.get(*key) {
            merged.insert((*key).to_string(), value.clone());
        }
        let last_key = format!("{key}_last");
        if let Some(value) = data.get(last_key.as_str()) {
            merged.insert(last_key, value.clone());
        }
    }
}

fn build_bilibili_overview(
    account_id: &str,
    period_days: u16,
    data: &Value,
    cumulative: bool,
    now: DateTime<Utc>,
) -> ChannelAccountOverview {
    let metrics = BILI_OVERVIEW_METRICS
        .iter()
        .map(|spec| build_bilibili_metric(spec, data, cumulative))
        .collect();
    ChannelAccountOverview {
        account_id: account_id.to_string(),
        platform_id: "bilibili".to_string(),
        period_days,
        metrics,
        summary: bilibili_overview_summary(data),
        updated_at: Some(now),
        sync_status: "synced".to_string(),
        error: None,
    }
}

fn build_bilibili_metric(
    spec: &BiliOverviewMetricSpec,
    data: &Value,
    cumulative: bool,
) -> ChannelOverviewMetric {
    let value = signed_key(data, spec.data_key);
    let last_value = signed_key(data, &format!("{}_last", spec.data_key));
    let delta = value.zip(last_value).map(|(value, last_value)| value - last_value);
    ChannelOverviewMetric {
        key: spec.key.to_string(),
        label: if cumulative {
            spec.cumulative_label
        } else {
            spec.label
        }
        .to_string(),
        value: value.map(|value| value.to_string()),
        compare_label: None,
        trend: delta.and_then(format_bilibili_delta),
        tone: delta.map(delta_tone),
    }
}

fn bilibili_overview_summary(stat: &Value) -> Option<String> {
    let log_date = count_key(stat, "log_date")?;
    let text = log_date.to_string();
    if text.len() != 8 {
        return None;
    }
    Some(format!("更新至 {}-{}-{}", &text[0..4], &text[4..6], &text[6..8]))
}

async fn fetch_bilibili_video_works_page(
    cookie_header: &str,
    account_id: &str,
    page_key: &str,
) -> Result<ChannelWorksPage, String> {
    let page = bilibili_page_number(page_key);
    let params = vec![
        ("status", "pubed".to_string()),
        ("pn", page.to_string()),
        ("ps", BILI_WORKS_PAGE_SIZE.to_string()),
    ];
    let url = Url::parse_with_params(VIDEO_WORKS_API, params)
        .map_err(|error| format!("B 站视频列表地址无效: {error}"))?;
    let value = request_plugin_json("GET", url.as_str(), cookie_header, VIDEO_WORKS_HEADERS)
        .await
        .map_err(|error| format!("B 站视频列表接口不可用: {error}"))?;
    if !bilibili_response_success(&value) {
        return Err(bilibili_error_message(&value, "B 站视频列表读取失败"));
    }
    let data = value.get("data").unwrap_or(&value);
    let mut works = data
        .get("arc_audits")
        .or_else(|| data.get("archives"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parse_bilibili_video_work(item, account_id))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for work in &mut works {
        materialize_bilibili_work_cover(work).await;
    }
    let total = data
        .get("page")
        .and_then(|page| count_key(page, "count"))
        .unwrap_or(works.len() as u64);
    let has_more = (page * BILI_WORKS_PAGE_SIZE) < total as i64;
    Ok(ChannelWorksPage {
        account_id: account_id.to_string(),
        platform_id: "bilibili".to_string(),
        page_key: page.to_string(),
        work_type: Some(BiliWorkKind::Video.as_str().to_string()),
        next_page_key: has_more.then(|| (page + 1).to_string()),
        has_more,
        works,
        updated_at: Some(Utc::now()),
        sync_status: "synced".to_string(),
        error: None,
    })
}

async fn fetch_bilibili_article_works_page(
    cookie_header: &str,
    account_id: &str,
    page_key: &str,
) -> Result<ChannelWorksPage, String> {
    let page = bilibili_page_number(page_key);
    let params = vec![
        ("ps", BILI_WORKS_PAGE_SIZE.to_string()),
        ("pn", page.to_string()),
        ("classification_type", "0".to_string()),
        ("creation_type", "0".to_string()),
    ];
    let url = Url::parse_with_params(ARTICLE_WORKS_API, params)
        .map_err(|error| format!("B 站图文列表地址无效: {error}"))?;
    let value = request_plugin_json("GET", url.as_str(), cookie_header, ARTICLE_WORKS_HEADERS)
        .await
        .map_err(|error| format!("B 站图文列表接口不可用: {error}"))?;
    if !bilibili_response_success(&value) {
        return Err(bilibili_error_message(&value, "B 站图文列表读取失败"));
    }
    let data = value.get("data").unwrap_or(&value);
    let mut works = data
        .get("items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parse_bilibili_article_work(item, account_id))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for work in &mut works {
        materialize_bilibili_work_cover(work).await;
    }
    let has_more = data
        .get("has_more")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            count_key(data, "total")
                .map(|total| (page * BILI_WORKS_PAGE_SIZE) < total as i64)
                .unwrap_or(false)
        });
    Ok(ChannelWorksPage {
        account_id: account_id.to_string(),
        platform_id: "bilibili".to_string(),
        page_key: page.to_string(),
        work_type: Some(BiliWorkKind::Article.as_str().to_string()),
        next_page_key: has_more.then(|| (page + 1).to_string()),
        has_more,
        works,
        updated_at: Some(Utc::now()),
        sync_status: "synced".to_string(),
        error: None,
    })
}

fn parse_bilibili_video_work(item: &Value, account_id: &str) -> Option<ChannelContentWork> {
    let archive = item.get("Archive").or_else(|| item.get("archive")).unwrap_or(item);
    let aid = count_key(archive, "aid")
        .map(|value| value.to_string())
        .or_else(|| text_key(archive, "aid"))?;
    let bvid = text_key(archive, "bvid");
    let title = text_key(archive, "title").unwrap_or_else(|| "未命名视频".to_string());
    let stat = item.get("stat").or_else(|| item.get("Stat"));
    let cover_url = first_profile_image(item, WORK_COVER_KEYS).map(bilibili_cover_thumbnail_url);
    let views = stat.and_then(|stat| count_key(stat, "view").or_else(|| count_key(stat, "vv")));
    let likes = stat.and_then(|stat| count_key(stat, "like"));
    let comments = stat.and_then(|stat| count_key(stat, "reply"));
    let collects = stat.and_then(|stat| count_key(stat, "favorite").or_else(|| count_key(stat, "fav")));
    let shares = stat.and_then(|stat| count_key(stat, "share"));
    let metrics = video_work_metrics(item, stat);
    Some(ChannelContentWork {
        id: format!("bilibili-video-{}", bvid.clone().unwrap_or_else(|| aid.clone())),
        platform_id: "bilibili".to_string(),
        account_id: account_id.to_string(),
        title,
        cover_url,
        link: Some(match bvid {
            Some(bvid) if !bvid.is_empty() => format!("https://www.bilibili.com/video/{bvid}"),
            _ => format!("https://www.bilibili.com/video/av{aid}"),
        }),
        published_at: count_key(archive, "ptime")
            .or_else(|| count_key(archive, "ctime"))
            .and_then(|value| DateTime::from_timestamp(value as i64, 0)),
        status: bilibili_archive_status(archive).to_string(),
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
        metrics,
        badges: bilibili_video_badges(archive),
        work_type: Some(BiliWorkKind::Video.as_str().to_string()),
    })
}

fn parse_bilibili_article_work(item: &Value, account_id: &str) -> Option<ChannelContentWork> {
    let dyn_id = text_key(item, "dyn_id")
        .or_else(|| text_key(item, "rid"))
        .or_else(|| text_key(item, "id"))?;
    let title = text_key(item, "title")
        .or_else(|| text_key(item, "summary"))
        .unwrap_or_else(|| "未命名图文".to_string());
    let stat = item.get("stat");
    let cover_url = first_profile_image(item, WORK_COVER_KEYS).map(bilibili_cover_thumbnail_url);
    let status = bilibili_article_status(item);
    let metrics = article_work_metrics(stat);
    Some(ChannelContentWork {
        id: format!("bilibili-article-{dyn_id}"),
        platform_id: "bilibili".to_string(),
        account_id: account_id.to_string(),
        title,
        cover_url,
        link: Some(format!("https://www.bilibili.com/opus/{dyn_id}")),
        published_at: text_key(item, "pub_time").and_then(|value| parse_bilibili_datetime(&value)),
        status: status.to_string(),
        views: stat.and_then(|stat| count_key(stat, "view")),
        impressions: None,
        likes: stat.and_then(|stat| count_key(stat, "like")),
        collects: stat.and_then(|stat| count_key(stat, "favorite")),
        comments: stat.and_then(|stat| count_key(stat, "reply")),
        shares: None,
        cover_click_rate: None,
        avg_view_time: None,
        gained_followers: None,
        data_updated_at: None,
        metrics,
        badges: bilibili_article_badges(item),
        work_type: Some(BiliWorkKind::Article.as_str().to_string()),
    })
}

async fn materialize_bilibili_work_cover(work: &mut ChannelContentWork) {
    let Some(cover_url) = work.cover_url.clone() else {
        return;
    };
    if cover_url.trim().is_empty() || cover_url.starts_with("data:image") {
        return;
    }
    work.cover_url = Some(materialize_platform_image("bilibili", cover_url).await);
}

fn bilibili_cover_thumbnail_url(value: String) -> String {
    let value = normalize_platform_image_url("bilibili", value);
    if value.starts_with("data:image") || !is_bilibili_image_host(&value) || value.contains('@') {
        return value;
    }
    format!("{value}@156w_98h_1c.webp")
}

fn is_bilibili_image_host(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()))
        .is_some_and(|host| host.ends_with(".hdslb.com"))
}

fn video_work_metrics(item: &Value, stat: Option<&Value>) -> Vec<ChannelWorkMetric> {
    if let Some(fields) = item.get("display_fields").and_then(Value::as_array) {
        let metrics = fields
            .iter()
            .filter_map(|field| {
                let label = text_key(field, "desc")?;
                Some(ChannelWorkMetric {
                    key: text_key(field, "name").unwrap_or_else(|| label.clone()),
                    label,
                    value: field.get("value").and_then(value_text),
                })
            })
            .collect::<Vec<_>>();
        if !metrics.is_empty() {
            return metrics;
        }
    }
    let specs = [
        ("view", "播放", "view"),
        ("like", "点赞", "like"),
        ("danmaku", "弹幕", "danmaku"),
        ("reply", "评论", "reply"),
        ("coin", "硬币", "coin"),
        ("favorite", "收藏", "favorite"),
        ("share", "分享", "share"),
    ];
    specs
        .iter()
        .map(|(key, label, stat_key)| ChannelWorkMetric {
            key: (*key).to_string(),
            label: (*label).to_string(),
            value: stat.and_then(|stat| count_key(stat, stat_key)).map(|value| value.to_string()),
        })
        .collect()
}

fn article_work_metrics(stat: Option<&Value>) -> Vec<ChannelWorkMetric> {
    let specs = [
        ("view", "浏览", "view"),
        ("like", "点赞", "like"),
        ("reply", "评论", "reply"),
        ("favorite", "收藏", "favorite"),
        ("coin", "硬币", "coin"),
    ];
    specs
        .iter()
        .map(|(key, label, stat_key)| ChannelWorkMetric {
            key: (*key).to_string(),
            label: (*label).to_string(),
            value: stat.and_then(|stat| count_key(stat, stat_key)).map(|value| value.to_string()),
        })
        .collect()
}

fn bilibili_archive_status(archive: &Value) -> &'static str {
    match signed_key(archive, "state").unwrap_or(0) {
        0 => "published",
        -1 | -4 | -30 | -40 => "reviewing",
        _ => "draft",
    }
}

fn bilibili_article_status(item: &Value) -> &'static str {
    match item
        .get("filter_group")
        .and_then(|value| signed_key(value, "filter_type"))
        .unwrap_or(2)
    {
        2 => "published",
        1 => "reviewing",
        _ => "draft",
    }
}

fn bilibili_video_badges(archive: &Value) -> Vec<String> {
    let mut badges = Vec::new();
    if archive.get("porder").is_some_and(|value| !value.is_null())
        || signed_key(archive, "is_top").unwrap_or(0) > 0
    {
        badges.push("置顶".to_string());
    }
    if signed_key(archive, "no_public").unwrap_or(0) > 0
        || archive
            .get("attrs")
            .and_then(|attrs| signed_key(attrs, "no_public"))
            .unwrap_or(0)
            > 0
        || signed_key(archive, "is_only_self").unwrap_or(0) > 0
    {
        badges.push("仅自己可见".to_string());
    } else if let Some(state_desc) = text_key(archive, "state_desc").filter(|value| !value.is_empty()) {
        badges.push(state_desc);
    }
    badges
}

fn bilibili_article_badges(item: &Value) -> Vec<String> {
    let mut badges = Vec::new();
    if let Some(reason) = item
        .get("filter_group")
        .and_then(|value| text_key(value, "reason"))
        .filter(|value| !value.is_empty() && value != "审核通过")
    {
        badges.push(reason);
    }
    if item
        .get("filter_group")
        .and_then(|value| text_key(value, "timing_waiting_pub"))
        .is_some_and(|value| value != "0")
    {
        badges.push("定时发布".to_string());
    }
    badges
}

fn bilibili_response_success(value: &Value) -> bool {
    first_i64(value, &["code"]).unwrap_or(-1) == 0
}

fn bilibili_error_message(value: &Value, fallback: &str) -> String {
    first_string(value, &["message", "msg"])
        .filter(|message| !message.trim().is_empty() && message.trim() != "0")
        .map(|message| format!("{fallback}: {message}"))
        .unwrap_or_else(|| fallback.to_string())
}

fn bilibili_page_number(page_key: &str) -> i64 {
    page_key
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn parse_bilibili_datetime(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(seconds) = value.parse::<i64>() {
        return DateTime::from_timestamp(seconds, 0);
    }
    let parsed = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M"))
        .ok()?;
    FixedOffset::east_opt(8 * 3600)?
        .from_local_datetime(&parsed)
        .single()
        .map(|value| value.with_timezone(&Utc))
}

fn text_key(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(value_text)
}

fn count_key(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(value_u64)
}

fn signed_key(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(value_i64)
}

fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let text = text.trim();
            if text.is_empty() || text == "-" {
                None
            } else {
                Some(text.to_string())
            }
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn value_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number
            .as_u64()
            .or_else(|| number.as_i64().filter(|value| *value >= 0).map(|value| value as u64)),
        Value::String(text) => text
            .trim()
            .replace(',', "")
            .parse::<u64>()
            .ok(),
        _ => None,
    }
}

fn value_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_u64().and_then(|value| i64::try_from(value).ok())),
        Value::String(text) => text
            .trim()
            .replace(',', "")
            .parse::<i64>()
            .ok(),
        _ => None,
    }
}

fn format_bilibili_delta(value: i64) -> Option<String> {
    if value == 0 {
        None
    } else if value > 0 {
        Some(format!("▲ {}", value))
    } else {
        Some(format!("▼ {}", value.abs()))
    }
}

fn delta_tone(value: i64) -> String {
    if value >= 0 {
        "up".to_string()
    } else {
        "down".to_string()
    }
}
