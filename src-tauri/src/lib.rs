use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    process::Command,
    sync::{
        atomic::AtomicBool,
        Mutex,
    },
};
use tauri::{webview::Cookie, AppHandle, Emitter, Manager, State};
use tauri::{WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::{form_urlencoded, Url};
use uuid::Uuid;

mod channels;
mod common;
mod http_callback;
mod json_ext;
mod local_store;
mod oauth;
mod plugin_accounts;
mod relay;
mod settings;
mod webview_windows;

use common::*;
use http_callback::*;
use json_ext::*;
use local_store::*;
use oauth::*;
use plugin_accounts::*;
use relay::*;
use settings::*;
use webview_windows::*;

const DESKTOP_CHROME_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const CHANNEL_ACCOUNT_UPDATED_EVENT: &str = "channel-account-updated";
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
                            if let Some(window) = app.get_webview_window(&window_label) {
                                destroy_webview_window(&window);
                            }
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
                    if let Some(window) = app.get_webview_window(&window_label) {
                        destroy_webview_window(&window);
                    }
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
                    if let Some(window) = app.get_webview_window(&window_label) {
                        destroy_webview_window(&window);
                    }
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
    let (account, saved_login_cookie, saved_webview_session_id) = {
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

    match normalize_platform_id(&account.platform_id).as_str() {
        "douyin" => {
            open_douyin_creator_webview(
                &app,
                &account,
                saved_login_cookie.as_deref(),
                saved_webview_session_id.as_deref(),
            )?;
            Ok(account)
        }
        "xiaohongshu" => {
            open_xhs_creator_webview(
                &app,
                &account,
                saved_login_cookie.as_deref(),
                saved_webview_session_id.as_deref(),
            )?;
            Ok(account)
        }
        "wechat-channels" => {
            open_wx_channels_webview(
                &app,
                &account,
                saved_login_cookie.as_deref(),
                saved_webview_session_id.as_deref(),
            )?;
            Ok(account)
        }
        "bilibili" => {
            open_bilibili_creator_webview(
                &app,
                &account,
                saved_login_cookie.as_deref(),
                saved_webview_session_id.as_deref(),
            )?;
            Ok(account)
        }
        "kuaishou" => {
            open_kuaishou_creator_webview(
                &app,
                &account,
                saved_login_cookie.as_deref(),
                saved_webview_session_id.as_deref(),
            )?;
            Ok(account)
        }
        _ => {
            let url = account_homepage_url(&account)?;
            open_external_url(&url)?;
            Ok(account)
        }
    }
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
