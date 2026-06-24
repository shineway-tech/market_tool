use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    fs,
    io,
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
        Mutex,
    },
};
use tauri::{webview::Cookie, AppHandle, Emitter, Manager, State};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
use tauri::{WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::{form_urlencoded, Url};
use uuid::Uuid;

mod channels;
mod common;
mod http_callback;
mod json_ext;
mod local_store;
mod settings;
mod webview_windows;

use common::*;
use http_callback::*;
use json_ext::*;
use local_store::*;
use settings::*;
use webview_windows::*;

const CALLBACK_PORT_START: u16 = 17654;
const CALLBACK_PORT_END: u16 = 17674;
const RELAY_SERVER_URL: &str = "https://aitoearn.cn/api";
const DESKTOP_CHROME_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const CHANNEL_ACCOUNT_UPDATED_EVENT: &str = "channel-account-updated";
const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;
const FOLLOWER_COUNT_KEYS: &[&str] = &[
    "fans_count",
    "fansCount",
    "fans",
    "fan_count",
    "fanCount",
    "fans_num",
    "fansNum",
    "fans_number",
    "fansNumber",
    "fans_total",
    "fansTotal",
    "followers",
    "followers_count",
    "followersCount",
    "follower_count",
    "followerCount",
    "followed_count",
    "followedCount",
    "fans_count_show",
    "fansCountShow",
    "fansNumShow",
    "fans_count_str",
    "fansCountStr",
    "displayFans",
    "fansDisplay",
    "fanCountShow",
    "followersCountShow",
];
const BILIBILI_FOLLOWER_COUNT_KEYS: &[&str] = &[
    "follower",
    "fans_count",
    "fansCount",
    "fans",
    "followers",
    "followers_count",
    "followersCount",
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
    "faved_num",
    "favedNum",
    "liked_num",
    "likedNum",
    "like_num",
    "likeNum",
    "like_collect_count",
    "likeCollectCount",
    "liked_collect_count",
    "likedCollectCount",
    "like_collect_num",
    "likeCollectNum",
    "liked_collect_num",
    "likedCollectNum",
    "like_collect_number",
    "likeCollectNumber",
    "liked_collect_number",
    "likedCollectNumber",
    "like_and_collect",
    "likeAndCollect",
    "like_and_collect_count",
    "likeAndCollectCount",
    "liked_and_collected",
    "likedAndCollected",
    "liked_and_collected_count",
    "likedAndCollectedCount",
    "like_count_show",
    "likeCountShow",
    "liked_count_show",
    "likedCountShow",
    "liked_num_show",
    "likedNumShow",
    "total_liked",
    "totalLiked",
    "total_like",
    "totalLike",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfo {
    id: String,
    name: String,
    slug: String,
    color: String,
    description: String,
    supports_builtin_oauth: bool,
}

#[derive(Debug, Clone)]
struct RelaySettings {
    enabled: bool,
    server_url: String,
    api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlatformAuthSettings {
    platform_id: String,
    mode: AuthMode,
    auth_url: String,
    token_url: String,
    profile_url: String,
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthSettings {
    platforms: Vec<PlatformAuthSettings>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum AuthMode {
    Creator,
    OAuth,
}

impl<'de> Deserialize<'de> for AuthMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "creator" | "Creator" | "relay" | "Relay" => Ok(Self::Creator),
            "oAuth" | "oauth" | "OAuth" | "oauth2" | "OAuth2" => Ok(Self::OAuth),
            other => Err(serde::de::Error::unknown_variant(other, &["creator", "oAuth"])),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelAccount {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    platform_id: String,
    uid: String,
    nickname: String,
    avatar: String,
    followers: Option<u64>,
    likes: Option<u64>,
    status: AccountStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relay_account_ref: Option<String>,
    token: Option<OAuthToken>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_sync_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AccountStatus {
    Active,
    Expired,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreFile {
    accounts: Vec<ChannelAccount>,
    settings: AuthSettings,
    #[serde(default)]
    account_secrets: HashMap<String, AccountSecret>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountSecret {
    login_cookie: Option<String>,
    webview_session_id: Option<String>,
}

#[derive(Debug, Clone)]
struct PendingAuth {
    user_id: String,
    platform_id: String,
    callback_url: String,
    plugin_window_label: Option<String>,
    plugin_login_target: Option<String>,
    relay_platform_id: Option<String>,
    relay_session_id: Option<String>,
    relay_window_label: Option<String>,
    created_at: DateTime<Utc>,
}

struct RuntimeState {
    store: Mutex<StoreFile>,
    pending_auth: Mutex<HashMap<String, PendingAuth>>,
    callback_port: Mutex<Option<u16>>,
    callback_starting: AtomicBool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    platforms: Vec<PlatformInfo>,
    accounts: Vec<ChannelAccount>,
    settings: AuthSettings,
    callback_base_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartLoginResponse {
    task_id: String,
    url: String,
    callback_url: String,
    mode: AuthMode,
    auth_type: String,
    session_id: Option<String>,
    expires_at: Option<String>,
    instructions: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartLoginRequest {
    user_id: String,
    platform_id: String,
    login_target: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsRequest {
    settings: AuthSettings,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthTaskStatus {
    task_id: String,
    status: String,
    account: Option<ChannelAccount>,
    message: Option<String>,
}

#[derive(Debug)]
struct CreatorLoginSession {
    url: String,
    session_id: String,
    expires_at: Option<String>,
    instructions: Option<String>,
    auth_type: String,
}

#[derive(Debug)]
struct AitoearnAuthSession {
    url: String,
    session_id: String,
    expires_at: Option<String>,
    instructions: Option<String>,
    auth_type: String,
}

#[derive(Debug, Clone)]
struct PluginAccountInfo {
    uid: String,
    account: String,
    nickname: String,
    avatar: String,
    fans_count: Option<u64>,
    like_count: Option<u64>,
    login_cookie: String,
}

#[derive(Debug, Clone, Default)]
struct CreatorSessionStatus {
    login_cookie: Option<String>,
    webview_session_id: Option<String>,
    profile: Option<PluginAccountInfo>,
}

#[derive(Debug)]
enum PluginAuthError {
    NotLoggedIn(String),
    Failed(String),
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let store = load_store(&app.handle())?;
            app.manage(RuntimeState {
                store: Mutex::new(store),
                pending_auth: Mutex::new(HashMap::new()),
                callback_port: Mutex::new(None),
                callback_starting: AtomicBool::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_bootstrap,
            list_channel_accounts,
            save_auth_settings,
            start_channel_login,
            get_auth_task_status,
            refresh_channel_account,
            mark_channel_account_unavailable,
            open_account_homepage,
            delete_channel_account
        ])
        .run(tauri::generate_context!())
        .expect("error while running marketing master");
}

#[tauri::command]
async fn get_bootstrap(
    state: State<'_, RuntimeState>,
    user_id: String,
) -> Result<Bootstrap, String> {
    let user_id = normalize_user_id(&user_id)?;
    let store = state.store.lock().map_err(lock_error)?;
    let settings = store.settings.clone();
    let callback_base_url = state
        .callback_port
        .lock()
        .map_err(lock_error)?
        .map(|port| format!("http://127.0.0.1:{port}"));

    Ok(Bootstrap {
        platforms: default_platforms(),
        accounts: user_accounts(&store, &user_id),
        settings,
        callback_base_url,
    })
}

#[tauri::command]
async fn list_channel_accounts(
    state: State<'_, RuntimeState>,
    user_id: String,
) -> Result<Vec<ChannelAccount>, String> {
    let user_id = normalize_user_id(&user_id)?;
    let store = state.store.lock().map_err(lock_error)?;
    Ok(user_accounts(&store, &user_id))
}

#[tauri::command]
fn save_auth_settings(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: SaveSettingsRequest,
) -> Result<AuthSettings, String> {
    let mut store = state.store.lock().map_err(lock_error)?;
    store.settings = normalize_settings(request.settings);
    persist_store(&app, &store)?;
    Ok(store.settings.clone())
}

#[tauri::command]
async fn start_channel_login(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: StartLoginRequest,
) -> Result<StartLoginResponse, String> {
    let user_id = normalize_user_id(&request.user_id)?;
    let task_id = Uuid::new_v4().to_string();
    let normalized_platform_id = normalize_platform_id(&request.platform_id);
    let settings = {
        let store = state.store.lock().map_err(lock_error)?;
        store.settings.clone()
    };
    let platform_settings = settings
        .platforms
        .iter()
        .find(|item| normalize_platform_id(&item.platform_id) == normalized_platform_id)
        .cloned()
        .ok_or_else(|| "未找到平台授权参数".to_string())?;

    if normalized_platform_id == "kuaishou" {
        let relay = aitoearn_relay_settings(&app);
        let relay_platform_id = aitoearn_platform_id(&normalized_platform_id)
            .ok_or_else(|| "当前平台不在 AiToEarn 支持列表中".to_string())?;
        let session = create_aitoearn_auth_session(&relay, relay_platform_id).await?;
        let callback_url = format!(
            "aitoearn://channel-auth/relay-callback?platform={}&taskId={}",
            encode_query(&normalized_platform_id),
            encode_query(&task_id)
        );
        let mut relay_window_label = None;
        let mut auth_type = session.auth_type.clone();
        if should_open_relay_auth_in_app(&normalized_platform_id, &session.auth_type, &session.url) {
            relay_window_label = Some(open_relay_auth_window(
                &app,
                &normalized_platform_id,
                &task_id,
                &session.url,
            )?);
            auth_type = "relay".to_string();
        }

        {
            let mut pending = state.pending_auth.lock().map_err(lock_error)?;
            pending.insert(
                task_id.clone(),
                PendingAuth {
                    user_id,
                    platform_id: normalized_platform_id.clone(),
                    callback_url: callback_url.clone(),
                    plugin_window_label: None,
                    plugin_login_target: None,
                    relay_platform_id: Some(relay_platform_id.to_string()),
                    relay_session_id: Some(session.session_id.clone()),
                    relay_window_label: relay_window_label.clone(),
                    created_at: Utc::now(),
                },
            );
        }

        return Ok(StartLoginResponse {
            task_id,
            url: session.url,
            callback_url,
            mode: AuthMode::Creator,
            auth_type,
            session_id: Some(session.session_id),
            expires_at: session.expires_at,
            instructions: session.instructions,
        });
    }

    let mode = platform_settings.mode.clone();
    let callback_base = match mode {
        AuthMode::OAuth => ensure_callback_server(app.clone(), &state).await?,
        AuthMode::Creator => "creator://channel-auth".to_string(),
    };
    let callback_path = match mode {
        AuthMode::OAuth => "oauth-callback",
        AuthMode::Creator => "creator-callback",
    };
    let callback_url = format!(
        "{callback_base}/{callback_path}?platform={}&taskId={}",
        encode_query(&request.platform_id),
        encode_query(&task_id)
    );

    let mut plugin_window_label = None;
    let mut plugin_login_target = None;
    let mut expires_at = None;
    let mut instructions = None;
    let mut auth_type = "oauth".to_string();

    let auth_url = match mode {
        AuthMode::Creator => {
            if !is_plugin_auth_platform(&platform_settings.platform_id) {
                return Err("当前平台暂不支持创作中心登录".to_string());
            }
            let target = normalize_plugin_login_target(
                &platform_settings.platform_id,
                request.login_target.as_deref(),
            );
            let session =
                open_plugin_login_window(&app, &platform_settings.platform_id, &task_id, target)?;
            plugin_login_target = target.map(ToString::to_string);
            plugin_window_label = Some(session.session_id.clone());
            expires_at = session.expires_at.clone();
            instructions = session.instructions.clone();
            auth_type = session.auth_type.clone();
            session.url
        }
        AuthMode::OAuth => create_direct_oauth_url(&platform_settings, &callback_url, &task_id)?,
    };

    {
        let mut pending = state.pending_auth.lock().map_err(lock_error)?;
        pending.insert(
            task_id.clone(),
            PendingAuth {
                user_id,
                platform_id: request.platform_id.clone(),
                callback_url: callback_url.clone(),
                plugin_window_label: plugin_window_label.clone(),
                plugin_login_target: plugin_login_target.clone(),
                relay_platform_id: None,
                relay_session_id: None,
                relay_window_label: None,
                created_at: Utc::now(),
            },
        );
    }

    if auth_type == "oauth" && !auth_url.starts_with("data:image") {
        open_external_url(&auth_url)?;
    }

    Ok(StartLoginResponse {
        task_id,
        url: auth_url,
        callback_url,
        mode,
        auth_type,
        session_id: plugin_window_label,
        expires_at,
        instructions,
    })
}

#[tauri::command]
async fn get_auth_task_status(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    task_id: String,
    user_id: String,
) -> Result<AuthTaskStatus, String> {
    let user_id = normalize_user_id(&user_id)?;
    let pending_task = state
        .pending_auth
        .lock()
        .map_err(lock_error)?
        .get(&task_id)
        .filter(|task| task.user_id == user_id)
        .cloned();

    if let Some(task) = pending_task.clone() {
        if let Some(window_label) = task.plugin_window_label.clone() {
            match collect_plugin_account_info(
                &app,
                &task.platform_id,
                &window_label,
                task.plugin_login_target.as_deref(),
            )
            .await
            {
                Ok(profile) => {
                    if let Some(existing) =
                        existing_plugin_account_for_profile(&app, &task.user_id, &task.platform_id, &profile)?
                    {
                        if !matches!(existing.status, AccountStatus::Active) {
                            let account =
                                update_plugin_account_profile(&app, &task.user_id, &existing.id, &profile)?;
                            let _ = upsert_account_webview_session(&app, &account.id, &task_id);
                            let _ = app
                                .get_webview_window(&window_label)
                                .map(|window| window.close());
                            state
                                .pending_auth
                                .lock()
                                .map_err(lock_error)?
                                .remove(&task_id);
                            return Ok(AuthTaskStatus {
                                task_id,
                                status: "success".to_string(),
                                account: Some(account),
                                message: None,
                            });
                        }
                        return Ok(AuthTaskStatus {
                            task_id,
                            status: "pending".to_string(),
                            account: None,
                            message: Some(format!(
                                "当前窗口登录的是已添加账号「{}」。如需添加第二个账号，请在该窗口退出当前账号后再登录新账号。",
                                existing.nickname
                            )),
                        });
                    }

                    let account = plugin_info_to_channel_account(&task.platform_id, &profile);
                    let account = upsert_account_for_user(&app, &task.user_id, account)?;
                    upsert_account_secret(&app, &account.id, &profile.login_cookie)?;
                    let _ = upsert_account_webview_session(&app, &account.id, &task_id);
                    let _ = app
                        .get_webview_window(&window_label)
                        .map(|window| window.close());
                    state
                        .pending_auth
                        .lock()
                        .map_err(lock_error)?
                        .remove(&task_id);
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "success".to_string(),
                        account: Some(account),
                        message: None,
                    });
                }
                Err(PluginAuthError::NotLoggedIn(message)) => {
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "pending".to_string(),
                        account: None,
                        message: Some(message),
                    });
                }
                Err(PluginAuthError::Failed(message)) => {
                    let _ = app
                        .get_webview_window(&window_label)
                        .map(|window| window.close());
                    state
                        .pending_auth
                        .lock()
                        .map_err(lock_error)?
                        .remove(&task_id);
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "failed".to_string(),
                        account: None,
                        message: Some(message),
                    });
                }
            }
        }
    }

    if let Some(task) = pending_task.clone() {
        if let (Some(relay_platform_id), Some(session_id)) =
            (task.relay_platform_id.clone(), task.relay_session_id.clone())
        {
            let relay = aitoearn_relay_settings(&app);
            let status = fetch_aitoearn_auth_status(&relay, &relay_platform_id, &session_id)
                .await?;
            let remote_status = first_string(&status, &["status"]).unwrap_or_default();
            if matches!(remote_status.as_str(), "completed" | "success") {
                let synced = fetch_aitoearn_accounts(&relay).await?;
                let account = find_synced_auth_account(&synced, &task.platform_id, &status)
                    .or_else(|| {
                        synced
                            .iter()
                            .find(|item| {
                                normalize_platform_id(&item.platform_id) == normalize_platform_id(&task.platform_id)
                                    && item.created_at >= task.created_at
                            })
                            .cloned()
                    })
                    .or_else(|| {
                        synced
                            .iter()
                            .filter(|item| normalize_platform_id(&item.platform_id) == normalize_platform_id(&task.platform_id))
                            .max_by_key(|item| item.created_at)
                            .cloned()
                    })
                    .ok_or_else(|| "快手授权已完成，但没有同步到账号信息，请重新授权。".to_string())?;
                let account = upsert_account_for_user(&app, &task.user_id, account)?;
                let login_cookie = capture_auth_window_cookies_any(
                    &app,
                    task.relay_window_label.as_deref(),
                    channel_cookie_urls("kuaishou"),
                );
                if let Some(login_cookie) = login_cookie.as_deref() {
                    upsert_account_secret(&app, &account.id, login_cookie)?;
                }
                let _ = upsert_account_webview_session(&app, &account.id, &task_id);
                state
                    .pending_auth
                    .lock()
                    .map_err(lock_error)?
                    .remove(&task_id);
                close_auth_window_by_label(&app, task.relay_window_label.as_deref());
                return Ok(AuthTaskStatus {
                    task_id,
                    status: "success".to_string(),
                    account: Some(account),
                    message: None,
                });
            }

            if matches!(remote_status.as_str(), "failed" | "expired" | "timeout") {
                let message = first_string(&status, &["message", "reason"])
                    .unwrap_or_else(|| "快手授权没有完成，请重新尝试。".to_string());
                state
                    .pending_auth
                    .lock()
                    .map_err(lock_error)?
                    .remove(&task_id);
                close_auth_window_by_label(&app, task.relay_window_label.as_deref());
                return Ok(AuthTaskStatus {
                    task_id,
                    status: "failed".to_string(),
                    account: None,
                    message: Some(message),
                });
            }

            return Ok(AuthTaskStatus {
                task_id,
                status: "pending".to_string(),
                account: None,
                message: first_string(&status, &["message", "reason"])
                    .or_else(|| Some("请在打开的快手授权窗口中完成登录。".to_string())),
            });
        }
    }

    let store = state.store.lock().map_err(lock_error)?;
    let accounts = user_accounts(&store, &user_id);
    let account = accounts
        .iter()
        .find(|item| {
            item.id == task_id
                || pending_task
                    .as_ref()
                    .map(|task| item.platform_id == task.platform_id && item.created_at >= task.created_at)
                    .unwrap_or(false)
        })
        .cloned();

    let status = if account.is_some() {
        "success"
    } else if pending_task.is_some() {
        "pending"
    } else {
        "unknown"
    };

    Ok(AuthTaskStatus {
        task_id,
        status: status.to_string(),
        account,
        message: None,
    })
}

#[tauri::command]
async fn refresh_channel_account(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(&user_id)?;
    let existing = {
        let store = state.store.lock().map_err(lock_error)?;
        store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .cloned()
    };
    let account = existing
        .as_ref()
        .ok_or_else(|| "账号不存在".to_string())?;
    if normalize_platform_id(&account.platform_id) == "kuaishou" {
        let relay = aitoearn_relay_settings(&app);
        let refreshed = refresh_aitoearn_channel_account(&relay, account).await?;
        return upsert_account_for_user(&app, &user_id, refreshed);
    }
    let (secret_cookie, secret_webview_session_id) = {
        let mut store = state.store.lock().map_err(lock_error)?;
        let migrated = migrate_account_secret_for_account(&mut store, account);
        let value = (
            account_secret_for_account(&store, account).and_then(|secret| secret.login_cookie),
            account_secret_for_account(&store, account).and_then(|secret| secret.webview_session_id),
        );
        if migrated {
            persist_store(&app, &store)?;
        }
        value
    };
    let creator_status = match check_creator_session(
        &app,
        account,
        secret_cookie.as_deref(),
        secret_webview_session_id.as_deref(),
    )
    .await
    {
        Ok(status) => status,
        Err(error) => {
            let _ = mark_account_expired(&app, &account.id);
            return Err(error);
        }
    };

    let mut store = state.store.lock().map_err(lock_error)?;
    if creator_status.login_cookie.is_some() || creator_status.webview_session_id.is_some() {
        let secret = store.account_secrets.entry(account_id.clone()).or_default();
        if let Some(login_cookie) = creator_status.login_cookie.as_ref() {
            if !login_cookie.trim().is_empty() {
                secret.login_cookie = Some(login_cookie.clone());
            }
        }
        if let Some(webview_session_id) = creator_status.webview_session_id.as_ref() {
            if !webview_session_id.trim().is_empty() {
                secret.webview_session_id = Some(webview_session_id.clone());
            }
        }
    }
    let account = store
        .accounts
        .iter_mut()
        .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
        .ok_or_else(|| "账号不存在".to_string())?;
    if let Some(profile) = creator_status.profile.as_ref() {
        if !profile.nickname.trim().is_empty() {
            account.nickname = profile.nickname.clone();
        }
        if !profile.avatar.trim().is_empty() {
            account.avatar = profile.avatar.clone();
        }
        if let Some(fans_count) = profile.fans_count {
            account.followers = Some(fans_count);
        }
        if let Some(like_count) = profile.like_count {
            account.likes = Some(like_count);
        }
        if account.uid.trim().is_empty() || normalize_platform_id(&account.platform_id) == "douyin" {
            account.uid = profile.uid.clone();
        }
    }
    account.status = AccountStatus::Active;
    account.last_sync_at = Some(Utc::now());
    account.updated_at = Utc::now();
    let cloned = account.clone();
    persist_store(&app, &store)?;
    Ok(cloned)
}

#[tauri::command]
async fn mark_channel_account_unavailable(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(&user_id)?;
    {
        let store = state.store.lock().map_err(lock_error)?;
        store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .ok_or_else(|| "账号不存在".to_string())?;
    }
    mark_account_expired(&app, &account_id)
}

#[tauri::command]
async fn open_account_homepage(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(&user_id)?;
    let (mut account, saved_login_cookie, saved_webview_session_id) = {
        let mut store = state.store.lock().map_err(lock_error)?;
        let account = store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .cloned()
            .ok_or_else(|| "账号不存在".to_string())?;
        let migrated = migrate_account_secret_for_account(&mut store, &account);
        let secret = account_secret_for_account(&store, &account);
        let saved_login_cookie = secret.as_ref().and_then(|secret| secret.login_cookie.clone());
        let saved_webview_session_id = secret.and_then(|secret| secret.webview_session_id);
        if migrated {
            persist_store(&app, &store)?;
        }
        Ok::<_, String>((account, saved_login_cookie, saved_webview_session_id))
    }
    ?;

    if normalize_platform_id(&account.platform_id) == "xiaohongshu" {
        open_xhs_creator_webview(
            &app,
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        )?;
        spawn_creator_session_check(
            app.clone(),
            user_id,
            account.clone(),
            saved_login_cookie,
            saved_webview_session_id,
        );
        return Ok(account);
    }

    if normalize_platform_id(&account.platform_id) == "kuaishou" {
        open_kuaishou_creator_webview(
            &app,
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        )?;
        return Ok(account);
    }

    let creator_status = match check_creator_session(
        &app,
        &account,
        saved_login_cookie.as_deref(),
        saved_webview_session_id.as_deref(),
    )
    .await
    {
        Ok(status) => status,
        Err(error) => {
            let _ = mark_account_expired(&app, &account.id);
            return Err(error);
        }
    };
    if let Some(profile) = creator_status.profile.as_ref() {
        account = update_plugin_account_profile(&app, &user_id, &account.id, profile)?;
    }
    let login_cookie = creator_status
        .login_cookie
        .as_deref()
        .or(saved_login_cookie.as_deref());
    let webview_session_id = creator_status
        .webview_session_id
        .as_deref()
        .or(saved_webview_session_id.as_deref());

    match normalize_platform_id(&account.platform_id).as_str() {
        "douyin" => {
            open_douyin_creator_webview(&app, &account, login_cookie, webview_session_id)?;
            Ok(account)
        }
        "xiaohongshu" => {
            open_xhs_creator_webview(&app, &account, login_cookie, webview_session_id)?;
            Ok(account)
        }
        "wechat-channels" => {
            open_wx_channels_webview(&app, &account, login_cookie, webview_session_id)?;
            Ok(account)
        }
        "bilibili" => {
            open_bilibili_creator_webview(&app, &account, login_cookie, webview_session_id)?;
            Ok(account)
        }
        "kuaishou" => {
            open_kuaishou_creator_webview(&app, &account, login_cookie, webview_session_id)?;
            Ok(account)
        }
        _ => {
            let url = account_homepage_url(&account)?;
            open_external_url(&url)?;
            Ok(account)
        }
    }
}

fn spawn_creator_session_check(
    app: AppHandle,
    user_id: String,
    account: ChannelAccount,
    saved_login_cookie: Option<String>,
    saved_webview_session_id: Option<String>,
) {
    tauri::async_runtime::spawn(async move {
        match check_creator_session(
            &app,
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        )
        .await
        {
            Ok(status) => {
                if let Some(profile) = status.profile.as_ref() {
                    if let Err(error) =
                        update_plugin_account_profile(&app, &user_id, &account.id, profile)
                    {
                        eprintln!(
                            "[creator-session:{}] profile update failed for {}: {error}",
                            account.platform_id, account.id
                        );
                    }
                }
                if let Some(login_cookie) = status.login_cookie.as_ref() {
                    if let Err(error) = upsert_account_secret(&app, &account.id, login_cookie) {
                        eprintln!(
                            "[creator-session:{}] cookie update failed for {}: {error}",
                            account.platform_id, account.id
                        );
                    }
                }
                if let Some(webview_session_id) = status.webview_session_id.as_ref() {
                    if let Err(error) =
                        upsert_account_webview_session(&app, &account.id, webview_session_id)
                    {
                        eprintln!(
                            "[creator-session:{}] session update failed for {}: {error}",
                            account.platform_id, account.id
                        );
                    }
                }
            }
            Err(error) => {
                eprintln!(
                    "[creator-session:{}] background check failed for {}: {error}",
                    account.platform_id, account.id
                );
                let _ = mark_account_expired(&app, &account.id);
            }
        }
    });
}

#[tauri::command]
async fn delete_channel_account(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<(), String> {
    let user_id = normalize_user_id(&user_id)?;
    let account = {
        let store = state.store.lock().map_err(lock_error)?;
        store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .cloned()
    };
    if let Some(account) = account.as_ref() {
        if normalize_platform_id(&account.platform_id) == "kuaishou" {
            let relay = aitoearn_relay_settings(&app);
            let _ = delete_aitoearn_account(&relay, account).await;
        }
    }
    let mut store = state.store.lock().map_err(lock_error)?;
    let original_len = store.accounts.len();
    store
        .accounts
        .retain(|item| !(item.id == account_id && account_belongs_to_user(item, &user_id)));
    if store.accounts.len() == original_len {
        return Err("账号不存在".to_string());
    }
    for secret_key in account
        .as_ref()
        .map(account_secret_candidates)
        .unwrap_or_default()
    {
        store.account_secrets.remove(&secret_key);
    }
    persist_store(&app, &store)?;
    Ok(())
}

fn aitoearn_relay_settings(app: &AppHandle) -> RelaySettings {
    let mut server_url = env::var("CHANNEL_NEST_RELAY_SERVER_URL")
        .or_else(|_| env::var("MARKETING_MASTER_RELAY_SERVER_URL"))
        .or_else(|_| env::var("AITOEARN_RELAY_SERVER_URL"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| RELAY_SERVER_URL.to_string());
    let mut api_key = env::var("CHANNEL_NEST_RELAY_API_KEY")
        .or_else(|_| env::var("MARKETING_MASTER_RELAY_API_KEY"))
        .or_else(|_| env::var("AITOEARN_RELAY_API_KEY"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| option_env!("CHANNEL_NEST_RELAY_API_KEY").map(ToString::to_string))
        .or_else(|| option_env!("MARKETING_MASTER_RELAY_API_KEY").map(ToString::to_string))
        .or_else(|| option_env!("AITOEARN_RELAY_API_KEY").map(ToString::to_string))
        .unwrap_or_default();

    for path in relay_config_candidates(app) {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            if let Some(value) = json_config_string(&value, &["serverUrl", "server_url", "baseUrl", "base_url"]) {
                server_url = value;
            }
            if api_key.trim().is_empty() {
                if let Some(value) = json_config_string(&value, &["apiKey", "api_key", "relayApiKey", "relay_api_key"]) {
                    api_key = value;
                }
            }
            continue;
        }
        if let Some(value) = env_config_string(
            &text,
            &[
                "CHANNEL_NEST_RELAY_SERVER_URL",
                "MARKETING_MASTER_RELAY_SERVER_URL",
                "AITOEARN_RELAY_SERVER_URL",
            ],
        ) {
            server_url = value;
        }
        if api_key.trim().is_empty() {
            if let Some(value) = env_config_string(
                &text,
                &[
                    "CHANNEL_NEST_RELAY_API_KEY",
                    "MARKETING_MASTER_RELAY_API_KEY",
                    "AITOEARN_RELAY_API_KEY",
                    "RELAY_API_KEY",
                ],
            ) {
                api_key = value;
            }
        }
    }

    RelaySettings {
        enabled: !api_key.trim().is_empty(),
        server_url,
        api_key,
    }
}

fn relay_config_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(current) = env::current_dir() {
        let mut cursor = Some(current.as_path());
        for _ in 0..4 {
            if let Some(path) = cursor {
                push_unique_path(&mut dirs, path.to_path_buf());
                cursor = path.parent();
            }
        }
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        push_unique_path(&mut dirs, resource_dir);
    }
    if let Ok(app_config_dir) = app.path().app_config_dir() {
        push_unique_path(&mut dirs, app_config_dir);
    }

    let mut files = Vec::new();
    for dir in dirs {
        for relative in [
            ".secret/aitoearn-relay.json",
            ".secrets/aitoearn-relay.json",
            ".secret/local-build.env",
            ".secrets/local-build.env",
            "aitoearn-relay.json",
        ] {
            push_unique_path(&mut files, dir.join(relative));
        }
    }
    files
}

fn push_unique_path(values: &mut Vec<PathBuf>, value: PathBuf) {
    if !values.iter().any(|item| item == &value) {
        values.push(value);
    }
}

fn json_config_string(value: &Value, keys: &[&str]) -> Option<String> {
    first_string(value, keys)
        .map(|value| trim_config_value(&value))
        .filter(|value| !value.is_empty())
}

fn env_config_string(text: &str, keys: &[&str]) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if keys.iter().any(|candidate| candidate == &key.trim()) {
            let value = trim_config_value(value);
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn trim_config_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

async fn create_aitoearn_auth_session(
    relay: &RelaySettings,
    relay_platform_id: &str,
) -> Result<AitoearnAuthSession, String> {
    if !relay.enabled || relay.server_url.trim().is_empty() || relay.api_key.trim().is_empty() {
        return Err("AiToEarn relay 参数不可用，请在 .secret/aitoearn-relay.json 或 .secrets/local-build.env 中配置 API Key。".to_string());
    }

    let group_id = default_aitoearn_group_id(relay).await.unwrap_or(None);
    let mut params = Vec::new();
    if let Some(group_id) = group_id.as_deref() {
        params.push(("groupId", group_id));
    }
    let value = aitoearn_get(
        relay,
        &format!("v2/channels/accounts/auth/{relay_platform_id}"),
        &params,
    )
    .await?;
    ensure_aitoearn_success(&value)?;

    let data = relay_response_data(&value);
    let url = first_string(data, &["url", "uri"])
        .ok_or_else(|| "授权响应缺少授权 URL".to_string())?;
    let session_id = first_string(data, &["sessionId", "session_id"])
        .ok_or_else(|| "授权响应缺少 sessionId".to_string())?;
    let auth_type = if url.starts_with("data:image") {
        "qrcode"
    } else {
        "oauth"
    }
    .to_string();
    let instructions = data
        .get("authInstructions")
        .and_then(|value| first_string(value, &["zh-CN", "zh", "en-US", "en"]))
        .or_else(|| Some("请在打开的快手授权窗口中完成登录。".to_string()));

    Ok(AitoearnAuthSession {
        url,
        session_id,
        expires_at: first_string(data, &["expiresAt", "expires_at"]),
        instructions,
        auth_type,
    })
}

async fn fetch_aitoearn_auth_status(
    relay: &RelaySettings,
    relay_platform_id: &str,
    session_id: &str,
) -> Result<Value, String> {
    let value = aitoearn_get(
        relay,
        &format!(
            "v2/channels/accounts/auth/{}/status/{}",
            encode_path_segment(relay_platform_id),
            encode_path_segment(session_id)
        ),
        &[],
    )
    .await?;
    ensure_aitoearn_success(&value)?;
    Ok(relay_response_data(&value).clone())
}

async fn fetch_aitoearn_accounts(relay: &RelaySettings) -> Result<Vec<ChannelAccount>, String> {
    let value = aitoearn_get(relay, "v2/channels/accounts", &[]).await?;
    ensure_aitoearn_success(&value)?;
    let data = relay_response_data(&value);
    let list = data
        .get("list")
        .and_then(Value::as_array)
        .or_else(|| data.as_array())
        .ok_or_else(|| "AiToEarn 账号列表响应格式无效".to_string())?;

    let mut accounts = Vec::new();
    for item in list {
        if let Some(account) = aitoearn_account_from_value(item) {
            if normalize_platform_id(&account.platform_id) == "kuaishou" {
                accounts.push(account);
            }
        }
    }
    Ok(accounts)
}

async fn refresh_aitoearn_channel_account(
    relay: &RelaySettings,
    account: &ChannelAccount,
) -> Result<ChannelAccount, String> {
    let accounts = fetch_aitoearn_accounts(relay).await?;
    let mut refreshed = accounts
        .into_iter()
        .find(|item| {
            same_relay_account(item, account)
                || (!account.uid.trim().is_empty() && item.uid == account.uid)
        })
        .ok_or_else(|| "快手 relay 账号不存在或授权已失效，请重新登录。".to_string())?;

    if let Ok(Some(followers)) = refresh_aitoearn_account_analytics(relay, account).await {
        refreshed.followers = Some(followers);
    }
    refreshed.user_id = account.user_id.clone();
    refreshed.created_at = account.created_at;
    refreshed.updated_at = Utc::now();
    refreshed.last_sync_at = Some(Utc::now());
    Ok(refreshed)
}

fn same_relay_account(left: &ChannelAccount, right: &ChannelAccount) -> bool {
    match (
        left.relay_account_ref.as_deref(),
        right.relay_account_ref.as_deref(),
    ) {
        (Some(left), Some(right)) if left == right => true,
        _ => false,
    }
}

async fn refresh_aitoearn_account_analytics(
    relay: &RelaySettings,
    account: &ChannelAccount,
) -> Result<Option<u64>, String> {
    let Some(relay_account_id) = account.relay_account_ref.as_deref() else {
        return Ok(None);
    };
    let value = aitoearn_get(
        relay,
        &format!(
            "v2/channels/accounts/{}/analytics",
            encode_path_segment(relay_account_id)
        ),
        &[],
    )
    .await?;
    ensure_aitoearn_success(&value)?;
    let data = relay_response_data(&value);
    Ok(first_count(data, FOLLOWER_COUNT_KEYS).or_else(|| first_count(&value, FOLLOWER_COUNT_KEYS)))
}

async fn delete_aitoearn_account(
    relay: &RelaySettings,
    account: &ChannelAccount,
) -> Result<(), String> {
    let Some(relay_account_id) = account.relay_account_ref.as_deref() else {
        return Ok(());
    };
    let value = aitoearn_delete(
        relay,
        &format!("v2/channels/accounts/{}", encode_path_segment(relay_account_id)),
    )
    .await?;
    ensure_aitoearn_success(&value)?;
    Ok(())
}

async fn default_aitoearn_group_id(relay: &RelaySettings) -> Result<Option<String>, String> {
    let value = aitoearn_get(relay, "v2/channels/account-groups", &[]).await?;
    ensure_aitoearn_success(&value)?;
    let data = relay_response_data(&value);
    let Some(groups) = data.as_array() else {
        return Ok(None);
    };
    let default_group = groups
        .iter()
        .find(|item| item.get("isDefault").and_then(Value::as_bool) == Some(true))
        .or_else(|| groups.first());
    Ok(default_group.and_then(|item| first_string(item, &["id"])))
}

async fn aitoearn_get(
    relay: &RelaySettings,
    path: &str,
    params: &[(&str, &str)],
) -> Result<Value, String> {
    let mut url = relay_url(relay, path)?;
    for (key, value) in params {
        url.query_pairs_mut().append_pair(key, value);
    }
    aitoearn_request(Client::new().get(url), relay).await
}

async fn aitoearn_delete(relay: &RelaySettings, path: &str) -> Result<Value, String> {
    let url = relay_url(relay, path)?;
    aitoearn_request(Client::new().delete(url), relay).await
}

async fn aitoearn_request(
    request: reqwest::RequestBuilder,
    relay: &RelaySettings,
) -> Result<Value, String> {
    let response = request
        .header("x-api-key", relay.api_key.trim())
        .header("Accept-Language", "zh-CN")
        .timeout(std::time::Duration::from_secs(18))
        .send()
        .await
        .map_err(|error| format!("请求 AiToEarn relay 失败: {error}"))?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|error| format!("AiToEarn relay 返回不是 JSON: {error}"))?;
    if !status.is_success() {
        return Err(format!("AiToEarn relay 返回 HTTP {status}: {}", relay_error_message(&value)));
    }
    Ok(value)
}

fn relay_url(relay: &RelaySettings, path: &str) -> Result<Url, String> {
    let base = relay.server_url.trim().trim_end_matches('/');
    let path = path.trim().trim_start_matches('/');
    Url::parse(&format!("{base}/{path}")).map_err(|error| format!("AiToEarn relay 地址无效: {error}"))
}

fn ensure_aitoearn_success(value: &Value) -> Result<(), String> {
    if let Some(code) = value.get("code").and_then(Value::as_i64) {
        if code != 0 {
            return Err(match code {
                401 | 403 => "AiToEarn relay API Key 认证失败".to_string(),
                _ => relay_error_message(value),
            });
        }
    }
    Ok(())
}

fn relay_response_data(value: &Value) -> &Value {
    value.get("data").unwrap_or(value)
}

fn aitoearn_account_from_value(value: &Value) -> Option<ChannelAccount> {
    let remote_platform = first_string(value, &["type", "platform", "platformId", "platform_id"])?;
    let platform_id = normalize_platform_id(&remote_platform);
    let id = first_string(value, &["id", "accountId", "account_id"])
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let uid = first_string(value, &["uid", "platformUid", "platform_uid", "openId", "open_id"])
        .unwrap_or_else(|| id.clone());
    let nickname = first_string(value, &["nickname", "name", "displayName", "display_name"])
        .unwrap_or_else(|| platform_name(&platform_id).to_string());
    let avatar = first_profile_image(
        value,
        &[
            "avatar",
            "avatarUrl",
            "avatar_url",
            "headImg",
            "headImgUrl",
            "head_img",
            "profileImageUrl",
            "profile_image_url",
            "image",
            "imageUrl",
            "image_url",
        ],
    )
    .map(|value| normalize_platform_image_url(&platform_id, value))
    .unwrap_or_default();
    let status = relay_account_status(value);
    let created_at = first_string(value, &["createdAt", "created_at"])
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let updated_at = first_string(value, &["updatedAt", "updated_at"])
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let last_sync_at = first_string(value, &["lastStatsTime", "lastSyncAt", "last_sync_at", "updatedAt", "updated_at"])
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc))
        .or(Some(updated_at));

    Some(ChannelAccount {
        id: id.clone(),
        user_id: None,
        platform_id,
        uid,
        nickname,
        avatar,
        followers: first_count(value, FOLLOWER_COUNT_KEYS),
        likes: first_count(value, LIKE_COUNT_KEYS),
        status,
        relay_account_ref: Some(id),
        token: None,
        created_at,
        updated_at,
        last_sync_at,
    })
}

fn relay_account_status(value: &Value) -> AccountStatus {
    if let Some(status) = value.get("status").and_then(Value::as_i64) {
        return match status {
            1 => AccountStatus::Active,
            0 => AccountStatus::Pending,
            _ => AccountStatus::Expired,
        };
    }
    let status = first_string(value, &["status", "state"])
        .unwrap_or_else(|| "active".to_string())
        .to_ascii_lowercase();
    match status.as_str() {
        "active" | "enabled" | "success" | "completed" | "1" => AccountStatus::Active,
        "pending" | "processing" | "0" => AccountStatus::Pending,
        _ => AccountStatus::Expired,
    }
}

fn find_synced_auth_account(
    accounts: &[ChannelAccount],
    platform_id: &str,
    status: &Value,
) -> Option<ChannelAccount> {
    let mut ids = Vec::new();
    if let Some(id) = first_string(status, &["accountId", "account_id", "id"]) {
        ids.push(id);
    }
    if let Some(values) = status.get("accountIds").and_then(Value::as_array) {
        ids.extend(values.iter().filter_map(Value::as_str).map(ToString::to_string));
    }
    if let Some(values) = status.get("accounts").and_then(Value::as_array) {
        for item in values {
            if let Some(id) = first_string(item, &["accountId", "account_id", "id"]) {
                ids.push(id);
            }
        }
    }
    if ids.is_empty() {
        return None;
    }
    accounts
        .iter()
        .filter(|item| normalize_platform_id(&item.platform_id) == normalize_platform_id(platform_id))
        .find(|item| {
            ids.iter().any(|id| {
                item.id == *id || item.relay_account_ref.as_deref() == Some(id.as_str())
            })
        })
        .cloned()
}

fn should_open_relay_auth_in_app(platform_id: &str, auth_type: &str, url: &str) -> bool {
    normalize_platform_id(platform_id) == "kuaishou"
        && auth_type == "oauth"
        && !url.trim().is_empty()
        && !url.starts_with("data:image")
}

fn relay_auth_window_label(platform_id: &str, task_id: &str) -> String {
    format!(
        "relay-auth-{}-{}",
        normalize_platform_id(platform_id).replace('-', "_"),
        task_suffix(task_id)
    )
}

fn close_relay_auth_windows_for_platform(app: &AppHandle, platform_id: &str, keep_label: &str) {
    let prefix = format!(
        "relay-auth-{}-",
        normalize_platform_id(platform_id).replace('-', "_")
    );
    for window in app.webview_windows().into_values() {
        let label = window.label();
        if label.starts_with(&prefix) && label != keep_label {
            let _ = window.close();
        }
    }
}

fn close_auth_window_by_label(app: &AppHandle, label: Option<&str>) {
    if let Some(label) = label {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.close();
        }
    }
}

fn kuaishou_qrcode_login_url(url: &Url) -> Option<Url> {
    let is_login_page = url
        .host_str()
        .map(|host| host.eq_ignore_ascii_case("open.kuaishou.com"))
        .unwrap_or(false)
        && url.path() == "/web/oauth/login";
    if !is_login_page {
        return None;
    }
    let has_qrcode_flag = url
        .query_pairs()
        .any(|(key, value)| key == "needQrcode" && value == "1");
    if has_qrcode_flag {
        return None;
    }

    let mut login_url = url.clone();
    let has_qr_type = login_url
        .query_pairs()
        .any(|(key, _)| key == "needQrType");
    {
        let mut pairs = login_url.query_pairs_mut();
        pairs.append_pair("needQrcode", "1");
        if !has_qr_type {
            pairs.append_pair("needQrType", "identity-verify-auto");
        }
    }
    Some(login_url)
}

fn kuaishou_pc_authorize_url(url: &Url) -> Option<Url> {
    let is_authorize_page = url
        .host_str()
        .map(|host| host.eq_ignore_ascii_case("open.kuaishou.com"))
        .unwrap_or(false)
        && url.path() == "/oauth2/authorize";
    if !is_authorize_page {
        return None;
    }
    if url
        .query_pairs()
        .any(|(key, value)| key == "ua" && value.eq_ignore_ascii_case("pc"))
    {
        return None;
    }
    let mut next = url.clone();
    next.query_pairs_mut().append_pair("ua", "pc");
    Some(next)
}

fn open_relay_auth_window(
    app: &AppHandle,
    platform_id: &str,
    task_id: &str,
    auth_url: &str,
) -> Result<String, String> {
    let normalized = normalize_platform_id(platform_id);
    let mut url = Url::parse(auth_url).map_err(|error| format!("快手授权地址无效: {error}"))?;
    if normalized == "kuaishou" {
        if let Some(pc_url) = kuaishou_pc_authorize_url(&url) {
            url = pc_url;
        }
    }
    let label = relay_auth_window_label(platform_id, task_id);
    close_relay_auth_windows_for_platform(app, platform_id, &label);
    let title = format!("授权{} - 营销大师", platform_name(platform_id));

    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_title(&title);
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(label);
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建快手授权窗口数据目录: {error}"))?
        .join("relay-auth")
        .join(&normalized)
        .join(task_id);
    let window = WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1120.0, 780.0)
        .min_inner_size(960.0, 640.0)
        .data_directory(data_dir)
        .data_store_identifier(task_data_store_identifier(task_id))
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                if let Some(next) = kuaishou_qrcode_login_url(payload.url()) {
                    let _ = window.navigate(next);
                }
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开快手授权窗口失败: {error}"))?;
    let _ = window.clear_all_browsing_data();
    let _ = window.navigate(url);
    Ok(label)
}

fn capture_auth_window_cookies_any(
    app: &AppHandle,
    label: Option<&str>,
    urls: &[&str],
) -> Option<String> {
    let window = label.and_then(|label| app.get_webview_window(label))?;
    collect_webview_cookies(&window, urls)
        .ok()
        .map(|(_, login_cookie)| login_cookie)
        .filter(|value| !value.trim().is_empty())
}

fn relay_error_message(value: &Value) -> String {
    first_string(value, &["message", "msg", "error", "reason"])
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| "AiToEarn relay 暂时不可用，请稍后再试".to_string())
}

fn aitoearn_platform_id(platform_id: &str) -> Option<&'static str> {
    match normalize_platform_id(platform_id).as_str() {
        "kuaishou" => Some("KWAI"),
        _ => None,
    }
}

fn encode_path_segment(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

async fn check_creator_session(
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
        let _ = window.close();
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

    std::thread::sleep(std::time::Duration::from_millis(450));
    let cookies = collect_webview_cookies(
        &window,
        &[
            "https://www.xiaohongshu.com/",
            "https://creator.xiaohongshu.com/",
            "https://edith.xiaohongshu.com/",
        ],
    );
    let _ = window.close();

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
        let _ = window.close();
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

    std::thread::sleep(std::time::Duration::from_millis(450));
    let profile = match collect_wx_sph_plugin_account(&window).await {
        Ok(profile) => Some(profile),
        Err(error) => {
            eprintln!("[refresh:wx-sph] profile scan failed for {task_id}: {}", plugin_error_message(&error));
            None
        }
    };
    let _ = window.close();

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

fn existing_plugin_account_for_profile(
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

fn plugin_error_message(error: &PluginAuthError) -> String {
    match error {
        PluginAuthError::NotLoggedIn(message) | PluginAuthError::Failed(message) => message.clone(),
    }
}

fn is_plugin_auth_platform(platform_id: &str) -> bool {
    channels::is_plugin_auth_platform(platform_id)
}

async fn collect_plugin_account_info(
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

fn plugin_info_to_channel_account(
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

fn first_profile_image(value: &Value, keys: &[&str]) -> Option<String> {
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

fn normalize_platform_image_url(platform_id: &str, value: String) -> String {
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

fn create_direct_oauth_url(
    platform_settings: &PlatformAuthSettings,
    callback_url: &str,
    task_id: &str,
) -> Result<String, String> {
    if platform_settings.auth_url.trim().is_empty() || platform_settings.client_id.trim().is_empty()
    {
        return Err("请先填写该平台的 OAuth 授权地址和 Client ID".to_string());
    }
    let mut url = Url::parse(platform_settings.auth_url.trim())
        .map_err(|error| format!("OAuth 授权地址无效: {error}"))?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", platform_settings.client_id.trim())
        .append_pair("redirect_uri", callback_url)
        .append_pair("state", task_id);
    if !platform_settings.scopes.is_empty() {
        url.query_pairs_mut()
            .append_pair("scope", &platform_settings.scopes.join(" "));
    }
    Ok(url.to_string())
}

async fn ensure_callback_server(
    app: AppHandle,
    state: &RuntimeState,
) -> Result<String, String> {
    if let Some(port) = *state.callback_port.lock().map_err(lock_error)? {
        return Ok(format!("http://127.0.0.1:{port}"));
    }
    if state.callback_starting.swap(true, Ordering::SeqCst) {
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            if let Some(port) = *state.callback_port.lock().map_err(lock_error)? {
                return Ok(format!("http://127.0.0.1:{port}"));
            }
        }
        return Err("本地回调服务正在启动，请稍后重试".to_string());
    }

    let mut last_error: Option<io::Error> = None;
    for port in CALLBACK_PORT_START..=CALLBACK_PORT_END {
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(listener) => {
                *state.callback_port.lock().map_err(lock_error)? = Some(port);
                state.callback_starting.store(false, Ordering::SeqCst);
                tauri::async_runtime::spawn(callback_loop(app, listener));
                return Ok(format!("http://127.0.0.1:{port}"));
            }
            Err(error) => last_error = Some(error),
        }
    }

    state.callback_starting.store(false, Ordering::SeqCst);
    Err(format!(
        "无法启动本地回调服务: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "端口不可用".to_string())
    ))
}

async fn callback_loop(app: AppHandle, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = handle_callback_connection(app, stream).await {
                        eprintln!("callback connection failed: {error}");
                    }
                });
            }
            Err(error) => {
                eprintln!("callback listener failed: {error}");
                break;
            }
        }
    }
}

async fn handle_callback_connection(app: AppHandle, mut stream: TcpStream) -> Result<(), String> {
    let mut buffer = vec![0_u8; 64 * 1024];
    let size = stream
        .read(&mut buffer)
        .await
        .map_err(|error| error.to_string())?;
    let raw = String::from_utf8_lossy(&buffer[..size]).to_string();
    let request = parse_http_request(&raw)?;
    let result = match request.path.as_str() {
        path if path.starts_with("/oauth-callback") => finish_oauth_callback(&app, &request).await,
        _ => Err("未知回调路径".to_string()),
    };

    let html = match result {
        Ok(account) => success_page(&account.nickname),
        Err(error) => error_page(&error),
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.as_bytes().len(),
        html
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn finish_oauth_callback(
    app: &AppHandle,
    request: &HttpRequest,
) -> Result<ChannelAccount, String> {
    let state = request
        .query
        .get("state")
        .cloned()
        .or_else(|| request.query.get("taskId").cloned())
        .ok_or_else(|| "OAuth 回调缺少 state".to_string())?;
    let code = request
        .query
        .get("code")
        .cloned()
        .ok_or_else(|| "OAuth 回调缺少 code".to_string())?;

    let runtime = app.state::<RuntimeState>();
    let pending = runtime
        .pending_auth
        .lock()
        .map_err(lock_error)?
        .get(&state)
        .cloned()
        .ok_or_else(|| "授权任务不存在或已过期".to_string())?;
    let settings = runtime.store.lock().map_err(lock_error)?.settings.clone();
    let platform_settings = settings
        .platforms
        .iter()
        .find(|item| item.platform_id == pending.platform_id)
        .cloned()
        .ok_or_else(|| "平台授权参数不存在".to_string())?;

    let token = exchange_oauth_token(&platform_settings, &pending.callback_url, &code).await?;
    let profile = fetch_oauth_profile(&platform_settings, &token.access_token)
        .await
        .unwrap_or_else(|_| json!({}));
    let uid = first_string(&profile, &["id", "uid", "open_id", "unionid", "sub", "user_id"])
        .unwrap_or_else(|| state.clone());
    let nickname = first_string(&profile, &["nickname", "name", "username", "screen_name"])
        .unwrap_or_else(|| platform_name(&platform_settings.platform_id).to_string());
    let avatar = first_string(
        &profile,
        &[
            "avatar",
            "avatar_url",
            "picture",
            "profile_image_url",
            "profile_picture_url",
        ],
    )
    .unwrap_or_default();
    let followers = first_count(&profile, FOLLOWER_COUNT_KEYS);
    let likes = first_count(&profile, LIKE_COUNT_KEYS);

    upsert_account_for_user(
        app,
        &pending.user_id,
        ChannelAccount {
            id: Uuid::new_v4().to_string(),
            user_id: None,
            platform_id: platform_settings.platform_id,
            uid,
            nickname,
            avatar,
            followers,
            likes,
            status: AccountStatus::Active,
            relay_account_ref: None,
            token: Some(token),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_sync_at: Some(Utc::now()),
        },
    )
}

async fn exchange_oauth_token(
    settings: &PlatformAuthSettings,
    redirect_uri: &str,
    code: &str,
) -> Result<OAuthToken, String> {
    if settings.token_url.trim().is_empty() {
        return Err("OAuth 回调成功，但未配置 Token URL，无法换取 access_token".to_string());
    }
    let client = Client::new();
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("client_id", settings.client_id.clone()),
        ("code", code.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
    ];
    if !settings.client_secret.trim().is_empty() {
        form.push(("client_secret", settings.client_secret.clone()));
    }
    let value: Value = client
        .post(settings.token_url.trim())
        .form(&form)
        .send()
        .await
        .map_err(|error| format!("Token 请求失败: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Token 响应不是 JSON: {error}"))?;
    let access_token = first_string(&value, &["access_token", "accessToken"])
        .ok_or_else(|| format!("Token 响应缺少 access_token: {value}"))?;
    let refresh_token = first_string(&value, &["refresh_token", "refreshToken"]);
    let expires_at = first_i64(&value, &["expires_in", "expiresIn"])
        .map(|seconds| Utc::now() + Duration::seconds(seconds));

    Ok(OAuthToken {
        access_token,
        refresh_token,
        expires_at,
        raw: value,
    })
}

async fn fetch_oauth_profile(
    settings: &PlatformAuthSettings,
    access_token: &str,
) -> Result<Value, String> {
    if settings.profile_url.trim().is_empty() {
        return Ok(json!({}));
    }
    Client::new()
        .get(settings.profile_url.trim())
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| format!("用户资料请求失败: {error}"))?
        .json()
        .await
        .map_err(|error| format!("用户资料响应不是 JSON: {error}"))
}
