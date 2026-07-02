use super::*;
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::PathBuf, process::Stdio, sync::OnceLock};

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
const CREATOR_LATEST_NOTE_API: &str =
    "https://creator.xiaohongshu.com/api/galaxy/creator/home/latest_note_data";
const CREATOR_FANS_OVERALL_API: &str =
    "https://creator.xiaohongshu.com/api/galaxy/creator/data/fans/overall_new";
const CREATOR_ACCOUNT_BASE_API: &str =
    "https://creator.xiaohongshu.com/api/galaxy/v2/creator/datacenter/account/base";
const CREATOR_NOTE_DETAIL_API: &str =
    "https://creator.xiaohongshu.com/api/galaxy/creator/data/note_detail_new";
const CREATOR_NOTE_BASE_API: &str =
    "https://creator.xiaohongshu.com/api/galaxy/creator/datacenter/note/base";
const CREATOR_POSTED_NOTES_API: &str =
    "https://creator.xiaohongshu.com/api/galaxy/v2/creator/note/user/posted";
const EDITH_USER_ME_API: &str = "https://edith.xiaohongshu.com/api/sns/web/v2/user/me";
const XHS_CREATOR_SIGNER_JS: &str = include_str!("../../resources/xhs-signer/xhs_creator_260411.js");

const CREATOR_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://creator.xiaohongshu.com"),
    ("Referer", CREATOR_HOME_URL),
];
const EDITH_HEADERS: &[(&str, &str)] = &[
    ("Origin", "https://www.xiaohongshu.com"),
    ("Referer", "https://www.xiaohongshu.com/"),
];

const USER_UID_KEYS: &[&str] = &["redId", "red_id", "red_num", "redNum", "user_id", "userId", "id"];
const EDITH_UID_KEYS: &[&str] = &["redId", "red_id", "red_num", "redNum", "user_id", "userId", "id"];
const CREATOR_UID_KEYS: &[&str] = &[
    "redId",
    "red_id",
    "red_num",
    "redNum",
    "user_id",
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
const FOLLOWING_COUNT_KEYS: &[&str] = &[
    "following_count",
    "followingCount",
    "follow_count",
    "followCount",
    "follow_num",
    "followNum",
    "follows",
    "follows_count",
    "followsCount",
    "followings",
    "attention_count",
    "attentionCount",
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
const NOTE_ID_KEYS: &[&str] = &["id", "note_id", "noteId", "item_id", "itemId"];
const NOTE_TITLE_KEYS: &[&str] = &["title", "display_title", "displayTitle", "desc", "content"];
const NOTE_COVER_KEYS: &[&str] = &[
    "coverUrl",
    "cover_url",
    "cover",
    "image",
    "image_url",
    "imageUrl",
    "url",
];
const NOTE_LINK_KEYS: &[&str] = &["link", "url", "share_url", "shareUrl"];
const NOTE_TIME_KEYS: &[&str] = &["postTime", "post_time", "publishTime", "publish_time", "time"];
const NOTE_VIEW_KEYS: &[&str] = &["view_count", "viewCount", "views", "read_count", "readCount"];
const NOTE_IMPRESSION_KEYS: &[&str] = &[
    "impl_count",
    "exposure_count",
    "exposureCount",
    "impression_count",
    "impressionCount",
];
const NOTE_LIKE_KEYS: &[&str] = &["likes", "like_count", "likeCount", "liked_count", "likedCount"];
const NOTE_COLLECT_KEYS: &[&str] = &[
    "collected_count",
    "collect_count",
    "collectCount",
    "collects",
    "fav_count",
    "favCount",
];
const NOTE_COMMENT_KEYS: &[&str] = &[
    "comments_count",
    "comment_count",
    "commentCount",
    "comments",
];
const NOTE_SHARE_KEYS: &[&str] = &["shared_count", "share_count", "shareCount", "shares"];
const NOTE_DETAIL_IMPRESSION_KEYS: &[&str] = &[
    "implCount",
    "impl_count",
    "impCount",
    "imp_count",
    "exposureCount",
    "exposure_count",
];
const NOTE_DETAIL_VIEW_KEYS: &[&str] = &["viewCount", "view_count", "views"];
const NOTE_DETAIL_COVER_CLICK_RATE_KEYS: &[&str] = &["coverClickRate", "cover_click_rate"];
const NOTE_DETAIL_AVG_VIEW_TIME_KEYS: &[&str] = &[
    "viewTimeAvgWithDouble",
    "avg_view_time",
    "avgViewTimeWithDouble",
    "view_time_avg_with_double",
    "viewTimeAvg",
    "view_time_avg",
    "avgViewTime",
];
const NOTE_DETAIL_GAINED_FOLLOWER_KEYS: &[&str] = &[
    "riseFansCount",
    "rise_fans_count",
    "followFromDiscovery",
    "netRiseFansCount",
];
const NOTE_DETAIL_UPDATED_AT_KEYS: &[&str] = &[
    "basicDataLastUpdateTime",
    "analyseInfosLastUpdateTime",
    "dataLastUpdateTime",
    "data_update_time",
    "dataUpdateTime",
    "lastUpdateTime",
    "basic_data_last_update_time",
    "analyse_infos_last_update_time",
    "data_last_update_time",
    "end_time",
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
    let creator_following_count = creator_data.and_then(|data| first_count(data, FOLLOWING_COUNT_KEYS));
    let creator_like_count = creator_data.and_then(|data| first_count(data, LIKE_COUNT_KEYS));
    let user_ok = user
        .as_ref()
        .map(|value| response_success(value) && user_uid.is_some())
        .unwrap_or(false);
    let creator_has_profile = creator_uid.is_some()
        || creator_nickname.is_some()
        || creator_avatar.is_some()
        || creator_fans_count.is_some()
        || creator_following_count.is_some()
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

    let uid = user_uid.or(creator_uid).or(edith_uid).unwrap_or_default();
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
        following_count: creator_following_count
            .or_else(|| first_count_from_values(&[user_data, edith_data], FOLLOWING_COUNT_KEYS)),
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct XhsCreatorSignatureInput<'a> {
    api: &'a str,
    data: &'a str,
    a1: &'a str,
}

#[derive(Debug, Deserialize)]
struct XhsCreatorSignature {
    #[serde(rename = "x-s")]
    x_s: String,
    #[serde(rename = "x-t")]
    x_t: String,
    #[serde(rename = "x-s-common")]
    x_s_common: String,
    #[serde(rename = "x-b3-traceid")]
    x_b3_traceid: String,
    #[serde(rename = "x-xray-traceid")]
    x_xray_traceid: String,
}

async fn request_xhs_creator_signed_json(
    method: &str,
    url: &str,
    cookie_header: &str,
    login_cookie: &str,
) -> Result<Value, String> {
    let a1 = xhs_cookie_value(login_cookie, cookie_header, "a1")
        .ok_or_else(|| "小红书 Cookie 缺少 a1，无法生成创作中心接口签名，请重新登录。".to_string())?;
    let api = xhs_creator_signature_api(url)?;
    let signature = generate_xhs_creator_signature(&api, "", &a1)?;
    let client = Client::new();
    let mut request = if method.eq_ignore_ascii_case("POST") {
        client.post(url)
    } else {
        client.get(url)
    };
    request = request
        .header("Cookie", cookie_header)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 Edg/138.0.0.0")
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .header("Origin", "https://creator.xiaohongshu.com")
        .header("Referer", CREATOR_HOME_URL)
        .header("sec-ch-ua", "\"Not)A;Brand\";v=\"8\", \"Chromium\";v=\"138\", \"Microsoft Edge\";v=\"138\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header("x-s", signature.x_s)
        .header("x-t", signature.x_t)
        .header("x-s-common", signature.x_s_common)
        .header("x-b3-traceid", signature.x_b3_traceid)
        .header("x-xray-traceid", signature.x_xray_traceid)
        .timeout(std::time::Duration::from_secs(18));
    if method.eq_ignore_ascii_case("POST") {
        request = request
            .header("Content-Type", "application/json;charset=utf-8")
            .body("{}");
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("小红书创作中心接口请求失败: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("小红书创作中心接口返回 HTTP {status}"));
    }
    response
        .json()
        .await
        .map_err(|error| format!("小红书创作中心接口不是 JSON: {error}"))
}

async fn request_xhs_creator_payload(
    method: &str,
    url: &str,
    cookie_header: &str,
    login_cookie: &str,
) -> Result<Option<Value>, String> {
    request_xhs_creator_signed_json(method, url, cookie_header, login_cookie)
        .await
        .map(|value| xhs_response_payload(&value).cloned())
}

fn record_xhs_sync_result(
    label: &str,
    result: Result<Option<Value>, String>,
    sync_errors: &mut Vec<String>,
) -> Option<Value> {
    match result {
        Ok(value) => value,
        Err(error) => {
            sync_errors.push(format!("{label}: {error}"));
            None
        }
    }
}

fn xhs_creator_api_url(base: &str, params: &[(&str, &str)]) -> String {
    let mut url = base.to_string();
    for (index, (key, value)) in params.iter().enumerate() {
        url.push(if index == 0 { '?' } else { '&' });
        url.push_str(key);
        url.push('=');
        url.push_str(&encode_query(value));
    }
    url
}

fn xhs_creator_signature_api(url: &str) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|error| format!("小红书创作中心接口 URL 无效: {error}"))?;
    let mut api = parsed.path().to_string();
    if let Some(query) = parsed.query().filter(|value| !value.trim().is_empty()) {
        api.push('?');
        api.push_str(query);
    }
    Ok(api)
}

fn generate_xhs_creator_signature(api: &str, data: &str, a1: &str) -> Result<XhsCreatorSignature, String> {
    let signer_path = xhs_creator_signer_path()?;
    let input = XhsCreatorSignatureInput { api, data, a1 };
    let mut child = Command::new("node")
        .arg("-e")
        .arg(XHS_CREATOR_SIGNER_RUNNER)
        .arg(&signer_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("启动小红书签名运行时失败，请确认已安装 Node.js: {error}"))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| "小红书签名运行时输入不可用".to_string())?;
        let payload = serde_json::to_vec(&input).map_err(|error| format!("小红书签名参数序列化失败: {error}"))?;
        stdin
            .write_all(&payload)
            .map_err(|error| format!("写入小红书签名参数失败: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("读取小红书签名结果失败: {error}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            "小红书签名运行时执行失败".to_string()
        } else {
            format!("小红书签名运行时执行失败: {message}")
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let signature_json = stdout
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with('{') && line.ends_with('}'))
        .unwrap_or_else(|| stdout.trim());
    serde_json::from_str::<XhsCreatorSignature>(signature_json)
        .map_err(|error| format!("解析小红书签名结果失败: {error}"))
}

fn xhs_creator_signer_path() -> Result<PathBuf, String> {
    static SIGNER_PATH: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    SIGNER_PATH
        .get_or_init(|| {
            let dir = std::env::temp_dir().join("channel-nest-xhs-signer");
            fs::create_dir_all(&dir).map_err(|error| format!("创建小红书签名目录失败: {error}"))?;
            let path = dir.join("xhs_creator_260411.js");
            let should_write = fs::metadata(&path)
                .map(|metadata| metadata.len() != XHS_CREATOR_SIGNER_JS.len() as u64)
                .unwrap_or(true);
            if should_write {
                fs::write(&path, XHS_CREATOR_SIGNER_JS)
                    .map_err(|error| format!("写入小红书签名资源失败: {error}"))?;
            }
            Ok(path)
        })
        .clone()
}

fn xhs_cookie_value(login_cookie: &str, cookie_header: &str, name: &str) -> Option<String> {
    let trimmed = login_cookie.trim();
    if trimmed.starts_with('[') {
        if let Ok(Value::Array(cookies)) = serde_json::from_str::<Value>(trimmed) {
            if let Some(value) = cookies.iter().find_map(|cookie| {
                let cookie_name = cookie.get("name")?.as_str()?;
                if cookie_name == name {
                    cookie.get("value")?.as_str().map(ToString::to_string)
                } else {
                    None
                }
            }) {
                return Some(value);
            }
        }
    }
    cookie_header.split(';').find_map(|part| {
        let (cookie_name, value) = part.trim().split_once('=')?;
        if cookie_name.trim() == name {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

const XHS_CREATOR_SIGNER_RUNNER: &str = r#"
const fs = require("fs");
const crypto = require("crypto");
const Module = require("module");

const signerPath = process.argv[1];
const input = JSON.parse(fs.readFileSync(0, "utf8"));

const originalLoad = Module._load;
Module._load = function(request, parent, isMain) {
  if (request === "crypto-js") {
    return {
      MD5(value) {
        return {
          toString() {
            return crypto.createHash("md5").update(String(value)).digest("hex");
          }
        };
      }
    };
  }
  return originalLoad.apply(this, arguments);
};

function traceHex(length) {
  return crypto.randomBytes(Math.ceil(length / 2)).toString("hex").slice(0, length);
}

const originalConsole = {
  log: console.log,
  info: console.info,
  warn: console.warn,
  error: console.error
};

function muteConsole() {
  console.log = console.info = console.warn = console.error = function() {};
}

function restoreConsole() {
  console.log = originalConsole.log;
  console.info = originalConsole.info;
  console.warn = originalConsole.warn;
  console.error = originalConsole.error;
}

let signature;
muteConsole();
try {
  const signer = require(signerPath);
  const result = signer.get_request_headers_params(input.api, input.data || "", input.a1);
  signature = {
    "x-s": result.xs,
    "x-t": String(result.xt),
    "x-s-common": result.xs_common,
    "x-b3-traceid": traceHex(16),
    "x-xray-traceid": traceHex(32)
  };
} finally {
  restoreConsole();
}

process.stdout.write(JSON.stringify(signature));
"#;

pub(super) async fn fetch_xhs_account_content(
    cookie_header: &str,
    login_cookie: String,
    account_id: &str,
) -> Result<ChannelAccountContent, String> {
    let now = Utc::now();
    let profile = fetch_xhs_plugin_account_from_cookie(cookie_header, login_cookie.clone(), Some("creator"))
        .await
        .map_err(|error| plugin_error_message(&error))?;

    let profile_snapshot = ChannelAccountProfileSnapshot {
        account_id: account_id.to_string(),
        platform_id: "xiaohongshu".to_string(),
        followers: profile.fans_count,
        following: profile.following_count,
        likes: profile.like_count,
        last_sync_at: Some(now),
        updated_at: Some(now),
        sync_status: "synced".to_string(),
        error: None,
    };

    let mut sync_errors = Vec::new();
    let account_base = record_xhs_sync_result(
        "账号数据",
        request_xhs_creator_payload("GET", CREATOR_ACCOUNT_BASE_API, cookie_header, &login_cookie)
            .await,
        &mut sync_errors,
    );
    let latest_note = record_xhs_sync_result(
        "最新笔记",
        request_xhs_creator_payload("GET", CREATOR_LATEST_NOTE_API, cookie_header, &login_cookie)
            .await,
        &mut sync_errors,
    );
    let fans_overall = record_xhs_sync_result(
        "粉丝数据",
        request_xhs_creator_payload("GET", CREATOR_FANS_OVERALL_API, cookie_header, &login_cookie)
            .await,
        &mut sync_errors,
    );

    let latest_note_info = latest_note.as_ref().and_then(|value| value.get("noteInfo")).cloned();
    let note_id = latest_note_info
        .as_ref()
        .and_then(|value| first_string_deep(value, NOTE_ID_KEYS));
    let note_base = if let Some(note_id) = note_id.as_deref().filter(|value| !value.trim().is_empty()) {
        let url = xhs_creator_api_url(CREATOR_NOTE_BASE_API, &[("note_id", note_id)]);
        record_xhs_sync_result(
            "笔记核心数据",
            request_xhs_creator_payload("GET", &url, cookie_header, &login_cookie).await,
            &mut sync_errors,
        )
    } else {
        None
    };
    let note_detail = if let Some(note_id) = note_id.as_deref().filter(|value| !value.trim().is_empty()) {
        let url = xhs_creator_api_url(CREATOR_NOTE_DETAIL_API, &[("note_id", note_id)]);
        record_xhs_sync_result(
            "笔记详情",
            request_xhs_creator_payload("GET", &url, cookie_header, &login_cookie).await,
            &mut sync_errors,
        )
    } else {
        None
    };
    if account_base.is_none() && latest_note.is_none() && note_detail.is_none() {
        let message = if sync_errors.is_empty() {
            "小红书创作中心没有返回可用数据".to_string()
        } else {
            sync_errors.join("；")
        };
        return Err(message);
    }

    let (latest_work, latest_work_seven, latest_work_thirty) = build_xhs_latest_works(
        latest_note_info.as_ref(),
        latest_note.as_ref(),
        note_base.as_ref(),
        note_detail.as_ref(),
        account_id,
    )
    .await;

    Ok(ChannelAccountContent {
        account_id: account_id.to_string(),
        platform_id: "xiaohongshu".to_string(),
        profile: Some(profile_snapshot),
        overview_yesterday: None,
        overview_seven: Some(build_xhs_period_overview(
            account_id,
            7,
            account_base.as_ref(),
            note_detail.as_ref(),
            fans_overall.as_ref(),
            now,
        )),
        overview_thirty: Some(build_xhs_period_overview(
            account_id,
            30,
            account_base.as_ref(),
            note_detail.as_ref(),
            fans_overall.as_ref(),
            now,
        )),
        latest_work,
        latest_work_seven,
        latest_work_thirty,
        sync_status: "synced".to_string(),
        error: None,
        ..Default::default()
    })
}

pub(super) async fn fetch_xhs_works_page(
    cookie_header: &str,
    login_cookie: &str,
    account_id: &str,
    page_key: &str,
) -> Result<ChannelWorksPage, String> {
    let mut url = format!("{CREATOR_POSTED_NOTES_API}?tab=0");
    if !page_key.trim().is_empty() {
        url.push_str("&page=");
        url.push_str(&encode_query(page_key.trim()));
    }
    let value = request_xhs_creator_signed_json("GET", &url, cookie_header, login_cookie)
        .await
        .map_err(|error| format!("小红书作品列表请求失败: {error}"))?;
    if !response_success(&value) {
        let message = value
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("小红书作品列表同步失败");
        return Err(message.to_string());
    }
    let data = xhs_response_payload(&value).ok_or_else(|| "小红书作品列表没有返回数据".to_string())?;
    let mut works = Vec::new();
    for item in data
        .get("notes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(mut work) = parse_xhs_work(item, account_id, "list") {
            materialize_xhs_work_cover(&mut work).await;
            works.push(work);
        }
    }

    let next_page_key = data.get("page").and_then(page_key_from_value);
    let has_more = next_page_key
        .as_deref()
        .map(|value| value != "-1" && !value.trim().is_empty())
        .unwrap_or(false);

    Ok(ChannelWorksPage {
        account_id: account_id.to_string(),
        platform_id: "xiaohongshu".to_string(),
        page_key: page_key.to_string(),
        work_type: None,
        next_page_key: if has_more { next_page_key } else { None },
        has_more,
        works,
        updated_at: Some(Utc::now()),
        sync_status: "synced".to_string(),
        error: None,
    })
}

async fn build_xhs_latest_works(
    latest_note_info: Option<&Value>,
    latest_note: Option<&Value>,
    note_base: Option<&Value>,
    note_detail: Option<&Value>,
    account_id: &str,
) -> (
    Option<ChannelContentWork>,
    Option<ChannelContentWork>,
    Option<ChannelContentWork>,
) {
    let mut latest_work =
        latest_note_info.and_then(|value| parse_xhs_work(value, account_id, "latest"));
    if let Some(work) = latest_work.as_mut() {
        materialize_xhs_work_cover(work).await;
        apply_note_base_to_work(work, latest_note);
        apply_note_base_to_work(work, note_base);
    }

    let mut latest_work_seven = latest_work.clone();
    if let Some(work) = latest_work_seven.as_mut() {
        apply_note_base_to_work(work, note_detail.and_then(|value| value.get("seven")));
    }

    let mut latest_work_thirty = latest_work.clone();
    if let Some(work) = latest_work_thirty.as_mut() {
        apply_note_base_to_work(work, note_detail.and_then(|value| value.get("thirty")));
    }

    (latest_work, latest_work_seven, latest_work_thirty)
}

fn build_xhs_period_overview(
    account_id: &str,
    period_days: u16,
    account_base: Option<&Value>,
    note_detail: Option<&Value>,
    fans_overall: Option<&Value>,
    now: DateTime<Utc>,
) -> ChannelAccountOverview {
    let period_key = if period_days == 30 { "thirty" } else { "seven" };
    account_base
        .and_then(|value| value.get(period_key))
        .map(|value| build_xhs_datacenter_overview(account_id, period_days, value, now))
        .unwrap_or_else(|| {
            build_xhs_overview(
                account_id,
                period_days,
                note_detail.and_then(|value| value.get(period_key)),
                fans_overall.and_then(|value| value.get(period_key)),
                now,
            )
        })
}

fn build_xhs_overview(
    account_id: &str,
    period_days: u16,
    note: Option<&Value>,
    fans: Option<&Value>,
    now: DateTime<Utc>,
) -> ChannelAccountOverview {
    ChannelAccountOverview {
        account_id: account_id.to_string(),
        platform_id: "xiaohongshu".to_string(),
        period_days,
        metrics: xhs_fallback_overview_metrics(note, fans),
        summary: note
            .and_then(|value| value.get("summary"))
            .and_then(Value::as_str)
            .map(strip_html)
            .filter(|value| !value.trim().is_empty()),
        updated_at: Some(now),
        sync_status: "synced".to_string(),
        error: None,
    }
}

#[derive(Clone, Copy)]
enum XhsFallbackMetricSource {
    Empty,
    NoteCount,
    NetFans,
    RiseFans,
    LeaveFans,
}

#[derive(Clone, Copy)]
struct XhsFallbackMetricSpec {
    key: &'static str,
    label: &'static str,
    source: XhsFallbackMetricSource,
    value_key: Option<&'static str>,
    trend_key: Option<&'static str>,
}

impl XhsFallbackMetricSpec {
    const fn empty(key: &'static str, label: &'static str) -> Self {
        Self {
            key,
            label,
            source: XhsFallbackMetricSource::Empty,
            value_key: None,
            trend_key: None,
        }
    }

    const fn note_count(
        key: &'static str,
        label: &'static str,
        value_key: &'static str,
        trend_key: &'static str,
    ) -> Self {
        Self {
            key,
            label,
            source: XhsFallbackMetricSource::NoteCount,
            value_key: Some(value_key),
            trend_key: Some(trend_key),
        }
    }

    const fn net_fans(key: &'static str, label: &'static str) -> Self {
        Self {
            key,
            label,
            source: XhsFallbackMetricSource::NetFans,
            value_key: None,
            trend_key: None,
        }
    }

    const fn rise_fans(key: &'static str, label: &'static str) -> Self {
        Self {
            key,
            label,
            source: XhsFallbackMetricSource::RiseFans,
            value_key: None,
            trend_key: None,
        }
    }

    const fn leave_fans(key: &'static str, label: &'static str) -> Self {
        Self {
            key,
            label,
            source: XhsFallbackMetricSource::LeaveFans,
            value_key: None,
            trend_key: None,
        }
    }
}

const XHS_FALLBACK_OVERVIEW_METRICS: &[XhsFallbackMetricSpec] = &[
    XhsFallbackMetricSpec::empty("impressions", "曝光数"),
    XhsFallbackMetricSpec::note_count("views", "观看数", "view_count", "view_count_rate"),
    XhsFallbackMetricSpec::note_count("likes", "点赞数", "like_count", "like_count_rate"),
    XhsFallbackMetricSpec::note_count("comments", "评论数", "comment_count", "comment_count_rate"),
    XhsFallbackMetricSpec::net_fans("netFans", "净涨粉"),
    XhsFallbackMetricSpec::rise_fans("newFollows", "新增关注"),
    XhsFallbackMetricSpec::empty("coverClickRate", "封面点击率"),
    XhsFallbackMetricSpec::empty("completionRate", "视频完播率"),
    XhsFallbackMetricSpec::note_count("collects", "收藏数", "collect_count", "collect_count_rate"),
    XhsFallbackMetricSpec::note_count("shares", "分享数", "share_count", "share_count_rate"),
    XhsFallbackMetricSpec::leave_fans("unfollows", "取消关注"),
    XhsFallbackMetricSpec::note_count("homepageVisitors", "主页访客", "home_view_count", "home_view_count_rate"),
];

fn xhs_fallback_overview_metrics(
    note: Option<&Value>,
    fans: Option<&Value>,
) -> Vec<ChannelOverviewMetric> {
    let rise_fans = fans.and_then(|value| signed_count(value, "rise_fans_count"));
    let leave_fans = fans.and_then(|value| signed_count(value, "leave_fans_count"));
    let net_fans = match (rise_fans, leave_fans) {
        (Some(rise), Some(leave)) => Some(rise - leave),
        (Some(rise), None) => Some(rise),
        _ => None,
    };
    XHS_FALLBACK_OVERVIEW_METRICS
        .iter()
        .map(|spec| xhs_fallback_overview_metric(note, spec, rise_fans, leave_fans, net_fans))
        .collect()
}

fn xhs_fallback_overview_metric(
    note: Option<&Value>,
    spec: &XhsFallbackMetricSpec,
    rise_fans: Option<i64>,
    leave_fans: Option<i64>,
    net_fans: Option<i64>,
) -> ChannelOverviewMetric {
    let value = match spec.source {
        XhsFallbackMetricSource::Empty => None,
        XhsFallbackMetricSource::NoteCount => spec
            .value_key
            .and_then(|key| note.and_then(|value| unsigned_count(value, key)))
            .map(number_text),
        XhsFallbackMetricSource::NetFans => net_fans.map(signed_number_text),
        XhsFallbackMetricSource::RiseFans => rise_fans.map(signed_number_text),
        XhsFallbackMetricSource::LeaveFans => leave_fans.map(signed_number_text),
    };
    let trend = match spec.source {
        XhsFallbackMetricSource::NoteCount => spec
            .trend_key
            .and_then(|key| note.and_then(|value| trend_text(value, key))),
        _ => None,
    };
    overview_metric(spec.key, spec.label, value, trend)
}

fn build_xhs_datacenter_overview(
    account_id: &str,
    period_days: u16,
    data: &Value,
    now: DateTime<Utc>,
) -> ChannelAccountOverview {
    ChannelAccountOverview {
        account_id: account_id.to_string(),
        platform_id: "xiaohongshu".to_string(),
        period_days,
        metrics: xhs_datacenter_metrics(data),
        summary: data
            .get("summary")
            .and_then(Value::as_str)
            .map(strip_html)
            .filter(|value| !value.trim().is_empty()),
        updated_at: Some(now),
        sync_status: "synced".to_string(),
        error: None,
    }
}

#[derive(Clone, Copy)]
enum XhsMetricValueKind {
    Count,
    SignedCount,
    Percent,
}

#[derive(Clone, Copy)]
struct XhsDatacenterMetricSpec {
    key: &'static str,
    label: &'static str,
    value_key: &'static str,
    trend_key: &'static str,
    trend_display_key: &'static str,
    value_kind: XhsMetricValueKind,
}

impl XhsDatacenterMetricSpec {
    const fn count(
        key: &'static str,
        label: &'static str,
        value_key: &'static str,
        trend_key: &'static str,
        trend_display_key: &'static str,
    ) -> Self {
        Self {
            key,
            label,
            value_key,
            trend_key,
            trend_display_key,
            value_kind: XhsMetricValueKind::Count,
        }
    }

    const fn signed_count(
        key: &'static str,
        label: &'static str,
        value_key: &'static str,
        trend_key: &'static str,
        trend_display_key: &'static str,
    ) -> Self {
        Self {
            key,
            label,
            value_key,
            trend_key,
            trend_display_key,
            value_kind: XhsMetricValueKind::SignedCount,
        }
    }

    const fn percent(
        key: &'static str,
        label: &'static str,
        value_key: &'static str,
        trend_key: &'static str,
        trend_display_key: &'static str,
    ) -> Self {
        Self {
            key,
            label,
            value_key,
            trend_key,
            trend_display_key,
            value_kind: XhsMetricValueKind::Percent,
        }
    }
}

const XHS_DATACENTER_METRICS: &[XhsDatacenterMetricSpec] = &[
    XhsDatacenterMetricSpec::count("impressions", "曝光数", "impl_count", "impl_count_rate", "impl_count_rate_display"),
    XhsDatacenterMetricSpec::count("views", "观看数", "view_count", "view_count_rate", "view_count_rate_display"),
    XhsDatacenterMetricSpec::count("likes", "点赞数", "like_count", "like_count_rate", "like_count_rate_display"),
    XhsDatacenterMetricSpec::count("comments", "评论数", "comment_count", "comment_count_rate", "comment_count_rate_display"),
    XhsDatacenterMetricSpec::signed_count(
        "netFans",
        "净涨粉",
        "net_rise_fans_count",
        "net_rise_fans_count_rate",
        "net_rise_fans_count_rate_display",
    ),
    XhsDatacenterMetricSpec::count(
        "newFollows",
        "新增关注",
        "rise_fans_count",
        "rise_fans_count_rate",
        "rise_fans_count_rate_display",
    ),
    XhsDatacenterMetricSpec::percent(
        "coverClickRate",
        "封面点击率",
        "cover_click_rate",
        "cover_click_cycle_rate",
        "cover_click_cycle_rate_display",
    ),
    XhsDatacenterMetricSpec::percent(
        "completionRate",
        "视频完播率",
        "video_full_view_rate",
        "video_full_view_cycle_rate",
        "video_full_view_cycle_rate_display",
    ),
    XhsDatacenterMetricSpec::count("collects", "收藏数", "collect_count", "collect_count_rate", "collect_count_rate_display"),
    XhsDatacenterMetricSpec::count("shares", "分享数", "share_count", "share_count_rate", "share_count_rate_display"),
    XhsDatacenterMetricSpec::count(
        "unfollows",
        "取消关注",
        "loss_fans_count",
        "loss_fans_count_rate",
        "loss_fans_count_rate_display",
    ),
    XhsDatacenterMetricSpec::count(
        "homepageVisitors",
        "主页访客",
        "home_view_count",
        "home_view_count_rate",
        "home_view_count_rate_display",
    ),
];

fn xhs_datacenter_metrics(data: &Value) -> Vec<ChannelOverviewMetric> {
    XHS_DATACENTER_METRICS
        .iter()
        .map(|spec| xhs_datacenter_metric(data, spec))
        .collect()
}

fn xhs_datacenter_metric(data: &Value, spec: &XhsDatacenterMetricSpec) -> ChannelOverviewMetric {
    let value = match spec.value_kind {
        XhsMetricValueKind::Count => unsigned_count(data, spec.value_key).map(number_text),
        XhsMetricValueKind::SignedCount => signed_count(data, spec.value_key).map(signed_number_text),
        XhsMetricValueKind::Percent => decimal_value(data, spec.value_key).map(percent_text),
    };
    overview_metric(
        spec.key,
        spec.label,
        value,
        datacenter_trend_text(data, spec.trend_key, spec.trend_display_key),
    )
}

fn datacenter_trend_text(data: &Value, key: &str, display_key: &str) -> Option<String> {
    if data
        .get(display_key)
        .and_then(Value::as_bool)
        .map(|display| !display)
        .unwrap_or(false)
    {
        return Some("-".to_string());
    }
    decimal_value(data, key).map(trend_number_text)
}

fn overview_metric(
    key: &str,
    label: &str,
    value: Option<String>,
    trend: Option<String>,
) -> ChannelOverviewMetric {
    let tone = trend.as_deref().and_then(|trend| {
        if trend.starts_with('-') {
            Some("down".to_string())
        } else if trend.starts_with('+') {
            Some("up".to_string())
        } else {
            None
        }
    });
    ChannelOverviewMetric {
        key: key.to_string(),
        label: label.to_string(),
        value,
        compare_label: Some("环比".to_string()),
        trend,
        tone,
    }
}

async fn materialize_xhs_work_cover(work: &mut ChannelContentWork) {
    let Some(cover_url) = work.cover_url.clone() else {
        return;
    };
    if cover_url.trim().is_empty() || cover_url.starts_with("data:image") {
        return;
    }
    work.cover_url = Some(materialize_platform_image("xiaohongshu", cover_url).await);
}

fn parse_xhs_work(value: &Value, account_id: &str, source: &str) -> Option<ChannelContentWork> {
    let id = first_string_deep(value, NOTE_ID_KEYS)?;
    let title = first_string_deep(value, NOTE_TITLE_KEYS).unwrap_or_else(|| "未命名作品".to_string());
    let cover_url = first_profile_image(value, NOTE_COVER_KEYS)
        .map(normalize_image_url)
        .map(|value| normalize_platform_image_url("xiaohongshu", value));
    let link = first_string_deep(value, NOTE_LINK_KEYS);
    let published_at = first_time(value, NOTE_TIME_KEYS);
    Some(ChannelContentWork {
        id: format!("xiaohongshu-{source}-{id}"),
        platform_id: "xiaohongshu".to_string(),
        account_id: account_id.to_string(),
        title,
        cover_url,
        link,
        published_at,
        status: "published".to_string(),
        views: first_count(value, NOTE_VIEW_KEYS),
        impressions: first_count(value, NOTE_IMPRESSION_KEYS),
        likes: first_count(value, NOTE_LIKE_KEYS),
        collects: first_count(value, NOTE_COLLECT_KEYS),
        comments: first_count(value, NOTE_COMMENT_KEYS),
        shares: first_count(value, NOTE_SHARE_KEYS),
        cover_click_rate: None,
        avg_view_time: None,
        gained_followers: None,
        data_updated_at: None,
        metrics: Vec::new(),
        badges: xhs_work_badges(value),
        work_type: None,
    })
}

fn xhs_work_badges(value: &Value) -> Vec<String> {
    let mut badges = Vec::new();
    if any_truthy_deep(value, XHS_PINNED_BOOL_KEYS)
        || any_text_contains_deep(value, XHS_PINNED_TEXT_KEYS, "置顶")
    {
        push_unique_badge(&mut badges, "置顶");
    }
    if let Some(label) = xhs_visibility_badge(value) {
        push_unique_badge(&mut badges, label);
    }
    badges
}

const XHS_PINNED_BOOL_KEYS: &[&str] = &[
    "is_top",
    "isTop",
    "top",
    "is_pinned",
    "isPinned",
    "pinned",
    "is_sticky",
    "isSticky",
    "sticky",
    "is_stick",
    "isStick",
    "stick",
    "is_top_note",
    "isTopNote",
    "top_note",
    "topNote",
    "top_status",
    "topStatus",
    "sticky_status",
    "stickyStatus",
    "pinned_status",
    "pinnedStatus",
    "pin",
    "is_pin",
    "isPin",
    "is_pinned_note",
    "isPinnedNote",
    "pinned_note",
    "pinnedNote",
    "stick_status",
    "stickStatus",
    "is_note_top",
    "isNoteTop",
    "note_top",
    "noteTop",
    "show_top",
    "showTop",
    "top_flag",
    "topFlag",
    "is_top_flag",
    "isTopFlag",
];
const XHS_PINNED_TEXT_KEYS: &[&str] = &[
    "label_top_text",
    "labelTopText",
    "top_label",
    "topLabel",
    "top_text",
    "topText",
    "tag_text",
    "tagText",
    "tag_name",
    "tagName",
    "tag_title",
    "tagTitle",
    "label",
    "label_text",
    "labelText",
    "label_name",
    "labelName",
    "status_text",
    "statusText",
    "note_status_text",
    "noteStatusText",
    "type_name",
    "typeName",
];

fn xhs_visibility_badge(value: &Value) -> Option<String> {
    first_string_or_number_deep(
        value,
        &[
            "visibility",
            "visible_type",
            "visibleType",
            "visible_status",
            "visibleStatus",
            "privacy",
            "privacy_type",
            "privacyType",
            "private_status",
            "privateStatus",
            "permission",
            "permission_type",
            "permissionType",
        ],
    )
    .and_then(|value| xhs_visibility_label(&value))
}

fn xhs_visibility_label(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    if value.contains("公开") || lower.contains("public") {
        return Some("公开".to_string());
    }
    if value.contains("仅自己")
        || value.contains("私密")
        || value.contains("隐藏")
        || lower.contains("private")
        || lower.contains("only_me")
    {
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
        "1" => Some("仅自己可见".to_string()),
        "2" => Some("关注可见".to_string()),
        "3" => Some("好友可见".to_string()),
        _ if value.chars().count() <= 12 && value.contains("可见") => Some(value.to_string()),
        _ => None,
    }
}

fn any_truthy_deep(value: &Value, keys: &[&str]) -> bool {
    match value {
        Value::Object(map) => {
            for key in keys {
                if map.get(*key).and_then(value_to_bool).unwrap_or(false) {
                    return true;
                }
            }
            map.values().any(|value| any_truthy_deep(value, keys))
        }
        Value::Array(items) => items.iter().any(|value| any_truthy_deep(value, keys)),
        _ => false,
    }
}

fn any_text_contains_deep(value: &Value, keys: &[&str], needle: &str) -> bool {
    match value {
        Value::Object(map) => {
            for key in keys {
                if map
                    .get(*key)
                    .and_then(value_to_text)
                    .map(|value| value.contains(needle))
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            map.values()
                .any(|value| any_text_contains_deep(value, keys, needle))
        }
        Value::Array(items) => items
            .iter()
            .any(|value| any_text_contains_deep(value, keys, needle)),
        _ => false,
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

fn push_unique_badge(badges: &mut Vec<String>, label: impl Into<String>) {
    let label = label.into().trim().to_string();
    if !label.is_empty() && !badges.iter().any(|item| item == &label) {
        badges.push(label);
    }
}

fn apply_note_base_to_work(work: &mut ChannelContentWork, detail: Option<&Value>) {
    let Some(detail) = detail else {
        return;
    };
    let result = detail.get("result").filter(|value| value.is_object());
    let view_data = detail.get("viewData").filter(|value| value.is_object());
    let fans_data = detail.get("fansData").filter(|value| value.is_object());
    let note_info = detail.get("note_info").filter(|value| value.is_object());
    let values = [result, view_data, fans_data, note_info, Some(detail)];

    apply_xhs_work_count_metrics(work, &values);
    apply_optional(
        &mut work.cover_click_rate,
        first_decimal_direct(&values, NOTE_DETAIL_COVER_CLICK_RATE_KEYS).map(percent_text),
    );
    apply_optional(
        &mut work.avg_view_time,
        first_decimal_direct(&values, NOTE_DETAIL_AVG_VIEW_TIME_KEYS).map(duration_text),
    );
    apply_optional(
        &mut work.gained_followers,
        first_signed_direct(&values, NOTE_DETAIL_GAINED_FOLLOWER_KEYS),
    );
    apply_optional(
        &mut work.data_updated_at,
        first_time(detail, NOTE_DETAIL_UPDATED_AT_KEYS),
    );
}

#[derive(Clone, Copy)]
enum XhsWorkCountField {
    Impressions,
    Views,
    Likes,
    Comments,
    Collects,
    Shares,
}

#[derive(Clone, Copy)]
struct XhsWorkCountSpec {
    field: XhsWorkCountField,
    keys: &'static [&'static str],
}

const XHS_WORK_COUNT_SPECS: &[XhsWorkCountSpec] = &[
    XhsWorkCountSpec {
        field: XhsWorkCountField::Impressions,
        keys: NOTE_DETAIL_IMPRESSION_KEYS,
    },
    XhsWorkCountSpec {
        field: XhsWorkCountField::Views,
        keys: NOTE_DETAIL_VIEW_KEYS,
    },
    XhsWorkCountSpec {
        field: XhsWorkCountField::Likes,
        keys: NOTE_LIKE_KEYS,
    },
    XhsWorkCountSpec {
        field: XhsWorkCountField::Comments,
        keys: NOTE_COMMENT_KEYS,
    },
    XhsWorkCountSpec {
        field: XhsWorkCountField::Collects,
        keys: NOTE_COLLECT_KEYS,
    },
    XhsWorkCountSpec {
        field: XhsWorkCountField::Shares,
        keys: NOTE_SHARE_KEYS,
    },
];

fn apply_xhs_work_count_metrics(work: &mut ChannelContentWork, values: &[Option<&Value>]) {
    for spec in XHS_WORK_COUNT_SPECS {
        let value = first_unsigned_direct(values, spec.keys);
        match spec.field {
            XhsWorkCountField::Impressions => apply_optional(&mut work.impressions, value),
            XhsWorkCountField::Views => apply_optional(&mut work.views, value),
            XhsWorkCountField::Likes => apply_optional(&mut work.likes, value),
            XhsWorkCountField::Comments => apply_optional(&mut work.comments, value),
            XhsWorkCountField::Collects => apply_optional(&mut work.collects, value),
            XhsWorkCountField::Shares => apply_optional(&mut work.shares, value),
        }
    }
}

fn apply_optional<T>(target: &mut Option<T>, value: Option<T>) {
    if value.is_some() {
        *target = value;
    }
}

fn first_time(value: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    keys.iter().find_map(|key| value.get(*key).and_then(time_from_value))
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
    if let Ok(value) = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M"))
    {
        let timezone = FixedOffset::east_opt(8 * 3600)?;
        return timezone
            .from_local_datetime(&value)
            .single()
            .map(|value| value.with_timezone(&Utc));
    }
    DateTime::parse_from_rfc3339(text)
        .map(|value| value.with_timezone(&Utc))
        .ok()
}

fn page_key_from_value(value: &Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_string());
    }
    if let Some(value) = value.as_i64() {
        return Some(value.to_string());
    }
    value.as_u64().map(|value| value.to_string())
}

fn first_unsigned_direct(values: &[Option<&Value>], keys: &[&str]) -> Option<u64> {
    values.iter().filter_map(|value| *value).find_map(|value| {
        keys.iter()
            .find_map(|key| value.get(*key).and_then(|value| parse_count_value(value, &[])))
    })
}

fn first_signed_direct(values: &[Option<&Value>], keys: &[&str]) -> Option<i64> {
    values.iter().filter_map(|value| *value).find_map(|value| {
        keys.iter().find_map(|key| {
            let value = value.get(*key)?;
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
                .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
        })
    })
}

fn first_decimal_direct(values: &[Option<&Value>], keys: &[&str]) -> Option<f64> {
    values.iter().filter_map(|value| *value).find_map(|value| {
        keys.iter().find_map(|key| decimal_value(value, key))
    })
}

fn unsigned_count(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| parse_count_value(value, &[]))
}

fn signed_count(value: &Value, key: &str) -> Option<i64> {
    let value = value.get(key)?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
}

fn decimal_value(value: &Value, key: &str) -> Option<f64> {
    let value = value.get(key)?;
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))
        .or_else(|| value.as_u64().map(|value| value as f64))
        .or_else(|| value.as_str()?.trim().parse::<f64>().ok())
}

fn trend_text(value: &Value, key: &str) -> Option<String> {
    let value = signed_count(value, key)?;
    if value == 0 {
        return Some("-".to_string());
    }
    if value > 0 {
        Some(format!("+{value}%"))
    } else {
        Some(format!("{value}%"))
    }
}

fn trend_number_text(value: f64) -> String {
    if value.abs() < f64::EPSILON {
        return "-".to_string();
    }
    let text = compact_decimal(value.abs());
    if value > 0.0 {
        format!("+{text}%")
    } else {
        format!("-{text}%")
    }
}

fn percent_text(value: f64) -> String {
    format!("{}%", compact_decimal(value))
}

fn duration_text(value: f64) -> String {
    let seconds = if value > 600.0 { value / 1000.0 } else { value };
    format!("{}秒", compact_decimal(seconds))
}

fn compact_decimal(value: f64) -> String {
    if (value.fract()).abs() < 0.05 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.1}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn number_text(value: u64) -> String {
    value.to_string()
}

fn signed_number_text(value: i64) -> String {
    value.to_string()
}

fn strip_html(value: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}
