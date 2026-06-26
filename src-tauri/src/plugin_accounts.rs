use super::*;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use std::{fs, sync::{Arc, Mutex}};
use tokio::sync::oneshot;

const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct PluginAccountInfo {
    pub(crate) uid: String,
    pub(crate) account: String,
    pub(crate) nickname: String,
    pub(crate) avatar: String,
    pub(crate) fans_count: Option<u64>,
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
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<CreatorSessionStatus, String> {
    let platform_id = normalize_platform_id(&account.platform_id);
    match platform_id.as_str() {
        "xiaohongshu" => {
            let profile = refresh_xhs_account_profile(app, account, saved_login_cookie)
                .await?
                .ok_or_else(|| "小红书登录已失效，请重新登录后再打开创作中心。".to_string())?;
            if !xhs_profile_matches_account(&profile, account) {
                return Err("当前小红书登录态不属于这个账号，请重新登录。".to_string());
            }
            let mut webview_session_id = saved_webview_session_id.map(ToString::to_string);
            if webview_session_id.is_none() {
                if let Some((session_id, _)) = find_xhs_session_for_account(app, account).await? {
                    upsert_account_webview_session(app, &account.id, &session_id)?;
                    webview_session_id = Some(session_id);
                }
            }
            Ok(CreatorSessionStatus {
                login_cookie: Some(profile.login_cookie.clone()),
                webview_session_id,
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

            let mut profile = None;
            let mut webview_session_id = saved_webview_session_id.map(ToString::to_string);
            if let Some(session_id) = webview_session_id.as_deref() {
                profile = refresh_wx_channels_account_from_task_store(app, account, session_id).await?;
            }
            if profile.is_none() {
                if let Some((session_id, found_profile)) = find_wx_channels_session_for_account(app, account).await? {
                    profile = Some(found_profile);
                    upsert_account_webview_session(app, &account.id, &session_id)?;
                    webview_session_id = Some(session_id);
                }
            }
            let Some(profile) = profile else {
                return Err("视频号登录已失效，请重新登录后再打开创作中心。".to_string());
            };
            Ok(CreatorSessionStatus {
                login_cookie: Some(profile.login_cookie.clone()),
                webview_session_id,
                profile: Some(profile),
            })
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

fn first_bilibili_mid(data: &Value) -> Option<String> {
    first_i64(data, &["mid", "uid", "id"])
        .filter(|value| *value > 0)
        .map(|value| value.to_string())
        .or_else(|| {
            first_string(data, &["mid", "uid", "id"])
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

async fn probe_bilibili_creator_session(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let value = request_plugin_json(
        "GET",
        "https://api.bilibili.com/x/web-interface/nav",
        cookie_header,
        &[
            ("Origin", "https://www.bilibili.com"),
            ("Referer", "https://www.bilibili.com/"),
        ],
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
        .and_then(|data| first_string(data, &["uname", "nickname", "name"]))
        .unwrap_or_else(|| platform_name("bilibili").to_string());
    let avatar = data
        .and_then(|data| first_profile_image(data, &["face", "avatar", "avatarUrl", "avatar_url"]))
        .unwrap_or_default();
    let avatar = materialize_account_avatar("bilibili", avatar).await;
    let account = if uid.trim().is_empty() {
        nickname.clone()
    } else {
        uid.clone()
    };
    let mut fans_count = data.and_then(|data| first_count(data, BILIBILI_FOLLOWER_COUNT_KEYS));
    if fans_count.is_none() {
        fans_count = fetch_bilibili_fans_count(cookie_header, &uid).await;
    }
    Ok(PluginAccountInfo {
        uid: account.clone(),
        account,
        nickname,
        avatar,
        fans_count,
        like_count: data.and_then(|data| first_count(data, LIKE_COUNT_KEYS)),
        login_cookie,
    })
}

async fn fetch_bilibili_fans_count(cookie_header: &str, uid: &str) -> Option<u64> {
    let uid = uid.trim();
    if uid.is_empty() || !uid.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let value = request_plugin_json(
        "GET",
        &format!("https://api.bilibili.com/x/relation/stat?vmid={uid}"),
        cookie_header,
        &[
            ("Origin", "https://www.bilibili.com"),
            ("Referer", "https://www.bilibili.com/"),
        ],
    )
    .await
    .ok()?;
    if first_i64(&value, &["code"]).unwrap_or(-1) != 0 {
        return None;
    }
    value
        .get("data")
        .and_then(|data| first_count(data, BILIBILI_FOLLOWER_COUNT_KEYS))
}

async fn refresh_xhs_account_profile(
    app: &AppHandle,
    account: &ChannelAccount,
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

    for task_id in plugin_auth_task_ids(app, "xiaohongshu")? {
        if let Some(profile) = refresh_xhs_account_from_task_store(app, account, &task_id).await? {
            return Ok(Some(profile));
        }
    }

    Err("小红书刷新需要重新授权一次账号，完成后客户端会保存登录态并支持后续刷新。".to_string())
}

async fn refresh_xhs_account_from_task_store(
    app: &AppHandle,
    account: &ChannelAccount,
    task_id: &str,
) -> Result<Option<PluginAccountInfo>, String> {
    let url = Url::parse("https://creator.xiaohongshu.com/")
        .map_err(|error| format!("小红书创作中心地址无效: {error}"))?;
    let label = format!("xhs-refresh-{}", task_suffix(task_id));
    if let Some(window) = app.get_webview_window(&label) {
        destroy_webview_window(&window);
    }
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法读取小红书授权数据目录: {error}"))?
        .join("plugin-auth")
        .join("xiaohongshu")
        .join(task_id);
    let window = WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::External(url))
        .title("刷新小红书资料 - 营销大师")
        .visible(false)
        .inner_size(360.0, 240.0)
        .data_directory(data_dir)
        .data_store_identifier(task_data_store_identifier(task_id))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .build()
        .map_err(|error| format!("读取小红书历史登录态失败: {error}"))?;
    prepare_external_webview_window(&window);

    std::thread::sleep(std::time::Duration::from_millis(450));
    let cookies = collect_webview_cookies(
        &window,
        &[
            "https://www.xiaohongshu.com/",
            "https://creator.xiaohongshu.com/",
            "https://edith.xiaohongshu.com/",
        ],
    );
    destroy_webview_window(&window);

    let (cookie_header, login_cookie) = match cookies {
        Ok(cookies) => cookies,
        Err(error) => {
            eprintln!("[refresh:xhs] cookie scan failed for {task_id}: {error}");
            return Ok(None);
        }
    };
    if cookie_header.trim().is_empty() {
        return Ok(None);
    }

    let profile =
        match fetch_xhs_plugin_account_from_cookie(&cookie_header, login_cookie, Some("creator")).await {
            Ok(profile) => profile,
            Err(error) => {
                eprintln!("[refresh:xhs] profile scan failed for {task_id}: {}", plugin_error_message(&error));
                return Ok(None);
            }
        };
    if xhs_profile_matches_account(&profile, account) {
        Ok(Some(profile))
    } else {
        Ok(None)
    }
}

async fn find_xhs_session_for_account(
    app: &AppHandle,
    account: &ChannelAccount,
) -> Result<Option<(String, PluginAccountInfo)>, String> {
    for task_id in plugin_auth_task_ids(app, "xiaohongshu")? {
        if let Some(profile) = refresh_xhs_account_from_task_store(app, account, &task_id).await? {
            return Ok(Some((task_id, profile)));
        }
    }
    Ok(None)
}

async fn find_wx_channels_session_for_account(
    app: &AppHandle,
    account: &ChannelAccount,
) -> Result<Option<(String, PluginAccountInfo)>, String> {
    for task_id in plugin_auth_task_ids(app, "wechat-channels")? {
        if let Some(profile) = refresh_wx_channels_account_from_task_store(app, account, &task_id).await? {
            return Ok(Some((task_id, profile)));
        }
    }
    Ok(None)
}

async fn refresh_wx_channels_account_from_task_store(
    app: &AppHandle,
    account: &ChannelAccount,
    task_id: &str,
) -> Result<Option<PluginAccountInfo>, String> {
    let url = creator_home_url("wechat-channels", "视频号后台")?;
    let label = format!("wx-sph-refresh-{}", task_suffix(task_id));
    if let Some(window) = app.get_webview_window(&label) {
        destroy_webview_window(&window);
    }
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法读取视频号授权数据目录: {error}"))?
        .join("plugin-auth")
        .join("wechat-channels")
        .join(task_id);
    let window = WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::External(url))
        .title("读取视频号登录态 - 营销大师")
        .visible(false)
        .inner_size(360.0, 240.0)
        .data_directory(data_dir)
        .data_store_identifier(task_data_store_identifier(task_id))
        .user_agent(DESKTOP_CHROME_UA)
        .build()
        .map_err(|error| format!("读取视频号历史登录态失败: {error}"))?;
    prepare_external_webview_window(&window);

    std::thread::sleep(std::time::Duration::from_millis(450));
    let profile = match collect_wx_sph_plugin_account(&window).await {
        Ok(profile) => Some(profile),
        Err(error) => {
            eprintln!("[refresh:wx-sph] profile scan failed for {task_id}: {}", plugin_error_message(&error));
            None
        }
    };
    destroy_webview_window(&window);

    if let Some(profile) = profile {
        if plugin_profile_matches_account(&profile, account) {
            return Ok(Some(profile));
        }
    }
    Ok(None)
}

fn plugin_auth_task_ids(app: &AppHandle, platform_id: &str) -> Result<Vec<String>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法读取授权数据目录: {error}"))?
        .join("plugin-auth")
        .join(normalize_platform_id(platform_id));
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut task_ids = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            entry.file_name().to_str().map(ToString::to_string)
        })
        .filter(|value| Uuid::parse_str(value).is_ok())
        .collect::<Vec<_>>();
    task_ids.sort();
    task_ids.reverse();
    Ok(task_ids)
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

fn xhs_profile_matches_account(profile: &PluginAccountInfo, account: &ChannelAccount) -> bool {
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

pub(crate) fn is_plugin_auth_platform(platform_id: &str) -> bool {
    channels::is_plugin_auth_platform(platform_id)
}

pub(crate) async fn collect_plugin_account_info(
    app: &AppHandle,
    platform_id: &str,
    window_label: &str,
    login_target: Option<&str>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let window = app
        .get_webview_window(window_label)
        .ok_or_else(|| PluginAuthError::NotLoggedIn("授权窗口已关闭，请重新打开并完成登录。".to_string()))?;
    match normalize_platform_id(platform_id).as_str() {
        "xiaohongshu" => Ok(collect_xhs_plugin_account(&window, login_target).await?),
        "wechat-channels" => Ok(collect_wx_sph_plugin_account(&window).await?),
        "bilibili" => Ok(collect_bilibili_plugin_account(&window).await?),
        "douyin" => Ok(collect_douyin_plugin_account(&window).await?),
        "kuaishou" => Ok(collect_kuaishou_plugin_account(&window).await?),
        _ => {
            return Err(PluginAuthError::Failed(
                "当前平台不支持插件式授权".to_string(),
            ))
        }
    }
}

async fn collect_bilibili_plugin_account(
    window: &WebviewWindow<tauri::Wry>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let (cookie_header, login_cookie) =
        collect_webview_cookies(window, channel_cookie_urls("bilibili"))
            .map_err(PluginAuthError::Failed)?;
    if cookie_header.trim().is_empty() {
        return Err(PluginAuthError::NotLoggedIn(
            "请先在打开的 B 站窗口完成登录。".to_string(),
        ));
    }
    probe_bilibili_creator_session(&cookie_header, login_cookie)
        .await
        .map_err(PluginAuthError::NotLoggedIn)
}

async fn collect_douyin_plugin_account(
    window: &WebviewWindow<tauri::Wry>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let (cookie_header, login_cookie) =
        collect_webview_cookies(window, channel_cookie_urls("douyin"))
            .map_err(PluginAuthError::Failed)?;
    if !has_douyin_login_cookie(&login_cookie) {
        return Err(PluginAuthError::NotLoggedIn(
            "请先在打开的抖音创作者中心完成登录。".to_string(),
        ));
    }
    fetch_douyin_creator_account_from_cookie(&cookie_header, login_cookie)
        .await
        .map_err(PluginAuthError::Failed)
}

async fn fetch_douyin_creator_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let headers = [
        ("Origin", "https://creator.douyin.com"),
        (
            "Referer",
            "https://creator.douyin.com/creator-micro/home?enter_from=dou_web",
        ),
    ];
    let pc_user = request_plugin_json(
        "GET",
        "https://creator.douyin.com/aweme/v1/creator/pc/user/info/",
        cookie_header,
        &headers,
    )
    .await
    .map_err(|error| format!("抖音创作者中心账号接口不可用: {error}"))?;
    if !douyin_response_success(&pc_user) {
        return Err("抖音网页登录态已失效，请重新登录后再打开创作中心。".to_string());
    }

    let user = request_plugin_json(
        "GET",
        "https://creator.douyin.com/aweme/v1/creator/user/info/",
        cookie_header,
        &headers,
    )
    .await
    .map_err(|error| format!("抖音创作者中心资料接口不可用: {error}"))?;
    if !douyin_response_success(&user) {
        return Err("抖音创作者中心资料读取失败，请重新登录后再同步。".to_string());
    }

    let verify_info = user
        .get("douyin_user_verify_info")
        .or_else(|| user.get("user_profile"));
    let uid = first_string_deep(
        &pc_user,
        &["uid", "user_id", "userId", "sec_uid", "secUid"],
    )
    .or_else(|| {
        verify_info.and_then(|value| {
            first_string_deep(
                value,
                &[
                    "uid",
                    "user_id",
                    "userId",
                    "sec_uid",
                    "secUid",
                    "douyin_unique_id",
                    "unique_id",
                    "uniqueId",
                ],
            )
        })
    })
    .or_else(|| {
        first_string_deep(
            &user,
            &[
                "uid",
                "user_id",
                "userId",
                "sec_uid",
                "secUid",
                "douyin_unique_id",
                "unique_id",
                "uniqueId",
            ],
        )
    })
    .unwrap_or_else(|| stable_label_fragment(cookie_header));
    let nickname = verify_info
        .and_then(|value| {
            first_string_deep(
                value,
                &[
                    "nick_name",
                    "nickName",
                    "nickname",
                    "name",
                    "display_name",
                    "displayName",
                ],
            )
        })
        .or_else(|| {
            first_string_deep(
                &user,
                &[
                    "nick_name",
                    "nickName",
                    "nickname",
                    "name",
                    "display_name",
                    "displayName",
                ],
            )
        })
        .unwrap_or_else(|| platform_name("douyin").to_string());
    let avatar = verify_info
        .and_then(|value| {
            first_profile_image(
                value,
                &[
                    "avatar_url",
                    "avatarUrl",
                    "avatar",
                    "avatar_thumb",
                    "avatarThumb",
                    "head_img",
                    "headImg",
                ],
            )
        })
        .or_else(|| {
            first_profile_image(
                &user,
                &[
                    "avatar_url",
                    "avatarUrl",
                    "avatar",
                    "avatar_thumb",
                    "avatarThumb",
                    "head_img",
                    "headImg",
                ],
            )
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

async fn collect_kuaishou_plugin_account(
    window: &WebviewWindow<tauri::Wry>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let (cookie_header, login_cookie) =
        collect_webview_cookies(window, channel_cookie_urls("kuaishou"))
            .map_err(PluginAuthError::Failed)?;
    if cookie_header.trim().is_empty() {
        return Err(PluginAuthError::NotLoggedIn(
            "请先在打开的快手创作者中心完成登录。".to_string(),
        ));
    }
    if has_kuaishou_creator_login_cookie_header(&cookie_header)
        && kuaishou_auth_window_should_enter_creator(window)
    {
        if let Ok(url) = creator_home_url("kuaishou", "快手创作者中心") {
            let _ = window.navigate(url);
            let _ = window.show();
            let _ = window.set_focus();
            return Err(PluginAuthError::NotLoggedIn(
                "已检测到快手登录态，正在进入快手创作者中心，请稍候。".to_string(),
            ));
        }
    }
    match request_kuaishou_creator_home_info_from_webview(window).await {
        Ok(value) => {
            return parse_kuaishou_creator_account(value, login_cookie)
                .await
                .map_err(PluginAuthError::NotLoggedIn);
        }
        Err(_) => {
            log_kuaishou_login_snapshot(window, &cookie_header).await;
        }
    }
    fetch_kuaishou_creator_account_from_cookie(&cookie_header, login_cookie)
        .await
        .map_err(PluginAuthError::NotLoggedIn)
}

fn has_kuaishou_creator_login_cookie_header(cookie_header: &str) -> bool {
    cookie_header.split(';').any(|pair| {
        let Some((name, value)) = pair.trim().split_once('=') else {
            return false;
        };
        let name = name.trim();
        !value.trim().is_empty()
            && matches!(
                name,
                "kuaishou.web.cp.api_st"
                    | "kuaishou.web.cp.api_ph"
                    | "passToken"
                    | "userId"
                    | "bUserId"
            )
    }) && cookie_header
        .split(';')
        .any(|pair| pair.trim().starts_with("kuaishou.web.cp.api_st="))
}

fn kuaishou_auth_window_should_enter_creator(window: &WebviewWindow<tauri::Wry>) -> bool {
    let Ok(url) = window.url() else {
        return true;
    };
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    !host.contains("cp.kuaishou.com")
}

async fn request_kuaishou_creator_home_info_from_webview(
    window: &WebviewWindow<tauri::Wry>,
) -> Result<Value, String> {
    let script = r#"
        (() => {
          try {
            const xhr = new XMLHttpRequest();
            xhr.open('POST', 'https://cp.kuaishou.com/rest/cp/creator/pc/home/infoV2', false);
            xhr.withCredentials = true;
            xhr.setRequestHeader('Accept', 'application/json, text/plain, */*');
            xhr.setRequestHeader('Content-Type', 'application/json;charset=utf-8');
            xhr.send('{}');
            const text = xhr.responseText || '';
            let data = null;
            try {
              data = JSON.parse(text);
            } catch (error) {
              data = { parseError: String(error), text: text.slice(0, 400) };
            }
            return { ok: xhr.status >= 200 && xhr.status < 300, status: xhr.status, url: location.href, data };
          } catch (error) {
            return { ok: false, status: 0, url: location.href, error: String(error) };
          }
        })()
    "#;
    let (sender, receiver) = oneshot::channel::<String>();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let callback_sender = Arc::clone(&sender);
    window
        .eval_with_callback(script, move |raw| {
            if let Ok(mut sender) = callback_sender.lock() {
                if let Some(sender) = sender.take() {
                    let _ = sender.send(raw);
                }
            }
        })
        .map_err(|error| format!("无法在快手登录窗口检查登录状态: {error}"))?;
    let raw = tokio::time::timeout(std::time::Duration::from_secs(10), receiver)
        .await
        .map_err(|_| "快手登录窗口状态检查超时".to_string())?
        .map_err(|_| "快手登录窗口状态检查被取消".to_string())?;
    let wrapped: Value =
        serde_json::from_str(&raw).map_err(|error| format!("快手登录窗口状态不是 JSON: {error}"))?;
    let status = first_i64(&wrapped, &["status"]).unwrap_or(0);
    let ok = wrapped
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(status >= 200 && status < 300);
    let url = wrapped.get("url").and_then(Value::as_str).unwrap_or_default();
    if !ok {
        eprintln!(
            "[plugin-auth:kuaishou] webview status={status} url={}",
            sanitize_sensitive_url(url)
        );
        return Err("请先在打开的快手创作者中心完成登录。".to_string());
    }
    wrapped
        .get("data")
        .cloned()
        .ok_or_else(|| "快手登录窗口状态缺少账号资料".to_string())
}

async fn log_kuaishou_login_snapshot(
    window: &WebviewWindow<tauri::Wry>,
    cookie_header: &str,
) {
    eprintln!(
        "[plugin-auth:kuaishou] cookie names={:?}",
        cookie_names_from_header(cookie_header)
    );
    let script = r#"
        (() => {
          const keys = (store) => {
            try {
              return Object.keys(store || {}).slice(0, 50);
            } catch (_) {
              return [];
            }
          };
          const cookieNames = (() => {
            try {
              return String(document.cookie || '')
                .split(';')
                .map((item) => item.split('=')[0].trim())
                .filter(Boolean)
                .slice(0, 50);
            } catch (_) {
              return [];
            }
          })();
          return {
            url: location.href,
            title: document.title,
            cookieNames,
            localStorageKeys: keys(localStorage),
            sessionStorageKeys: keys(sessionStorage)
          };
        })()
    "#;
    let (sender, receiver) = oneshot::channel::<String>();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let callback_sender = Arc::clone(&sender);
    if let Err(error) = window.eval_with_callback(script, move |raw| {
        if let Ok(mut sender) = callback_sender.lock() {
            if let Some(sender) = sender.take() {
                let _ = sender.send(raw);
            }
        }
    }) {
        eprintln!("[plugin-auth:kuaishou] snapshot failed: {error}");
        return;
    }
    let Ok(Ok(raw)) = tokio::time::timeout(std::time::Duration::from_secs(5), receiver).await else {
        eprintln!("[plugin-auth:kuaishou] snapshot timeout");
        return;
    };
    let Ok(mut value) = serde_json::from_str::<Value>(&raw) else {
        eprintln!("[plugin-auth:kuaishou] snapshot is not json");
        return;
    };
    if let Some(url) = value.get("url").and_then(Value::as_str).map(sanitize_sensitive_url) {
        value["url"] = Value::String(url);
    }
    eprintln!("[plugin-auth:kuaishou] snapshot={value}");
}

fn cookie_names_from_header(cookie_header: &str) -> Vec<String> {
    cookie_header
        .split(';')
        .filter_map(|pair| pair.trim().split_once('=').map(|(name, _)| name.trim()))
        .filter(|name| !name.is_empty())
        .take(50)
        .map(ToString::to_string)
        .collect()
}

fn sanitize_sensitive_url(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw) else {
        return raw.to_string();
    };
    let sensitive_keys = [
        "authToken",
        "token",
        "access_token",
        "refresh_token",
        "passToken",
        "captchaToken",
    ];
    let pairs = url
        .query_pairs()
        .map(|(key, value)| {
            let redacted = sensitive_keys
                .iter()
                .any(|item| key.eq_ignore_ascii_case(item));
            if redacted {
                (key.into_owned(), "***".to_string())
            } else {
                (key.into_owned(), value.into_owned())
            }
        })
        .collect::<Vec<_>>();
    url.set_query(None);
    if !pairs.is_empty() {
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in pairs {
                query.append_pair(&key, &value);
            }
        }
    }
    url.to_string()
}

async fn fetch_kuaishou_creator_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, String> {
    let value = request_plugin_json(
        "POST",
        "https://cp.kuaishou.com/rest/cp/creator/pc/home/infoV2",
        cookie_header,
        &[
            ("Origin", "https://cp.kuaishou.com"),
            ("Referer", "https://cp.kuaishou.com/profile"),
        ],
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
    let result = first_i64(&value, &["result", "code", "errCode", "errcode"]).unwrap_or(1);
    let uid = first_string_deep(payload, &["userKwaiId", "kwaiId", "userId", "id", "uid"])
        .or_else(|| {
            first_count(payload, &["userId", "id", "uid"])
                .filter(|value| *value > 0)
                .map(|value| value.to_string())
        })
        .unwrap_or_default();
    let nickname = first_string_deep(payload, &["userName", "nickname", "name", "displayName"])
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
        return Err("请先在打开的快手创作者中心完成登录。".to_string());
    }
    let avatar = first_profile_image(
        payload,
        &[
            "userAvatar",
            "avatar",
            "avatarUrl",
            "avatar_url",
            "headUrl",
            "head_url",
        ],
    )
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
        fans_count: first_count(payload, &["fansCnt", "fansCount", "fans", "followers"]),
        like_count: first_count(payload, &["likeCnt", "likeCount", "likes"]),
        login_cookie,
    })
}

async fn collect_xhs_plugin_account(
    window: &WebviewWindow<tauri::Wry>,
    login_target: Option<&str>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let (cookie_header, login_cookie) = collect_webview_cookies(
        window,
        &[
            "https://www.xiaohongshu.com/",
            "https://creator.xiaohongshu.com/",
            "https://edith.xiaohongshu.com/",
        ],
    )
    .map_err(PluginAuthError::Failed)?;
    let cookie_count = cookie_header
        .split(';')
        .filter(|item| !item.trim().is_empty())
        .count();
    eprintln!("[plugin-auth:xhs] cookies={cookie_count}");
    if cookie_header.trim().is_empty() {
        return Err(PluginAuthError::NotLoggedIn(match login_target {
            Some("home") => "请先在打开的小红书主页完成登录。".to_string(),
            Some("creator") => "请先在打开的小红书创作中心完成登录。".to_string(),
            _ => "请先在打开的小红书窗口完成登录。".to_string(),
        }));
    }

    fetch_xhs_plugin_account_from_cookie(&cookie_header, login_cookie, login_target).await
}

async fn fetch_xhs_plugin_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
    login_target: Option<&str>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let user_result = request_plugin_json(
        "GET",
        "https://creator.xiaohongshu.com/api/galaxy/user/info",
        cookie_header,
        &[
            ("Origin", "https://creator.xiaohongshu.com"),
            ("Referer", "https://creator.xiaohongshu.com/new/home"),
        ],
    )
    .await;
    if let Err(error) = &user_result {
        eprintln!("[plugin-auth:xhs] creator user request failed: {error}");
    }
    let edith_result = request_plugin_json(
        "GET",
        "https://edith.xiaohongshu.com/api/sns/web/v2/user/me",
        cookie_header,
        &[
            ("Origin", "https://www.xiaohongshu.com"),
            ("Referer", "https://www.xiaohongshu.com/"),
        ],
    )
    .await;
    if let Err(error) = &edith_result {
        eprintln!("[plugin-auth:xhs] edith profile request failed: {error}");
    }
    let creator_result = request_plugin_json(
        "GET",
        "https://creator.xiaohongshu.com/api/galaxy/creator/home/personal_info",
        cookie_header,
        &[
            ("Origin", "https://creator.xiaohongshu.com"),
            ("Referer", "https://creator.xiaohongshu.com/new/home"),
        ],
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
    let user_uid = user_data.and_then(|data| {
        first_string_deep(
            data,
            &[
                "user_id", "red_id", "userId", "user_id", "id", "redId", "red_id",
            ],
        )
    });
    let edith_uid = edith_data.and_then(|data| first_string_deep(data, &["user_id", "red_id", "userId", "id"]));
    let creator_uid = creator_data.and_then(|data| {
        first_string_deep(
            data,
            &[
                "user_id",
                "red_id",
                "userId",
                "id",
                "creator_id",
                "creatorId",
                "author_id",
                "authorId",
            ],
        )
    });
    let user_nickname = user_data.and_then(|data| {
        first_string_deep(
            data,
            &[
                "userName",
                "user_name",
                "nickname",
                "nickName",
                "name",
                "red_id",
                "redId",
            ],
        )
    });
    let creator_nickname = creator_data.and_then(|data| {
        first_string_deep(
            data,
            &[
                "name",
                "nickname",
                "nickName",
                "user_name",
                "userName",
                "creator_name",
                "creatorName",
            ],
        )
    });
    let user_avatar = user_data.and_then(|data| {
        first_profile_image(
            data,
            &[
                "userAvatar",
                "user_avatar",
                "avatar",
                "avatar_url",
                "avatarUrl",
                "head_img",
                "headImg",
                "headImgUrl",
            ],
        )
    });
    let creator_avatar = creator_data.and_then(|data| {
        first_profile_image(
            data,
            &[
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
            ],
        )
    });
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
            edith_data.and_then(|data| {
                first_string_deep(
                    data,
                    &["name", "nickname", "nickName", "user_name", "userName", "red_id"],
                )
            })
        })
        .unwrap_or_default();
    let avatar = creator_avatar
        .or(user_avatar)
        .or_else(|| {
            edith_data.and_then(|data| {
                first_profile_image(
                    data,
                    &[
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
                    ],
                )
            })
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

async fn collect_wx_sph_plugin_account(
    window: &WebviewWindow<tauri::Wry>,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let (cookie_header, login_cookie) = collect_webview_cookies(
        window,
        &[
            "https://channels.weixin.qq.com/",
            "https://channels.weixin.qq.com/platform",
        ],
    )
    .map_err(PluginAuthError::Failed)?;
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

async fn fetch_wx_channels_account_from_cookie(
    cookie_header: &str,
    login_cookie: String,
) -> Result<PluginAccountInfo, PluginAuthError> {
    let value = request_plugin_json(
        "POST",
        "https://channels.weixin.qq.com/cgi-bin/mmfinderassistant-bin/auth/auth_data",
        &cookie_header,
        &[
            ("Origin", "https://channels.weixin.qq.com"),
            ("Referer", "https://channels.weixin.qq.com/platform"),
            ("Content-Type", "application/json"),
        ],
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

    let uid = first_string_deep(
        finder_user,
        &[
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
        ],
    )
    .unwrap_or_default();
    let nickname = first_string_deep(
        finder_user,
        &["nickname", "nickName", "name", "displayName", "finderNickname"],
    )
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
        avatar: first_profile_image(
            finder_user,
            &[
                "headImgUrl",
                "head_img_url",
                "avatar",
                "avatarUrl",
                "avatar_url",
                "headImg",
                "image",
                "imageUrl",
            ],
        )
        .map(normalize_image_url)
        .unwrap_or_default(),
        fans_count: first_count(finder_user, FOLLOWER_COUNT_KEYS),
        like_count: first_count(finder_user, LIKE_COUNT_KEYS),
        login_cookie,
    })
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
        likes: account.like_count,
        status: AccountStatus::Active,
        relay_account_ref: None,
        token: None,
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
        request = request
            .header("Content-Type", "application/json;charset=utf-8")
            .body("{}");
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

fn xhs_response_payload(value: &Value) -> Option<&Value> {
    value
        .get("data")
        .filter(|data| !data.is_null())
        .or(Some(value))
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
    channels::platform(platform_id)
        .map(|platform| platform.materialize_avatar)
        .unwrap_or(false)
        && !value.trim().is_empty()
        && !value.trim_start().starts_with("data:image")
}

async fn materialize_account_avatar(platform_id: &str, value: String) -> String {
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
    if let Some(platform) = channels::platform(platform_id) {
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
    let value = normalize_image_url(value);
    if value.trim().is_empty() || value.starts_with("data:image") || Url::parse(&value).is_ok() {
        return value;
    }
    if normalize_platform_id(platform_id) == "xiaohongshu" {
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
