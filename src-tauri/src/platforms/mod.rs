use super::*;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use url::form_urlencoded;

mod bilibili;
mod douyin;
mod kuaishou;
mod wechat_channels;
mod xiaohongshu;

#[derive(Debug, Clone, Copy)]
pub(crate) enum HomepageKind {
    Creator,
    BilibiliSpaceOrSearch,
    KuaishouProfileOrSearch,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DomainRule {
    pub(crate) host: &'static str,
    pub(crate) include_subdomains: bool,
}

#[derive(Debug)]
pub(crate) struct ChannelPlatform {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) slug: &'static str,
    pub(crate) color: &'static str,
    pub(crate) description: &'static str,
    pub(crate) creator_home_url: &'static str,
    pub(crate) cookie_urls: &'static [&'static str],
    pub(crate) default_cookie_domain: &'static str,
    pub(crate) cookie_domains: &'static [DomainRule],
    pub(crate) login_cookie_names: &'static [&'static str],
    pub(crate) homepage_kind: HomepageKind,
    pub(crate) plugin_auth: bool,
    pub(crate) materialize_avatar: bool,
    pub(crate) avatar_referer: Option<&'static str>,
    pub(crate) avatar_origin: Option<&'static str>,
}

impl ChannelPlatform {
    pub(crate) fn homepage_url(&self, uid: &str, nickname: &str) -> Result<String, String> {
        match self.homepage_kind {
            HomepageKind::Creator => Ok(self.creator_home_url.to_string()),
            HomepageKind::BilibiliSpaceOrSearch => {
                let uid = uid.trim();
                if !uid.is_empty() && uid.chars().all(|ch| ch.is_ascii_digit()) {
                    Ok(format!("https://space.bilibili.com/{}", encode(uid)))
                } else {
                    account_search_url("https://search.bilibili.com/upuser?keyword=", nickname)
                }
            }
            HomepageKind::KuaishouProfileOrSearch => {
                let uid = uid.trim();
                if !uid.is_empty() {
                    Ok(format!("https://www.kuaishou.com/profile/{}", encode(uid)))
                } else {
                    account_search_url("https://www.kuaishou.com/search/author?searchKey=", nickname)
                }
            }
        }
    }

    pub(crate) fn allows_cookie_domain(&self, domain: &str) -> bool {
        let domain = normalize_domain(domain);
        domain.is_empty() || self.cookie_domains.iter().any(|rule| domain_matches(&domain, rule))
    }

    pub(crate) fn is_login_cookie_name(&self, name: &str) -> bool {
        let name = name.trim().to_ascii_lowercase();
        !name.is_empty() && self.login_cookie_names.iter().any(|item| item == &name)
    }
}

pub(crate) fn all() -> [&'static ChannelPlatform; 5] {
    [
        &xiaohongshu::SPEC,
        &wechat_channels::SPEC,
        &douyin::SPEC,
        &bilibili::SPEC,
        &kuaishou::SPEC,
    ]
}

pub(crate) fn platform(platform_id: &str) -> Option<&'static ChannelPlatform> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => Some(&xiaohongshu::SPEC),
        "wechat-channels" => Some(&wechat_channels::SPEC),
        "douyin" => Some(&douyin::SPEC),
        "bilibili" => Some(&bilibili::SPEC),
        "kuaishou" => Some(&kuaishou::SPEC),
        _ => None,
    }
}

pub(crate) fn normalize_platform_id(value: &str) -> String {
    match value {
        "xhs" | "Xhs" | "XHS" => "xiaohongshu".to_string(),
        "wxSph" | "wxsph" | "wechat" => "wechat-channels".to_string(),
        "kwai" | "KWAI" | "Kwai" => "kuaishou".to_string(),
        "BILIBILI" => "bilibili".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn platform_name(platform_id: &str) -> &'static str {
    platform(platform_id)
        .map(|item| item.name)
        .unwrap_or("渠道账号")
}

pub(crate) fn is_plugin_auth_platform(platform_id: &str) -> bool {
    platform(platform_id)
        .map(|item| item.plugin_auth)
        .unwrap_or(false)
}

pub(crate) fn normalize_plugin_login_target(
    platform_id: &str,
    login_target: Option<&str>,
) -> Option<&'static str> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => match login_target {
            Some("home" | "homepage") => Some("home"),
            Some("creator" | "creator-center" | "creation") => Some("creator"),
            _ => Some("creator"),
        },
        _ => None,
    }
}

pub(crate) fn plugin_login_url(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => match login_target {
            Some("home") => Some(xiaohongshu::SPEC.creator_home_url),
            _ => Some(xiaohongshu::SPEC.creator_home_url),
        },
        "wechat-channels" => Some(wechat_channels::SPEC.creator_home_url),
        "douyin" => Some(douyin::SPEC.creator_home_url),
        "bilibili" => Some(bilibili::SPEC.creator_home_url),
        "kuaishou" => Some(kuaishou::LOGIN_URL),
        _ => None,
    }
}

pub(crate) fn kuaishou_home_info_api_url() -> &'static str {
    kuaishou::HOME_INFO_API
}

pub(crate) fn kuaishou_article_manage_video_url() -> &'static str {
    kuaishou::ARTICLE_MANAGE_VIDEO_URL
}

pub(crate) fn kuaishou_article_manage_video_list_api_url() -> &'static str {
    kuaishou::ARTICLE_MANAGE_VIDEO_LIST_API
}

fn account_search_url(prefix: &str, keyword: &str) -> Result<String, String> {
    if keyword.trim().is_empty() {
        return Err("账号缺少主页标识，无法打开主页".to_string());
    }
    Ok(format!("{prefix}{}", encode(keyword.trim())))
}

fn encode(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn domain_matches(domain: &str, rule: &DomainRule) -> bool {
    let host = normalize_domain(rule.host);
    domain == host || (rule.include_subdomains && domain.ends_with(&format!(".{host}")))
}

use bilibili::{
    fetch_bilibili_account_content,
    fetch_bilibili_works_page,
    probe_bilibili_creator_session,
};
use douyin::{
    fetch_douyin_account_content,
    fetch_douyin_creator_account_from_cookie,
    fetch_douyin_works_page,
    has_douyin_login_cookie,
};
use kuaishou::{
    fetch_kuaishou_account_content,
    fetch_kuaishou_creator_account_from_cookie,
    fetch_kuaishou_works_page,
};
pub(crate) use kuaishou::{
    collect_kuaishou_plugin_account_from_browser_context,
    fetch_kuaishou_account_content_with_profile,
    has_kuaishou_creator_login_cookie_header,
    kuaishou_management_works_body,
    parse_kuaishou_management_works_page,
};
use wechat_channels::{
    fetch_wx_channels_account_content,
    fetch_wx_channels_account_from_cookie,
    fetch_wx_channels_works_page,
};
use xiaohongshu::{
    fetch_xhs_account_content,
    fetch_xhs_plugin_account_from_cookie,
    fetch_xhs_works_page,
    refresh_xhs_account_profile,
    xhs_profile_matches_account,
};

const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct PluginAccountInfo {
    pub(crate) uid: String,
    pub(crate) account: String,
    pub(crate) nickname: String,
    pub(crate) avatar: String,
    pub(crate) fans_count: Option<u64>,
    pub(crate) following_count: Option<u64>,
    pub(crate) like_count: Option<u64>,
    pub(crate) login_cookie: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CreatorSessionStatus {
    pub(crate) login_cookie: Option<String>,
    pub(crate) webview_session_id: Option<String>,
    pub(crate) profile: Option<PluginAccountInfo>,
}

#[derive(Debug)]
pub(crate) enum PluginAuthError {
    NotLoggedIn(String),
    Failed(String),
}

pub(crate) async fn check_creator_session(
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<CreatorSessionStatus, String> {
    let platform_id = normalize_platform_id(&account.platform_id);
    match platform_id.as_str() {
        "xiaohongshu" => {
            let profile = refresh_xhs_account_profile(saved_login_cookie)
                .await?
                .ok_or_else(|| "小红书登录已失效，请重新登录后再打开创作中心。".to_string())?;
            if !xhs_profile_matches_account(&profile, account) {
                return Err("当前小红书登录态不属于这个账号，请重新登录。".to_string());
            }
            Ok(CreatorSessionStatus {
                login_cookie: Some(profile.login_cookie.clone()),
                webview_session_id: saved_webview_session_id.map(ToString::to_string),
                profile: Some(profile),
            })
        }
        "wechat-channels" => {
            if let Some(login_cookie) = saved_login_cookie {
                let cookie_header = login_cookie_to_header(login_cookie);
                if !cookie_header.trim().is_empty() {
                    match fetch_wx_channels_account_from_cookie(&cookie_header, login_cookie.to_string()).await {
                        Ok(profile) if plugin_profile_matches_account(&profile, account) => {
                            return Ok(CreatorSessionStatus {
                                login_cookie: Some(profile.login_cookie.clone()),
                                webview_session_id: saved_webview_session_id.map(ToString::to_string),
                                profile: Some(profile),
                            });
                        }
                        Ok(_) => {
                            return Err("当前视频号登录态不属于这个账号，请重新登录。".to_string());
                        }
                        Err(error) => {
                            eprintln!(
                                "[creator-session:wx-sph] saved cookie probe failed: {}",
                                plugin_error_message(&error)
                            );
                        }
                    }
                }
            }
            Err("视频号登录已失效，请重新登录后再打开创作中心。".to_string())
        }
        "bilibili" => {
            let (cookie_header, login_cookie) =
                saved_cookie_header(saved_login_cookie, "B 站网页登录态已失效，请重新登录后再打开创作中心。")?;
            let profile = probe_bilibili_creator_session(&cookie_header, login_cookie).await?;
            Ok(CreatorSessionStatus {
                login_cookie: Some(profile.login_cookie.clone()),
                webview_session_id: saved_webview_session_id.map(ToString::to_string),
                profile: Some(profile),
            })
        }
        "douyin" => {
            let (cookie_header, login_cookie) =
                saved_cookie_header(saved_login_cookie, "抖音网页登录态已失效，请重新登录后再打开创作中心。")?;
            if !has_douyin_login_cookie(&login_cookie) {
                return Err("抖音网页登录态已失效，请重新登录后再打开创作中心。".to_string());
            }
            let profile = fetch_douyin_creator_account_from_cookie(&cookie_header, login_cookie.clone()).await?;
            Ok(CreatorSessionStatus {
                login_cookie: Some(login_cookie),
                webview_session_id: saved_webview_session_id.map(ToString::to_string),
                profile: Some(profile),
            })
        }
        "kuaishou" => {
            let (cookie_header, login_cookie) =
                saved_cookie_header(saved_login_cookie, "快手网页登录态已失效，请重新登录后再打开创作中心。")?;
            let profile = fetch_kuaishou_creator_account_from_cookie(&cookie_header, login_cookie.clone()).await?;
            Ok(CreatorSessionStatus {
                login_cookie: Some(login_cookie),
                webview_session_id: saved_webview_session_id.map(ToString::to_string),
                profile: Some(profile),
            })
        }
        _ => Ok(CreatorSessionStatus::default()),
    }
}

fn saved_cookie_header(saved_login_cookie: Option<&str>, expired_message: &str) -> Result<(String, String), String> {
    let login_cookie = saved_login_cookie
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| expired_message.to_string())?
        .to_string();
    let cookie_header = login_cookie_to_header(&login_cookie);
    if cookie_header.trim().is_empty() {
        return Err(expired_message.to_string());
    }
    Ok((cookie_header, login_cookie))
}

fn login_cookie_to_header(login_cookie: &str) -> String {
    let trimmed = login_cookie.trim();
    if trimmed.starts_with('[') {
        if let Ok(Value::Array(cookies)) = serde_json::from_str::<Value>(trimmed) {
            return cookies
                .iter()
                .filter_map(|cookie| {
                    Some(format!(
                        "{}={}",
                        cookie.get("name")?.as_str()?,
                        cookie.get("value")?.as_str()?
                    ))
                })
                .collect::<Vec<_>>()
                .join("; ");
        }
    }
    trimmed.to_string()
}

pub(crate) fn plugin_cookie_header(login_cookie: &str) -> String {
    login_cookie_to_header(login_cookie)
}

fn plugin_profile_matches_account(profile: &PluginAccountInfo, account: &ChannelAccount) -> bool {
    let profile_values = [&profile.uid, &profile.account, &profile.nickname]
        .into_iter()
        .map(|value| normalize_match_key(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let account_values = [
        account.uid.as_str(),
        account.nickname.as_str(),
        account.id.as_str(),
    ]
    .into_iter()
    .map(normalize_match_key)
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>();
    account_values
        .iter()
        .any(|value| profile_values.iter().any(|profile_value| profile_value == value))
}

fn plugin_profile_matches_account_strong(
    profile: &PluginAccountInfo,
    account: &ChannelAccount,
) -> bool {
    let profile_values = [&profile.uid, &profile.account]
        .into_iter()
        .map(|value| normalize_match_key(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let account_values = [
        account.uid.as_str(),
        account.id.as_str(),
    ]
    .into_iter()
    .map(normalize_match_key)
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>();
    account_values
        .iter()
        .any(|value| profile_values.iter().any(|profile_value| profile_value == value))
}

pub(crate) fn existing_plugin_account_for_profile(
    app: &AppHandle,
    user_id: &str,
    platform_id: &str,
    profile: &PluginAccountInfo,
) -> Result<Option<ChannelAccount>, String> {
    let runtime = app.state::<RuntimeState>();
    let store = runtime.store.lock().map_err(lock_error)?;
    let normalized = normalize_platform_id(platform_id);
    Ok(store
        .accounts
        .iter()
        .find(|account| {
            account_belongs_to_user(account, user_id)
                && normalize_platform_id(&account.platform_id) == normalized
                && plugin_profile_matches_account_strong(profile, account)
        })
        .cloned())
}

fn normalize_match_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn plugin_error_message(error: &PluginAuthError) -> String {
    match error {
        PluginAuthError::NotLoggedIn(message) | PluginAuthError::Failed(message) => message.clone(),
    }
}

pub(crate) async fn collect_plugin_account_info_from_cookie(
    platform_id: &str,
    cookie_header: String,
    login_cookie: String,
    login_target: Option<&str>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    if cookie_header.trim().is_empty() || login_cookie.trim().is_empty() {
        return Err(PluginAuthError::NotLoggedIn(match normalize_platform_id(platform_id).as_str() {
            "xiaohongshu" => match login_target {
                Some("home") => "请先在打开的小红书主页完成登录。".to_string(),
                _ => "请先在打开的小红书创作中心完成登录。".to_string(),
            },
            "wechat-channels" => "请先在打开的视频号窗口完成登录。".to_string(),
            "bilibili" => "请先在打开的 B 站窗口完成登录。".to_string(),
            "douyin" => "请先在打开的抖音创作者中心完成登录。".to_string(),
            "kuaishou" => "请先在打开的快手创作者中心完成登录。".to_string(),
            _ => "请先在打开的平台窗口完成登录。".to_string(),
        }));
    }

    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => fetch_xhs_plugin_account_from_cookie(&cookie_header, login_cookie, login_target).await,
        "wechat-channels" => {
            if !cookie_header
                .split(';')
                .any(|item| item.trim_start().to_ascii_lowercase().contains("sessionid="))
            {
                return Err(PluginAuthError::NotLoggedIn(
                    "请先在打开的视频号窗口完成登录。".to_string(),
                ));
            }
            fetch_wx_channels_account_from_cookie(&cookie_header, login_cookie).await
        }
        "bilibili" => probe_bilibili_creator_session(&cookie_header, login_cookie)
            .await
            .map_err(PluginAuthError::NotLoggedIn),
        "douyin" => {
            if !has_douyin_login_cookie(&login_cookie) {
                return Err(PluginAuthError::NotLoggedIn(
                    "请先在打开的抖音创作者中心完成登录。".to_string(),
                ));
            }
            fetch_douyin_creator_account_from_cookie(&cookie_header, login_cookie)
                .await
                .map_err(PluginAuthError::Failed)
        }
        "kuaishou" => {
            fetch_kuaishou_creator_account_from_cookie(&cookie_header, login_cookie)
                .await
                .map_err(PluginAuthError::NotLoggedIn)
        }
        _ => Err(PluginAuthError::Failed("当前平台不支持浏览器授权".to_string())),
    }
}

pub(crate) async fn fetch_platform_account_content(
    platform_id: &str,
    cookie_header: &str,
    login_cookie: String,
    account_id: &str,
) -> Result<ChannelAccountContent, String> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => fetch_xhs_account_content(cookie_header, login_cookie, account_id).await,
        "wechat-channels" => fetch_wx_channels_account_content(cookie_header, login_cookie, account_id).await,
        "douyin" => fetch_douyin_account_content(cookie_header, login_cookie, account_id).await,
        "bilibili" => fetch_bilibili_account_content(cookie_header, login_cookie, account_id).await,
        "kuaishou" => fetch_kuaishou_account_content(cookie_header, login_cookie, account_id).await,
        _ => Ok(ChannelAccountContent {
            account_id: account_id.to_string(),
            platform_id: normalize_platform_id(platform_id),
            sync_status: "unsupported".to_string(),
            error: Some("当前平台的数据看板暂未接入。".to_string()),
            ..Default::default()
        }),
    }
}

pub(crate) async fn fetch_platform_works_page(
    platform_id: &str,
    cookie_header: &str,
    login_cookie: &str,
    account_id: &str,
    page_key: &str,
    work_type: Option<&str>,
) -> Result<ChannelWorksPage, String> {
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => fetch_xhs_works_page(cookie_header, login_cookie, account_id, page_key).await,
        "wechat-channels" => fetch_wx_channels_works_page(cookie_header, account_id, page_key, work_type).await,
        "douyin" => fetch_douyin_works_page(cookie_header, account_id, page_key).await,
        "bilibili" => fetch_bilibili_works_page(cookie_header, account_id, page_key, work_type).await,
        "kuaishou" => fetch_kuaishou_works_page(cookie_header, account_id, page_key).await,
        _ => Ok(ChannelWorksPage {
            account_id: account_id.to_string(),
            platform_id: normalize_platform_id(platform_id),
            page_key: page_key.to_string(),
            work_type: work_type.map(ToString::to_string),
            sync_status: "unsupported".to_string(),
            error: Some("当前平台的作品列表暂未接入。".to_string()),
            ..Default::default()
        }),
    }
}

fn plugin_account_uid(account: &PluginAccountInfo) -> String {
    if account.uid.trim().is_empty() {
        account.account.clone()
    } else {
        account.uid.clone()
    }
}

pub(crate) fn plugin_info_to_channel_account(
    platform_id: &str,
    account: &PluginAccountInfo,
) -> ChannelAccount {
    let platform_id = normalize_platform_id(platform_id);
    let uid = plugin_account_uid(account);
    let id = format!(
        "{}_{}",
        platform_id,
        stable_label_fragment(&format!("{platform_id}:{uid}:{}", account.nickname))
    );
    let now = Utc::now();
    ChannelAccount {
        id,
        user_id: None,
        platform_id,
        uid,
        nickname: account.nickname.clone(),
        avatar: account.avatar.clone(),
        followers: account.fans_count,
        following: account.following_count,
        likes: account.like_count,
        status: AccountStatus::Active,
        created_at: now,
        updated_at: now,
        last_sync_at: Some(now),
    }
}

async fn request_plugin_json(
    method: &str,
    url: &str,
    cookie_header: &str,
    headers: &[(&str, &str)],
) -> Result<Value, String> {
    request_plugin_json_with_body(method, url, cookie_header, headers, None).await
}

async fn request_plugin_json_with_body(
    method: &str,
    url: &str,
    cookie_header: &str,
    headers: &[(&str, &str)],
    body: Option<Value>,
) -> Result<Value, String> {
    let client = Client::new();
    let mut request = if method.eq_ignore_ascii_case("POST") {
        client.post(url)
    } else {
        client.get(url)
    };
    request = request
        .header("Cookie", cookie_header)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .header("Accept", "application/json, text/plain, */*")
        .timeout(std::time::Duration::from_secs(18));
    if method.eq_ignore_ascii_case("POST") {
        let body = body.unwrap_or_else(|| Value::Object(Default::default()));
        request = request
            .header("Content-Type", "application/json;charset=utf-8")
            .json(&body);
    }
    for (key, value) in headers {
        request = request.header(*key, *value);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("请求平台账号资料失败: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("平台账号资料接口返回 HTTP {status}"));
    }
    response
        .json()
        .await
        .map_err(|error| format!("平台账号资料不是 JSON: {error}"))
}

fn response_success(value: &Value) -> bool {
    value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| first_i64(value, &["code", "errCode", "errcode"]).unwrap_or(0) == 0)
}

fn first_count_from_values(values: &[Option<&Value>], keys: &[&str]) -> Option<u64> {
    values.iter().find_map(|value| value.and_then(|value| first_count(value, keys)))
}

pub(crate) fn first_profile_image(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(value) = first_string(value, keys) {
        return Some(value);
    }
    match value {
        Value::Array(items) => items.iter().find_map(|item| first_profile_image(item, keys)),
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key).and_then(|value| string_from_image_value(value, keys)) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| first_profile_image(value, keys))
        }
        _ => None,
    }
}

fn string_from_image_value(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Array(items) => items.iter().find_map(|item| string_from_image_value(item, keys)),
        Value::Object(map) => {
            for key in ["url", "src", "link", "origin", "original", "large", "medium", "small", "value"] {
                if let Some(value) = map.get(key).and_then(Value::as_str) {
                    return Some(value.to_string());
                }
            }
            for key in keys {
                if let Some(value) = map.get(*key).and_then(|value| string_from_image_value(value, keys)) {
                    return Some(value);
                }
            }
            None
        }
        _ => None,
    }
}

fn should_materialize_avatar(platform_id: &str, value: &str) -> bool {
    crate::platforms::platform(platform_id)
        .map(|platform| platform.materialize_avatar)
        .unwrap_or(false)
        && !value.trim().is_empty()
        && !value.trim_start().starts_with("data:image")
}

async fn materialize_account_avatar(platform_id: &str, value: String) -> String {
    materialize_platform_image(platform_id, value).await
}

pub(crate) async fn materialize_platform_image(platform_id: &str, value: String) -> String {
    let value = normalize_platform_image_url(platform_id, value);
    if !should_materialize_avatar(platform_id, &value) {
        return value;
    }
    match fetch_avatar_data_url(platform_id, &value).await {
        Ok(data_url) => data_url,
        Err(error) => {
            eprintln!("[avatar:{}] {error}", normalize_platform_id(platform_id));
            value
        }
    }
}

async fn fetch_avatar_data_url(platform_id: &str, url: &str) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|error| format!("头像地址无效: {error}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("头像地址不是 HTTP 图片".to_string());
    }

    let mut request = Client::new()
        .get(parsed)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .header("Accept", "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8")
        .timeout(std::time::Duration::from_secs(15));
    if let Some(platform) = crate::platforms::platform(platform_id) {
        if let Some(referer) = platform.avatar_referer {
            request = request.header("Referer", referer);
        }
        if let Some(origin) = platform.avatar_origin {
            request = request.header("Origin", origin);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("头像图片请求失败: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("头像图片返回 HTTP {status}"));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("头像图片读取失败: {error}"))?;
    if bytes.is_empty() {
        return Err("头像图片为空".to_string());
    }
    if bytes.len() > MAX_AVATAR_BYTES {
        return Err("头像图片过大".to_string());
    }
    let mime = avatar_mime_type(content_type.as_deref(), bytes.as_ref());
    Ok(format!(
        "data:{mime};base64,{}",
        general_purpose::STANDARD.encode(bytes.as_ref())
    ))
}

fn avatar_mime_type(content_type: Option<&str>, bytes: &[u8]) -> String {
    if let Some(content_type) = content_type {
        let mime = content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if mime.starts_with("image/") {
            return mime;
        }
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg".to_string();
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        return "image/png".to_string();
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return "image/gif".to_string();
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return "image/webp".to_string();
    }
    "image/jpeg".to_string()
}

pub(crate) fn normalize_platform_image_url(platform_id: &str, value: String) -> String {
    let mut value = normalize_image_url(value);
    let platform_id = normalize_platform_id(platform_id);
    if matches!(platform_id.as_str(), "xiaohongshu" | "bilibili") && value.starts_with("http://") {
        value = value.replacen("http://", "https://", 1);
    }
    if value.trim().is_empty() || value.starts_with("data:image") || Url::parse(&value).is_ok() {
        return value;
    }
    if platform_id == "xiaohongshu" {
        return format!("https://img.xiaohongshu.com/{}", value.trim_start_matches('/'));
    }
    value
}

fn normalize_image_url(value: String) -> String {
    let value = value.trim().to_string();
    if value.starts_with("//") {
        format!("https:{value}")
    } else {
        value
    }
}
