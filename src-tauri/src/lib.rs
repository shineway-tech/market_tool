use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    io,
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::{webview::Cookie, AppHandle, Manager, State};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tauri::{WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::{form_urlencoded, Url};
use uuid::Uuid;

const CALLBACK_PORT_START: u16 = 17654;
const CALLBACK_PORT_END: u16 = 17674;
const RELAY_SERVER_URL: &str = "https://aitoearn.cn/api";
const RELAY_API_KEY: &str = match option_env!("MARKETING_MASTER_RELAY_API_KEY") {
    Some(value) => value,
    None => "",
};
const DESKTOP_CHROME_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const DOUYIN_CREATOR_HOME_URL: &str =
    "https://creator.douyin.com/creator-micro/home?enter_from=dou_web";
const DOUYIN_COOKIE_URLS: &[&str] = &[
    "https://www.douyin.com/",
    "https://douyin.com/",
    "https://creator.douyin.com/",
    "https://passport.douyin.com/",
    "https://sso.douyin.com/",
];
const XHS_CREATOR_HOME_URL: &str = "https://creator.xiaohongshu.com/";
const XHS_COOKIE_URLS: &[&str] = &[
    "https://www.xiaohongshu.com/",
    "https://creator.xiaohongshu.com/",
    "https://edith.xiaohongshu.com/",
];
const WECHAT_CHANNELS_HOME_URL: &str = "https://channels.weixin.qq.com/platform";
const WECHAT_CHANNELS_COOKIE_URLS: &[&str] = &[
    "https://channels.weixin.qq.com/",
    "https://channels.weixin.qq.com/platform",
];
const BILIBILI_CREATOR_HOME_URL: &str = "https://member.bilibili.com/platform/home";
const BILIBILI_COOKIE_URLS: &[&str] = &[
    "https://www.bilibili.com/",
    "https://bilibili.com/",
    "https://passport.bilibili.com/",
    "https://member.bilibili.com/",
    "https://space.bilibili.com/",
];
const KUAISHOU_CREATOR_HOME_URL: &str = "https://cp.kuaishou.com/";
const KUAISHOU_COOKIE_URLS: &[&str] = &[
    "https://www.kuaishou.com/",
    "https://kuaishou.com/",
    "https://cp.kuaishou.com/",
    "https://id.kuaishou.com/",
    "https://passport.kuaishou.com/",
];
const DOUYIN_LOGIN_COOKIE_NAMES: &[&str] = &[
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
    relay_path: String,
    relay_method: HttpMethod,
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
    relay: RelaySettings,
    platforms: Vec<PlatformAuthSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum AuthMode {
    Relay,
    OAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum HttpMethod {
    GET,
    POST,
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
    status: AccountStatus,
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
    relay_platform_id: Option<String>,
    relay_session_id: Option<String>,
    relay_window_label: Option<String>,
    douyin_cookie_account: Option<ChannelAccount>,
    douyin_cookie_window_label: Option<String>,
    douyin_cookie_window_opened_at: Option<DateTime<Utc>>,
    plugin_window_label: Option<String>,
    plugin_login_target: Option<String>,
    created_at: DateTime<Utc>,
}

struct RuntimeState {
    store: Mutex<StoreFile>,
    pending_auth: Mutex<HashMap<String, PendingAuth>>,
    pending_auth_cookies: Mutex<HashMap<String, String>>,
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
struct AitoearnAuthSession {
    url: String,
    session_id: String,
    expires_at: Option<String>,
    instructions: Option<String>,
    auth_type: String,
}

struct AuthWindowProfile {
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
    user_agent: &'static str,
}

#[derive(Debug, Clone)]
struct PluginAccountInfo {
    relay_platform_id: String,
    uid: String,
    account: String,
    nickname: String,
    avatar: String,
    fans_count: Option<u64>,
    login_cookie: String,
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
                pending_auth_cookies: Mutex::new(HashMap::new()),
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
    let mut settings = store.settings.clone();
    settings.relay.api_key.clear();
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
    let settings = {
        let store = state.store.lock().map_err(lock_error)?;
        store.settings.clone()
    };
    let platform_settings = settings
        .platforms
        .iter()
        .find(|item| item.platform_id == request.platform_id)
        .cloned()
        .ok_or_else(|| "未找到平台授权参数".to_string())?;

    let mode = platform_settings.mode.clone();
    let callback_base = match mode {
        AuthMode::OAuth => ensure_callback_server(app.clone(), &state).await?,
        AuthMode::Relay => "aitoearn://channel-auth".to_string(),
    };
    let callback_path = match mode {
        AuthMode::Relay => "relay-callback",
        AuthMode::OAuth => "oauth-callback",
    };
    let callback_url = format!(
        "{callback_base}/{callback_path}?platform={}&taskId={}",
        encode_query(&request.platform_id),
        encode_query(&task_id)
    );

    let mut relay_platform_id = None;
    let mut relay_session_id = None;
    let mut relay_window_label = None;
    let mut plugin_window_label = None;
    let mut plugin_login_target = None;
    let mut expires_at = None;
    let mut instructions = None;
    let mut auth_type = "oauth".to_string();
    let mut opened_in_app = false;

    let auth_url = match mode {
        AuthMode::Relay => {
            relay_platform_id = aitoearn_platform_id(&platform_settings.platform_id)
                .map(ToString::to_string);
            if is_plugin_auth_platform(&platform_settings.platform_id) {
                let target = normalize_plugin_login_target(
                    &platform_settings.platform_id,
                    request.login_target.as_deref(),
                );
                let session =
                    open_plugin_login_window(&app, &platform_settings.platform_id, &task_id, target)?;
                plugin_login_target = target.map(ToString::to_string);
                plugin_window_label = Some(session.session_id.clone());
                relay_session_id = None;
                expires_at = session.expires_at.clone();
                instructions = session.instructions.clone();
                auth_type = session.auth_type.clone();
                session.url
            } else {
                let session = create_aitoearn_auth_session(&settings.relay, &platform_settings).await?;
                if should_open_relay_auth_in_app(
                    &platform_settings.platform_id,
                    &session.auth_type,
                    &session.url,
                ) {
                    relay_window_label = Some(open_relay_auth_window(
                        &app,
                        &platform_settings.platform_id,
                        &task_id,
                        &session.url,
                    )?);
                    opened_in_app = true;
                }
                relay_session_id = Some(session.session_id.clone());
                expires_at = session.expires_at.clone();
                instructions = session.instructions.clone();
                auth_type = session.auth_type.clone();
                session.url
            }
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
                relay_platform_id: relay_platform_id.clone(),
                relay_session_id: relay_session_id.clone(),
                relay_window_label: relay_window_label.clone(),
                douyin_cookie_account: None,
                douyin_cookie_window_label: None,
                douyin_cookie_window_opened_at: None,
                plugin_window_label: plugin_window_label.clone(),
                plugin_login_target: plugin_login_target.clone(),
                created_at: Utc::now(),
            },
        );
    }

    if auth_type == "oauth" && !auth_url.starts_with("data:image") && !opened_in_app {
        open_external_url(&auth_url)?;
    }

    Ok(StartLoginResponse {
        task_id,
        url: auth_url,
        callback_url,
        mode,
        auth_type,
        session_id: relay_session_id,
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
        if normalize_platform_id(&task.platform_id) == "douyin" {
            if let Some(account) = task.douyin_cookie_account.clone() {
                let configured_window_label = task.douyin_cookie_window_label.clone();
                let existing_window_label = configured_window_label
                    .clone()
                    .filter(|label| app.get_webview_window(label).is_some());

                if configured_window_label.is_some() && existing_window_label.is_none() {
                    state
                        .pending_auth
                        .lock()
                        .map_err(lock_error)?
                        .remove(&task_id);
                    let _ = take_pending_auth_cookie(&app, &task_id);
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "failed".to_string(),
                        account: None,
                        message: Some("抖音网页登录窗口已关闭，授权流程已中断。".to_string()),
                    });
                }

                let cookie_window_ready = task
                    .douyin_cookie_window_opened_at
                    .map(|opened_at| Utc::now().signed_duration_since(opened_at) >= Duration::seconds(8))
                    .unwrap_or(true);
                let login_cookie = if cookie_window_ready {
                    existing_window_label
                        .as_deref()
                        .and_then(|label| {
                            capture_auth_window_login_cookie(&app, Some(label), DOUYIN_COOKIE_URLS)
                        })
                        .or_else(|| take_pending_auth_cookie(&app, &task_id).ok().flatten())
                } else {
                    None
                };

                if let Some(login_cookie) = login_cookie.as_deref() {
                    let account = upsert_account_for_user(&app, &task.user_id, account)?;
                    upsert_account_secret(&app, &account.id, login_cookie)?;
                    upsert_account_webview_session(&app, &account.id, &task_id)?;
                    state
                        .pending_auth
                        .lock()
                        .map_err(lock_error)?
                        .remove(&task_id);
                    close_auth_window_by_label(&app, existing_window_label.as_deref());
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "success".to_string(),
                        account: Some(account),
                        message: None,
                    });
                }

                let _window_label = match existing_window_label {
                    Some(label) => label,
                    None => {
                        eprintln!(
                            "[douyin-auth] reopening cookie binding window task={} account={}",
                            task_id, account.id
                        );
                        let label = open_douyin_cookie_binding_window(&app, &task_id, &account, None)?;
                        let mut pending = state.pending_auth.lock().map_err(lock_error)?;
                        if let Some(existing) = pending.get_mut(&task_id) {
                            existing.douyin_cookie_window_label = Some(label.clone());
                            existing.douyin_cookie_window_opened_at = Some(Utc::now());
                        }
                        label
                    }
                };

                return Ok(AuthTaskStatus {
                    task_id,
                    status: "pending".to_string(),
                    account: None,
                    message: Some("OAuth 授权已完成，请在打开的抖音创作者中心窗口中完成网页登录，用于保存免登录状态。".to_string()),
                });
            }
        }

        if let Some(window_label) = task.plugin_window_label.clone() {
            let settings = state.store.lock().map_err(lock_error)?.settings.clone();
            let relay_platform_id = task
                .relay_platform_id
                .clone()
                .or_else(|| aitoearn_platform_id(&task.platform_id).map(ToString::to_string))
                .ok_or_else(|| "当前平台不在 AiToEarn 支持列表中".to_string())?;
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

                    let account = match sync_plugin_account_to_aitoearn(
                        &app,
                        &task.user_id,
                        &settings.relay,
                        &relay_platform_id,
                        profile,
                    )
                    .await
                    {
                        Ok(account) => account,
                        Err(error) => {
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
                                message: Some(plugin_error_message(&error)),
                            });
                        }
                    };
                    if matches!(
                        normalize_platform_id(&task.platform_id).as_str(),
                        "xiaohongshu" | "wechat-channels"
                    ) {
                        let _ = upsert_account_webview_session(&app, &account.id, &task_id);
                    }
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

        if let (Some(relay_platform_id), Some(session_id)) =
            (task.relay_platform_id.clone(), task.relay_session_id.clone())
        {
            let settings = state.store.lock().map_err(lock_error)?.settings.clone();
            let status = fetch_aitoearn_auth_status(&settings.relay, &relay_platform_id, &session_id)
                .await?;
            let remote_status = first_string(&status, &["status"]).unwrap_or_default();
            if remote_status == "completed" || remote_status == "success" {
                let settings = state.store.lock().map_err(lock_error)?.settings.clone();
                let synced = fetch_aitoearn_accounts(&settings.relay).await?;
                let account = find_synced_auth_account(&synced, &task.platform_id, &status)
                    .or_else(|| {
                        synced
                            .iter()
                            .find(|item| {
                                item.platform_id == task.platform_id && item.created_at >= task.created_at
                            })
                            .cloned()
                    })
                    .or_else(|| {
                        synced
                            .iter()
                            .filter(|item| item.platform_id == task.platform_id)
                            .max_by_key(|item| item.created_at)
                            .cloned()
                    });
                if normalize_platform_id(&task.platform_id) == "douyin" {
                    let account = account
                        .clone()
                        .ok_or_else(|| "抖音授权已完成，但没有同步到账号信息，请重新授权。".to_string())?;
                    let _ = take_pending_auth_cookie(&app, &task_id);
                    close_auth_window_by_label(&app, task.relay_window_label.as_deref());
                    eprintln!(
                        "[douyin-auth] oauth completed, opening cookie binding window task={} account={}",
                        task_id, account.id
                    );
                    let cookie_window_label =
                        open_douyin_cookie_binding_window(&app, &task_id, &account, None)?;
                    {
                        let mut pending = state.pending_auth.lock().map_err(lock_error)?;
                        if let Some(existing) = pending.get_mut(&task_id) {
                            existing.relay_session_id = None;
                            existing.relay_window_label = None;
                            existing.douyin_cookie_account = Some(account);
                            existing.douyin_cookie_window_label = Some(cookie_window_label);
                            existing.douyin_cookie_window_opened_at = Some(Utc::now());
                        }
                    }
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "pending".to_string(),
                        account: None,
                        message: Some("抖音账号授权已完成，请在打开的抖音创作者中心窗口中完成网页登录，用于保存免登录状态。".to_string()),
                    });
                }
                if normalize_platform_id(&task.platform_id) == "bilibili" {
                    let account = account
                        .clone()
                        .ok_or_else(|| "B 站授权已完成，但没有同步到账号信息，请重新授权。".to_string())?;
                    let account = upsert_account_for_user(&app, &task.user_id, account)?;
                    let login_cookie = capture_auth_window_cookies_any(
                        &app,
                        task.relay_window_label.as_deref(),
                        BILIBILI_COOKIE_URLS,
                    );
                    if let Some(login_cookie) = login_cookie.as_deref() {
                        upsert_account_secret(&app, &account.id, login_cookie)?;
                    }
                    upsert_account_webview_session(&app, &account.id, &task_id)?;
                    if account.created_at < task.created_at {
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
                            message: Some(format!(
                                "当前窗口登录的是已添加的 B 站账号「{}」。请先退出该账号，再从 B 站列表加号重新添加新账号。",
                                account.nickname
                            )),
                        });
                    }
                }
                if normalize_platform_id(&task.platform_id) == "kuaishou" {
                    let account = account
                        .clone()
                        .ok_or_else(|| "快手授权已完成，但没有同步到账号信息，请重新授权。".to_string())?;
                    let account = upsert_account_for_user(&app, &task.user_id, account)?;
                    let login_cookie = capture_auth_window_cookies_any(
                        &app,
                        task.relay_window_label.as_deref(),
                        KUAISHOU_COOKIE_URLS,
                    );
                    if let Some(login_cookie) = login_cookie.as_deref() {
                        upsert_account_secret(&app, &account.id, login_cookie)?;
                    }
                    upsert_account_webview_session(&app, &account.id, &task_id)?;
                    if account.created_at < task.created_at {
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
                            message: Some(format!(
                                "当前窗口登录的是已添加的快手账号「{}」。请先退出该账号，再从快手列表加号重新添加新账号。",
                                account.nickname
                            )),
                        });
                    }
                }
                state
                    .pending_auth
                    .lock()
                    .map_err(lock_error)?
                    .remove(&task_id);
                close_auth_window_by_label(&app, task.relay_window_label.as_deref());
                return Ok(AuthTaskStatus {
                    task_id,
                    status: "success".to_string(),
                    account: account
                        .map(|item| upsert_account_for_user(&app, &task.user_id, item))
                        .transpose()?,
                    message: None,
                });
            }

            if remote_status == "failed" || remote_status == "expired" || remote_status == "timeout" {
                let message = first_string(&status, &["message", "reason"])
                    .unwrap_or_else(|| "平台授权没有完成，请重新尝试。".to_string());
                state
                    .pending_auth
                    .lock()
                    .map_err(lock_error)?
                    .remove(&task_id);
                let _ = take_pending_auth_cookie(&app, &task_id);
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
                message: None,
            });
        }
    }

    let store = state.store.lock().map_err(lock_error)?;
    let accounts = user_accounts(&store, &user_id);
    let account = accounts
        .iter()
        .find(|item| {
            item.relay_account_ref.as_deref() == Some(task_id.as_str())
                || item.id == task_id
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
    let (settings, secret_cookie) = {
        let mut store = state.store.lock().map_err(lock_error)?;
        let migrated = migrate_account_secret_for_account(&mut store, account);
        let value = (
            store.settings.clone(),
            account_secret_for_account(&store, account).and_then(|secret| secret.login_cookie),
        );
        if migrated {
            persist_store(&app, &store)?;
        }
        value
    };
    let xhs_result = if account.platform_id == "xiaohongshu" {
        Some(refresh_xhs_account_profile(&app, account, secret_cookie.as_deref()).await)
    } else {
        None
    };
    let analytics_result = if xhs_result.is_none() {
        refresh_aitoearn_account_analytics(&settings.relay, account).await
    } else {
        Ok(None)
    };
    if let Some(Err(error)) = xhs_result.as_ref() {
        return Err(error.clone());
    }
    if analytics_result.is_err() {
        return Err(format!(
            "刷新失败：{}",
            analytics_result
                .as_ref()
                .err()
                .cloned()
                .unwrap_or_else(|| "平台账号数据暂时不可用".to_string())
        ));
    }

    let analytics_followers = analytics_result.ok().flatten();
    let xhs_profile = xhs_result.and_then(Result::ok).flatten();
    if let Some(profile) = xhs_profile.as_ref() {
        let _ = sync_plugin_account_to_aitoearn(
            &app,
            &user_id,
            &settings.relay,
            &profile.relay_platform_id,
            profile.clone(),
        )
        .await;
    }

    let mut store = state.store.lock().map_err(lock_error)?;
    if let Some(profile) = xhs_profile.as_ref() {
        if !profile.login_cookie.trim().is_empty() {
            let secret = store.account_secrets.entry(account_id.clone()).or_default();
            secret.login_cookie = Some(profile.login_cookie.clone());
        }
    }
    let account = store
        .accounts
        .iter_mut()
        .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
        .ok_or_else(|| "账号不存在".to_string())?;
    if let Some(profile) = xhs_profile.as_ref() {
        if !profile.nickname.trim().is_empty() {
            account.nickname = profile.nickname.clone();
        }
        if !profile.avatar.trim().is_empty() {
            account.avatar = profile.avatar.clone();
        }
        if let Some(fans_count) = profile.fans_count {
            account.followers = Some(fans_count);
        }
        if account.uid.trim().is_empty() {
            account.uid = profile.uid.clone();
        }
    }
    if let Some(followers) = analytics_followers {
        account.followers = Some(followers);
    }
    account.status = match account.token.as_ref().and_then(|token| token.expires_at) {
        Some(expires_at) if expires_at <= Utc::now() => AccountStatus::Expired,
        _ => AccountStatus::Active,
    };
    account.last_sync_at = Some(Utc::now());
    account.updated_at = Utc::now();
    let cloned = account.clone();
    persist_store(&app, &store)?;
    Ok(cloned)
}

#[tauri::command]
async fn open_account_homepage(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<(), String> {
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

    if normalize_platform_id(&account.platform_id) == "douyin" {
        return open_douyin_creator_webview(
            &app,
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        );
    }
    if normalize_platform_id(&account.platform_id) == "xiaohongshu" {
        let mut login_cookie = saved_login_cookie;
        let mut webview_session_id = saved_webview_session_id;
        if webview_session_id.is_none() {
            if let Some((session_id, profile)) = find_xhs_session_for_account(&app, &account).await? {
                upsert_account_secret(&app, &account.id, &profile.login_cookie)?;
                upsert_account_webview_session(&app, &account.id, &session_id)?;
                login_cookie = Some(profile.login_cookie);
                webview_session_id = Some(session_id);
            }
        }
        return open_xhs_creator_webview(
            &app,
            &account,
            login_cookie.as_deref(),
            webview_session_id.as_deref(),
        );
    }
    if normalize_platform_id(&account.platform_id) == "wechat-channels" {
        let mut login_cookie = saved_login_cookie;
        let mut webview_session_id = saved_webview_session_id;
        if webview_session_id.is_none() {
            if let Some((session_id, profile)) = find_wx_channels_session_for_account(&app, &account).await? {
                upsert_account_secret(&app, &account.id, &profile.login_cookie)?;
                upsert_account_webview_session(&app, &account.id, &session_id)?;
                login_cookie = Some(profile.login_cookie);
                webview_session_id = Some(session_id);
            }
        }
        return open_wx_channels_webview(
            &app,
            &account,
            login_cookie.as_deref(),
            webview_session_id.as_deref(),
        );
    }
    if normalize_platform_id(&account.platform_id) == "bilibili" {
        return open_bilibili_creator_webview(
            &app,
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        );
    }
    if normalize_platform_id(&account.platform_id) == "kuaishou" {
        return open_kuaishou_creator_webview(
            &app,
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        );
    }

    let url = account_homepage_url(&account)?;
    open_external_url(&url)
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
        let settings = state.store.lock().map_err(lock_error)?.settings.clone();
        let _ = delete_aitoearn_account(&settings.relay, account).await;
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

async fn create_aitoearn_auth_session(
    relay: &RelaySettings,
    platform_settings: &PlatformAuthSettings,
) -> Result<AitoearnAuthSession, String> {
    if !relay.enabled || relay.server_url.trim().is_empty() || relay.api_key.trim().is_empty() {
        return Err("授权服务参数不可用，请检查内置配置".to_string());
    }

    let relay_platform_id = aitoearn_platform_id(&platform_settings.platform_id)
        .ok_or_else(|| "当前平台不在 AiToEarn 支持列表中".to_string())?;
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
        .ok_or_else(|| format!("授权响应缺少授权 URL: {value}"))?;
    let session_id = first_string(data, &["sessionId", "session_id"])
        .ok_or_else(|| format!("授权响应缺少 sessionId: {value}"))?;
    let auth_type = if url.starts_with("data:image") {
        "qrcode"
    } else {
        "oauth"
    }
    .to_string();
    let instructions = data
        .get("authInstructions")
        .and_then(|value| first_string(value, &["zh-CN", "zh", "en-US", "en"]));

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
        .ok_or_else(|| "账号列表响应格式无效".to_string())?;

    let mut accounts = Vec::new();
    for item in list {
        if let Some(account) = aitoearn_account_from_value(item) {
            accounts.push(account);
        }
    }
    Ok(accounts)
}

async fn refresh_aitoearn_account_analytics(
    relay: &RelaySettings,
    account: &ChannelAccount,
) -> Result<Option<u64>, String> {
    let Some(relay_account_id) = account
        .relay_account_ref
        .as_deref()
        .or_else(|| account.id.strip_prefix(""))
    else {
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
    let url = Url::parse(WECHAT_CHANNELS_HOME_URL)
        .map_err(|error| format!("视频号后台地址无效: {error}"))?;
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
        account.relay_account_ref.clone().unwrap_or_default(),
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
        account.relay_account_ref.as_deref().unwrap_or_default(),
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
        account.relay_account_ref.as_deref().unwrap_or_default(),
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

fn account_homepage_url(account: &ChannelAccount) -> Result<String, String> {
    let platform_id = normalize_platform_id(&account.platform_id);
    let uid = account.uid.trim();
    let nickname = account.nickname.trim();
    match platform_id.as_str() {
        "douyin" => {
            Ok(DOUYIN_CREATOR_HOME_URL.to_string())
        }
        "xiaohongshu" => {
            Ok(XHS_CREATOR_HOME_URL.to_string())
        }
        "wechat-channels" => {
            Ok(WECHAT_CHANNELS_HOME_URL.to_string())
        }
        "bilibili" => {
            if !uid.is_empty() && uid.chars().all(|ch| ch.is_ascii_digit()) {
                Ok(format!("https://space.bilibili.com/{}", encode_path_segment(uid)))
            } else {
                account_search_url("https://search.bilibili.com/upuser?keyword=", nickname)
            }
        }
        "kuaishou" => {
            if !uid.is_empty() {
                Ok(format!("https://www.kuaishou.com/profile/{}", encode_path_segment(uid)))
            } else {
                account_search_url("https://www.kuaishou.com/search/author?searchKey=", nickname)
            }
        }
        _ => Err("当前平台暂不支持打开主页".to_string()),
    }
}

fn open_douyin_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = Url::parse(DOUYIN_CREATOR_HOME_URL).map_err(|error| format!("抖音创作者中心地址无效: {error}"))?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-douyin-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "抖音账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 抖音创作者中心");

    if let Some(window) = app.get_webview_window(&label) {
        let should_delay_navigation = saved_login_cookie.is_some();
        if let Some(login_cookie) = saved_login_cookie {
            let _ = inject_douyin_login_cookie(&window, login_cookie);
        }
        let _ = window.set_title(&title);
        if should_delay_navigation {
            navigate_webview_after_delay(window.clone(), url.clone());
        } else {
            let _ = window.navigate(url);
        }
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let (data_dir, data_store_identifier) = if let Some(session_id) = saved_webview_session_id {
        (
            app.path()
                .app_data_dir()
                .map_err(|error| format!("无法创建抖音创作者中心数据目录: {error}"))?
                .join("auth-sessions")
                .join("douyin")
                .join(stable_label_fragment(session_id)),
            task_data_store_identifier(session_id),
        )
    } else {
        (
            app.path()
                .app_data_dir()
                .map_err(|error| format!("无法创建抖音创作者中心数据目录: {error}"))?
                .join("creator-home")
                .join("douyin")
                .join(&window_key),
            stable_data_store_identifier(&format!("creator-home:douyin:{}", account.id)),
        )
    };
    let account_id = account.id.clone();
    let app_for_load = app.clone();

    let initial_url = if saved_login_cookie.is_some() {
        Url::parse("about:blank").map_err(|error| format!("空白页地址无效: {error}"))?
    } else {
        url.clone()
    };

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(initial_url))
        .title(&title)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                && is_douyin_web_url(payload.url())
            {
                let _ = persist_webview_account_cookies(
                    &app_for_load,
                    &window,
                    &account_id,
                    DOUYIN_COOKIE_URLS,
                );
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开抖音创作者中心失败: {error}"))?;

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_douyin_login_cookie(&window, login_cookie);
        navigate_webview_after_delay(window.clone(), url);
    }

    Ok(())
}

fn navigate_webview_after_delay(window: WebviewWindow<tauri::Wry>, url: Url) {
    tauri::async_runtime::spawn(async move {
        std::thread::sleep(std::time::Duration::from_millis(600));
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
    });
}

fn open_xhs_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = Url::parse(XHS_CREATOR_HOME_URL).map_err(|error| format!("小红书创作中心地址无效: {error}"))?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-xhs-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "小红书账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 小红书创作中心");

    if let Some(window) = app.get_webview_window(&label) {
        if let Some(login_cookie) = saved_login_cookie {
            let _ = inject_xhs_login_cookie(&window, login_cookie);
        }
        let _ = window.set_title(&title);
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建小红书创作中心数据目录: {error}"))?;
    let (data_dir, data_store_identifier) = if let Some(session_id) = saved_webview_session_id {
        (
            app_data_dir
                .join("plugin-auth")
                .join("xiaohongshu")
                .join(session_id),
            task_data_store_identifier(session_id),
        )
    } else {
        (
            app_data_dir
                .join("creator-home")
                .join("xiaohongshu")
                .join(&window_key),
            stable_data_store_identifier(&format!("creator-home:xhs:{}", account.id)),
        )
    };
    let account_id = account.id.clone();
    let app_for_load = app.clone();

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                && is_xhs_web_url(payload.url())
            {
                let _ = persist_webview_account_cookies_any(
                    &app_for_load,
                    &window,
                    &account_id,
                    XHS_COOKIE_URLS,
                );
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开小红书创作中心失败: {error}"))?;
    let _ = window.show();
    let _ = window.set_focus();

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_xhs_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(())
}

fn open_wx_channels_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = Url::parse(WECHAT_CHANNELS_HOME_URL).map_err(|error| format!("视频号后台地址无效: {error}"))?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-wx-sph-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "视频号账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 视频号后台");

    if let Some(window) = app.get_webview_window(&label) {
        if let Some(login_cookie) = saved_login_cookie {
            let _ = inject_wx_channels_login_cookie(&window, login_cookie);
        }
        let _ = window.set_title(&title);
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建视频号后台数据目录: {error}"))?;
    let (data_dir, data_store_identifier) = if let Some(session_id) = saved_webview_session_id {
        (
            app_data_dir
                .join("plugin-auth")
                .join("wechat-channels")
                .join(session_id),
            task_data_store_identifier(session_id),
        )
    } else {
        (
            app_data_dir
                .join("creator-home")
                .join("wechat-channels")
                .join(&window_key),
            stable_data_store_identifier(&format!("creator-home:wx-sph:{}", account.id)),
        )
    };
    let account_id = account.id.clone();
    let app_for_load = app.clone();

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                && is_wx_channels_web_url(payload.url())
            {
                let _ = persist_webview_account_cookies_any(
                    &app_for_load,
                    &window,
                    &account_id,
                    WECHAT_CHANNELS_COOKIE_URLS,
                );
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开视频号后台失败: {error}"))?;
    let _ = window.show();
    let _ = window.set_focus();

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_wx_channels_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(())
}

fn open_bilibili_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = Url::parse(BILIBILI_CREATOR_HOME_URL)
        .map_err(|error| format!("B 站创作中心地址无效: {error}"))?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-bilibili-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "B 站账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - B 站创作中心");

    if let Some(window) = app.get_webview_window(&label) {
        if let Some(login_cookie) = saved_login_cookie {
            let _ = inject_bilibili_login_cookie(&window, login_cookie);
        }
        let _ = window.set_title(&title);
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建 B 站创作中心数据目录: {error}"))?;
    let (data_dir, data_store_identifier) = if let Some(session_id) = saved_webview_session_id {
        (
            app_data_dir
                .join("relay-auth")
                .join("bilibili")
                .join(session_id),
            task_data_store_identifier(session_id),
        )
    } else {
        (
            app_data_dir
                .join("creator-home")
                .join("bilibili")
                .join(&window_key),
            stable_data_store_identifier(&format!("creator-home:bilibili:{}", account.id)),
        )
    };
    let account_id = account.id.clone();
    let app_for_load = app.clone();

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                && is_bilibili_web_url(payload.url())
            {
                let _ = persist_webview_account_cookies_any(
                    &app_for_load,
                    &window,
                    &account_id,
                    BILIBILI_COOKIE_URLS,
                );
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开 B 站创作中心失败: {error}"))?;
    let _ = window.show();
    let _ = window.set_focus();

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_bilibili_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(())
}

fn open_kuaishou_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = Url::parse(KUAISHOU_CREATOR_HOME_URL)
        .map_err(|error| format!("快手创作者中心地址无效: {error}"))?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-kuaishou-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "快手账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 快手创作者中心");

    if let Some(window) = app.get_webview_window(&label) {
        if let Some(login_cookie) = saved_login_cookie {
            let _ = inject_kuaishou_login_cookie(&window, login_cookie);
        }
        let _ = window.set_title(&title);
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建快手创作者中心数据目录: {error}"))?;
    let (data_dir, data_store_identifier) = if let Some(session_id) = saved_webview_session_id {
        (
            app_data_dir
                .join("relay-auth")
                .join("kuaishou")
                .join(session_id),
            task_data_store_identifier(session_id),
        )
    } else {
        (
            app_data_dir
                .join("creator-home")
                .join("kuaishou")
                .join(&window_key),
            stable_data_store_identifier(&format!("creator-home:kuaishou:{}", account.id)),
        )
    };
    let account_id = account.id.clone();
    let app_for_load = app.clone();

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                && is_kuaishou_web_url(payload.url())
            {
                let _ = persist_webview_account_cookies_any(
                    &app_for_load,
                    &window,
                    &account_id,
                    KUAISHOU_COOKIE_URLS,
                );
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开快手创作者中心失败: {error}"))?;
    let _ = window.show();
    let _ = window.set_focus();

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_kuaishou_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(())
}

fn open_douyin_cookie_binding_window(
    app: &AppHandle,
    task_id: &str,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
) -> Result<String, String> {
    let url = Url::parse(DOUYIN_CREATOR_HOME_URL).map_err(|error| format!("抖音创作者中心地址无效: {error}"))?;
    let label = format!("douyin-cookie-bind-{}", task_suffix(task_id));
    let title_name = if account.nickname.trim().is_empty() {
        "抖音账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 绑定抖音网页登录态");
    eprintln!(
        "[douyin-auth] building cookie binding window label={} account={}",
        label, account.id
    );

    if let Some(window) = app.get_webview_window(&label) {
        if let Some(login_cookie) = saved_login_cookie {
            let _ = inject_douyin_login_cookie(&window, login_cookie);
        }
        let _ = window.set_title(&title);
        let _ = window.navigate(url);
        let _ = window.unminimize();
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let _ = window.set_focus();
        eprintln!("[douyin-auth] focused existing cookie binding window label={label}");
        return Ok(label);
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建抖音登录态数据目录: {error}"))?
        .join("auth-sessions")
        .join("douyin")
        .join(stable_label_fragment(task_id));

    let window = WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .always_on_top(true)
        .data_directory(data_dir)
        .data_store_identifier(task_data_store_identifier(task_id))
        .user_agent(DESKTOP_CHROME_UA)
        .center()
        .build()
        .map_err(|error| format!("打开抖音登录态绑定窗口失败: {error}"))?;
    let _ = window.unminimize();
    let _ = window.set_always_on_top(true);
    let _ = window.show();
    let _ = window.set_focus();
    eprintln!("[douyin-auth] opened cookie binding window label={label}");

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_douyin_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(label)
}

fn inject_douyin_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    if trimmed.starts_with('[') {
        let Value::Array(cookies) =
            serde_json::from_str::<Value>(trimmed).map_err(|error| format!("抖音登录态格式无效: {error}"))?
        else {
            return Ok(());
        };

        for cookie in cookies {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                continue;
            };
            let raw_domain = cookie.get("domain").and_then(Value::as_str).unwrap_or("");
            if !raw_domain.trim().is_empty() && !should_inject_douyin_cookie_domain(raw_domain) {
                continue;
            }
            let domain = if raw_domain.trim().is_empty() {
                ".douyin.com"
            } else {
                raw_domain
            };
            let path = cookie.get("path").and_then(Value::as_str).unwrap_or("/");
            let secure = cookie.get("secure").and_then(Value::as_bool).unwrap_or(true);
            let http_only = cookie
                .get("httpOnly")
                .or_else(|| cookie.get("http_only"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            set_webview_cookie(window, name, value, domain, path, secure, http_only)?;
        }
        return Ok(());
    }

    for pair in trimmed.split(';') {
        let pair = pair.trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        set_webview_cookie(window, name, value, ".douyin.com", "/", true, false)?;
    }
    Ok(())
}

fn inject_xhs_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    if trimmed.starts_with('[') {
        let Value::Array(cookies) =
            serde_json::from_str::<Value>(trimmed).map_err(|error| format!("小红书登录态格式无效: {error}"))?
        else {
            return Ok(());
        };

        for cookie in cookies {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                continue;
            };
            let raw_domain = cookie.get("domain").and_then(Value::as_str).unwrap_or("");
            if !raw_domain.trim().is_empty() && !should_inject_xhs_cookie_domain(raw_domain) {
                continue;
            }
            let domain = if raw_domain.trim().is_empty() {
                ".xiaohongshu.com"
            } else {
                raw_domain
            };
            let path = cookie.get("path").and_then(Value::as_str).unwrap_or("/");
            let secure = cookie.get("secure").and_then(Value::as_bool).unwrap_or(true);
            let http_only = cookie
                .get("httpOnly")
                .or_else(|| cookie.get("http_only"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            set_webview_cookie(window, name, value, domain, path, secure, http_only)?;
        }
        return Ok(());
    }

    for pair in trimmed.split(';') {
        let pair = pair.trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        set_webview_cookie(window, name, value, ".xiaohongshu.com", "/", true, false)?;
    }
    Ok(())
}

fn inject_wx_channels_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    if trimmed.starts_with('[') {
        let Value::Array(cookies) =
            serde_json::from_str::<Value>(trimmed).map_err(|error| format!("视频号登录态格式无效: {error}"))?
        else {
            return Ok(());
        };

        for cookie in cookies {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                continue;
            };
            let raw_domain = cookie.get("domain").and_then(Value::as_str).unwrap_or("");
            if !raw_domain.trim().is_empty() && !should_inject_wx_channels_cookie_domain(raw_domain) {
                continue;
            }
            let domain = if raw_domain.trim().is_empty() {
                "channels.weixin.qq.com"
            } else {
                raw_domain
            };
            let path = cookie.get("path").and_then(Value::as_str).unwrap_or("/");
            let secure = cookie.get("secure").and_then(Value::as_bool).unwrap_or(true);
            let http_only = cookie
                .get("httpOnly")
                .or_else(|| cookie.get("http_only"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            set_webview_cookie(window, name, value, domain, path, secure, http_only)?;
        }
        return Ok(());
    }

    for pair in trimmed.split(';') {
        let pair = pair.trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        set_webview_cookie(
            window,
            name,
            value,
            "channels.weixin.qq.com",
            "/",
            true,
            false,
        )?;
    }
    Ok(())
}

fn inject_bilibili_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    if trimmed.starts_with('[') {
        let Value::Array(cookies) =
            serde_json::from_str::<Value>(trimmed).map_err(|error| format!("B 站登录态格式无效: {error}"))?
        else {
            return Ok(());
        };

        for cookie in cookies {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                continue;
            };
            let raw_domain = cookie.get("domain").and_then(Value::as_str).unwrap_or("");
            if !raw_domain.trim().is_empty() && !should_inject_bilibili_cookie_domain(raw_domain) {
                continue;
            }
            let domain = if raw_domain.trim().is_empty() {
                ".bilibili.com"
            } else {
                raw_domain
            };
            let path = cookie.get("path").and_then(Value::as_str).unwrap_or("/");
            let secure = cookie.get("secure").and_then(Value::as_bool).unwrap_or(true);
            let http_only = cookie
                .get("httpOnly")
                .or_else(|| cookie.get("http_only"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            set_webview_cookie(window, name, value, domain, path, secure, http_only)?;
        }
        return Ok(());
    }

    for pair in trimmed.split(';') {
        let pair = pair.trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        set_webview_cookie(window, name, value, ".bilibili.com", "/", true, false)?;
    }
    Ok(())
}

fn inject_kuaishou_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    if trimmed.starts_with('[') {
        let Value::Array(cookies) =
            serde_json::from_str::<Value>(trimmed).map_err(|error| format!("快手登录态格式无效: {error}"))?
        else {
            return Ok(());
        };

        for cookie in cookies {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                continue;
            };
            let raw_domain = cookie.get("domain").and_then(Value::as_str).unwrap_or("");
            if !raw_domain.trim().is_empty() && !should_inject_kuaishou_cookie_domain(raw_domain) {
                continue;
            }
            let domain = if raw_domain.trim().is_empty() {
                ".kuaishou.com"
            } else {
                raw_domain
            };
            let path = cookie.get("path").and_then(Value::as_str).unwrap_or("/");
            let secure = cookie.get("secure").and_then(Value::as_bool).unwrap_or(true);
            let http_only = cookie
                .get("httpOnly")
                .or_else(|| cookie.get("http_only"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            set_webview_cookie(window, name, value, domain, path, secure, http_only)?;
        }
        return Ok(());
    }

    for pair in trimmed.split(';') {
        let pair = pair.trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        set_webview_cookie(window, name, value, ".kuaishou.com", "/", true, false)?;
    }
    Ok(())
}

fn set_webview_cookie(
    window: &WebviewWindow<tauri::Wry>,
    name: &str,
    value: &str,
    domain: &str,
    path: &str,
    secure: bool,
    http_only: bool,
) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(());
    }
    let domain = domain.trim();
    let path = path.trim();
    let mut cookie = Cookie::build((name.to_string(), value.to_string()))
        .path(if path.is_empty() { "/" } else { path })
        .secure(secure);
    if !domain.is_empty() {
        cookie = cookie.domain(domain.to_string());
    }
    if http_only {
        cookie = cookie.http_only(true);
    }
    window
        .set_cookie(cookie.build())
        .map_err(|error| format!("注入抖音登录态失败: {error}"))
}

fn persist_webview_account_cookies(
    app: &AppHandle,
    window: &WebviewWindow<tauri::Wry>,
    account_id: &str,
    urls: &[&str],
) -> Result<(), String> {
    let Some(login_cookie) = collect_webview_login_cookie(window, urls)? else {
        return Ok(());
    };
    upsert_account_secret(app, account_id, &login_cookie)
}

fn persist_webview_account_cookies_any(
    app: &AppHandle,
    window: &WebviewWindow<tauri::Wry>,
    account_id: &str,
    urls: &[&str],
) -> Result<(), String> {
    let (cookie_header, login_cookie) = collect_webview_cookies(window, urls)?;
    if cookie_header.trim().is_empty() {
        return Ok(());
    }
    upsert_account_secret(app, account_id, &login_cookie)
}

fn take_pending_auth_cookie(app: &AppHandle, task_id: &str) -> Result<Option<String>, String> {
    Ok(app
        .state::<RuntimeState>()
        .pending_auth_cookies
        .lock()
        .map_err(lock_error)?
        .remove(task_id))
}

fn capture_auth_window_login_cookie(
    app: &AppHandle,
    window_label: Option<&str>,
    urls: &[&str],
) -> Option<String> {
    let Some(window_label) = window_label else {
        return None;
    };
    let Some(window) = app.get_webview_window(window_label) else {
        return None;
    };
    collect_webview_login_cookie(&window, urls).ok().flatten()
}

fn capture_auth_window_cookies_any(
    app: &AppHandle,
    window_label: Option<&str>,
    urls: &[&str],
) -> Option<String> {
    let Some(window_label) = window_label else {
        return None;
    };
    let Some(window) = app.get_webview_window(window_label) else {
        return None;
    };
    let (cookie_header, login_cookie) = collect_webview_cookies(&window, urls).ok()?;
    if cookie_header.trim().is_empty() {
        return None;
    }
    Some(login_cookie)
}

fn collect_webview_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    urls: &[&str],
) -> Result<Option<String>, String> {
    let (cookie_header, login_cookie) = collect_webview_cookies(window, urls)?;
    if cookie_header.trim().is_empty() {
        return Ok(None);
    }
    if !has_douyin_login_cookie(&login_cookie) {
        return Ok(None);
    }
    Ok(Some(login_cookie))
}

fn has_douyin_login_cookie(login_cookie: &str) -> bool {
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.starts_with('[') {
        let Ok(Value::Array(cookies)) = serde_json::from_str::<Value>(trimmed) else {
            return false;
        };
        return cookies.iter().any(|cookie| {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                return false;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                return false;
            };
            if value.trim().is_empty() || !is_douyin_login_cookie_name(name) {
                return false;
            }
            cookie
                .get("domain")
                .and_then(Value::as_str)
                .map(should_inject_douyin_cookie_domain)
                .unwrap_or(true)
        });
    }

    trimmed.split(';').any(|pair| {
        let Some((name, value)) = pair.trim().split_once('=') else {
            return false;
        };
        is_douyin_login_cookie_name(name) && !value.trim().is_empty()
    })
}

fn is_douyin_login_cookie_name(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    DOUYIN_LOGIN_COOKIE_NAMES.iter().any(|item| item == &name)
}

fn should_inject_douyin_cookie_domain(domain: &str) -> bool {
    let domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    domain.is_empty() || domain == "douyin.com" || domain.ends_with(".douyin.com")
}

fn should_inject_xhs_cookie_domain(domain: &str) -> bool {
    let domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    domain.is_empty() || domain == "xiaohongshu.com" || domain.ends_with(".xiaohongshu.com")
}

fn should_inject_wx_channels_cookie_domain(domain: &str) -> bool {
    let domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    domain.is_empty() || domain == "channels.weixin.qq.com"
}

fn should_inject_bilibili_cookie_domain(domain: &str) -> bool {
    let domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    domain.is_empty() || domain == "bilibili.com" || domain.ends_with(".bilibili.com")
}

fn should_inject_kuaishou_cookie_domain(domain: &str) -> bool {
    let domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    domain.is_empty()
        || domain == "kuaishou.com"
        || domain.ends_with(".kuaishou.com")
        || domain == "kwai.com"
        || domain.ends_with(".kwai.com")
}

fn is_douyin_web_url(url: &Url) -> bool {
    url.host_str()
        .map(|host| {
            let host = host.to_ascii_lowercase();
            host == "douyin.com" || host.ends_with(".douyin.com")
        })
        .unwrap_or(false)
}

fn is_xhs_web_url(url: &Url) -> bool {
    url.host_str()
        .map(|host| {
            let host = host.to_ascii_lowercase();
            host == "xiaohongshu.com" || host.ends_with(".xiaohongshu.com")
        })
        .unwrap_or(false)
}

fn is_wx_channels_web_url(url: &Url) -> bool {
    url.host_str()
        .map(|host| host.eq_ignore_ascii_case("channels.weixin.qq.com"))
        .unwrap_or(false)
}

fn is_bilibili_web_url(url: &Url) -> bool {
    url.host_str()
        .map(|host| {
            let host = host.to_ascii_lowercase();
            host == "bilibili.com" || host.ends_with(".bilibili.com")
        })
        .unwrap_or(false)
}

fn is_kuaishou_web_url(url: &Url) -> bool {
    url.host_str()
        .map(|host| {
            let host = host.to_ascii_lowercase();
            host == "kuaishou.com"
                || host.ends_with(".kuaishou.com")
                || host == "kwai.com"
                || host.ends_with(".kwai.com")
        })
        .unwrap_or(false)
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

fn account_search_url(prefix: &str, keyword: &str) -> Result<String, String> {
    if keyword.trim().is_empty() {
        return Err("账号缺少主页标识，无法打开主页".to_string());
    }
    Ok(format!("{prefix}{}", encode_query(keyword.trim())))
}

fn is_plugin_auth_platform(platform_id: &str) -> bool {
    matches!(platform_id, "xiaohongshu" | "xhs" | "wechat-channels" | "wxSph" | "wxsph")
}

fn should_open_relay_auth_in_app(platform_id: &str, auth_type: &str, url: &str) -> bool {
    is_relay_auth_window_platform(platform_id)
        && auth_type == "oauth"
        && !url.trim().is_empty()
        && !url.starts_with("data:image")
}

fn is_relay_auth_window_platform(platform_id: &str) -> bool {
    matches!(
        normalize_platform_id(platform_id).as_str(),
        "bilibili" | "kuaishou" | "douyin"
    )
}

fn task_suffix(task_id: &str) -> String {
    task_id.chars().take(8).collect()
}

fn task_data_store_identifier(task_id: &str) -> [u8; 16] {
    Uuid::parse_str(task_id)
        .map(|uuid| *uuid.as_bytes())
        .unwrap_or_else(|_| *Uuid::new_v4().as_bytes())
}

fn stable_label_fragment(value: &str) -> String {
    format!("{:016x}", stable_hash(value, 0xcbf29ce484222325))
}

fn stable_data_store_identifier(value: &str) -> [u8; 16] {
    let first = stable_hash(value, 0xcbf29ce484222325);
    let second = stable_hash(value, 0x84222325cbf29ce4);
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&first.to_le_bytes());
    bytes[8..].copy_from_slice(&second.to_le_bytes());
    bytes
}

fn stable_hash(value: &str, seed: u64) -> u64 {
    value.as_bytes().iter().fold(seed, |hash, byte| {
        (hash ^ (*byte as u64)).wrapping_mul(0x100000001b3)
    })
}

fn relay_auth_window_label(platform_id: &str, task_id: &str) -> String {
    format!(
        "relay-auth-{}-{}",
        normalize_platform_id(platform_id).replace('-', "_"),
        task_suffix(task_id)
    )
}

fn relay_auth_window_profile(_platform_id: &str) -> AuthWindowProfile {
    AuthWindowProfile {
        width: 1120.0,
        height: 780.0,
        min_width: 960.0,
        min_height: 640.0,
        user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    }
}

fn close_auth_window_by_label(app: &AppHandle, label: Option<&str>) {
    if let Some(label) = label {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.close();
        }
    }
}

fn plugin_auth_window_label(platform_id: &str, task_id: &str) -> String {
    format!(
        "plugin-auth-{}-{}",
        normalize_platform_id(platform_id).replace('-', "_"),
        task_suffix(task_id)
    )
}

fn close_plugin_auth_windows_for_platform(app: &AppHandle, platform_id: &str, keep_label: &str) {
    let legacy_label = format!(
        "plugin-auth-{}",
        normalize_platform_id(platform_id).replace('-', "_")
    );
    if legacy_label != keep_label {
        if let Some(window) = app.get_webview_window(&legacy_label) {
            let _ = window.close();
        }
    }
    let prefix = format!(
        "plugin-auth-{}-",
        normalize_platform_id(platform_id).replace('-', "_")
    );
    for window in app.webview_windows().into_values() {
        let label = window.label();
        if label.starts_with(&prefix) && label != keep_label {
            let _ = window.close();
        }
    }
}

fn close_relay_auth_windows_for_platform(app: &AppHandle, platform_id: &str, keep_label: &str) {
    if !is_relay_auth_window_platform(platform_id) {
        return;
    }
    let legacy_label = format!(
        "relay-auth-{}",
        normalize_platform_id(platform_id).replace('-', "_")
    );
    if legacy_label != keep_label {
        if let Some(window) = app.get_webview_window(&legacy_label) {
            let _ = window.close();
        }
    }
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

fn open_relay_auth_window(
    app: &AppHandle,
    platform_id: &str,
    task_id: &str,
    auth_url: &str,
) -> Result<String, String> {
    let normalized = normalize_platform_id(platform_id);
    let mut url = Url::parse(auth_url).map_err(|error| format!("平台授权地址无效: {error}"))?;
    if normalized == "kuaishou" {
        if let Some(pc_url) = kuaishou_pc_authorize_url(&url) {
            url = pc_url;
        }
    }
    let label = relay_auth_window_label(platform_id, task_id);
    let title = format!("授权{} - 营销大师", platform_name(platform_id));
    let profile = relay_auth_window_profile(platform_id);
    close_relay_auth_windows_for_platform(app, platform_id, &label);

    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_title(&title);
        let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
            profile.min_width,
            profile.min_height,
        ))));
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
            profile.width,
            profile.height,
        )));
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(label);
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建授权窗口数据目录: {error}"))?
        .join("relay-auth")
        .join(&normalized)
        .join(task_id);
    let normalized_for_load = normalized.clone();

    let window = WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(profile.width, profile.height)
        .min_inner_size(profile.min_width, profile.min_height)
        .data_directory(data_dir)
        .data_store_identifier(task_data_store_identifier(task_id))
        .user_agent(profile.user_agent)
        .on_page_load(move |window, payload| {
            if normalized_for_load == "kuaishou"
                && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
            {
                if let Some(next) = kuaishou_qrcode_login_url(payload.url()) {
                    let _ = window.navigate(next);
                }
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开平台授权窗口失败: {error}"))?;
    if matches!(normalized.as_str(), "bilibili" | "kuaishou") {
        let _ = window.clear_all_browsing_data();
        let _ = window.navigate(url);
    }

    Ok(label)
}

fn normalize_plugin_login_target(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    if matches!(platform_id, "xiaohongshu" | "xhs") {
        return match login_target {
            Some("home" | "homepage") => Some("home"),
            Some("creator" | "creator-center" | "creation") => Some("creator"),
            _ => Some("creator"),
        };
    }
    None
}

fn plugin_login_url(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    match platform_id {
        "xiaohongshu" | "xhs" => match login_target {
            Some("home") => Some(XHS_CREATOR_HOME_URL),
            _ => Some("https://creator.xiaohongshu.com/"),
        },
        "wechat-channels" | "wxSph" | "wxsph" => Some("https://channels.weixin.qq.com/platform"),
        _ => None,
    }
}

fn open_plugin_login_window(
    app: &AppHandle,
    platform_id: &str,
    task_id: &str,
    login_target: Option<&str>,
) -> Result<AitoearnAuthSession, String> {
    let login_url = plugin_login_url(platform_id, login_target)
        .ok_or_else(|| "当前平台不支持插件式授权".to_string())?;
    let url = Url::parse(login_url).map_err(|error| format!("平台登录地址无效: {error}"))?;
    let label = plugin_auth_window_label(platform_id, task_id);
    let title = match (normalize_platform_id(platform_id).as_str(), login_target) {
        ("xiaohongshu", Some("home")) => "登录小红书主页 - 营销大师".to_string(),
        ("xiaohongshu", Some("creator")) => "登录小红书创作中心 - 营销大师".to_string(),
        _ => format!("登录{} - 营销大师", platform_name(platform_id)),
    };
    close_plugin_auth_windows_for_platform(app, platform_id, &label);

    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_title(&title);
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(AitoearnAuthSession {
            url: login_url.to_string(),
            session_id: label,
            expires_at: None,
            instructions: Some(plugin_login_instructions(platform_id, login_target)),
            auth_type: "plugin".to_string(),
        });
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建授权窗口数据目录: {error}"))?
        .join("plugin-auth")
        .join(normalize_platform_id(platform_id))
        .join(task_id);

    let window = WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1120.0, 780.0)
        .min_inner_size(960.0, 640.0)
        .data_directory(data_dir)
        .data_store_identifier(task_data_store_identifier(task_id))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .center()
        .build()
        .map_err(|error| format!("打开平台登录窗口失败: {error}"))?;
    if matches!(
        normalize_platform_id(platform_id).as_str(),
        "xiaohongshu" | "wechat-channels"
    ) {
        let _ = window.clear_all_browsing_data();
        let _ = window.navigate(url);
    }

    Ok(AitoearnAuthSession {
        url: login_url.to_string(),
        session_id: label,
        expires_at: None,
        instructions: Some(plugin_login_instructions(platform_id, login_target)),
        auth_type: "plugin".to_string(),
    })
}

fn plugin_login_instructions(platform_id: &str, login_target: Option<&str>) -> String {
    match (normalize_platform_id(platform_id).as_str(), login_target) {
        ("xiaohongshu", Some("home")) => {
            "请在打开的小红书主页完成登录。当前客户端以创作中心登录作为账号授权成功标准。".to_string()
        }
        ("xiaohongshu", Some("creator")) => {
            "请在打开的小红书创作中心完成登录，登录成功后会自动同步账号资料。".to_string()
        }
        _ => format!(
            "请在打开的{}窗口完成登录，登录成功后点击检查状态同步账号。",
            platform_name(platform_id)
        ),
    }
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
        _ => {
            return Err(PluginAuthError::Failed(
                "当前平台不支持插件式授权".to_string(),
            ))
        }
    }
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
    let home_result = request_plugin_json(
        "GET",
        "https://edith.xiaohongshu.com/api/sns/web/v2/user/me",
        cookie_header,
        &[
            ("Origin", "https://www.xiaohongshu.com"),
            ("Referer", "https://www.xiaohongshu.com/"),
        ],
    )
    .await;
    if let Err(error) = &home_result {
        eprintln!("[plugin-auth:xhs] home profile request failed: {error}");
    }
    let creator_result = request_plugin_json(
        "GET",
        "https://creator.xiaohongshu.com/api/galaxy/creator/home/personal_info",
        cookie_header,
        &[
            ("Origin", "https://creator.xiaohongshu.com"),
            ("Referer", "https://creator.xiaohongshu.com/"),
        ],
    )
    .await;
    if let Err(error) = &creator_result {
        eprintln!("[plugin-auth:xhs] creator profile request failed: {error}");
    }
    let home = home_result.ok();
    let creator = creator_result.ok();
    let home_data = home.as_ref().and_then(|value| value.get("data"));
    let creator_data = creator.as_ref().and_then(|value| value.get("data"));
    let home_ok = home
        .as_ref()
        .map(|value| response_success(value) && home_data.and_then(|data| first_string(data, &["red_id"])).is_some())
        .unwrap_or(false);
    let creator_ok = creator
        .as_ref()
        .map(|value| response_success(value) && creator_data.is_some())
        .unwrap_or(false);
    eprintln!("[plugin-auth:xhs] home_ok={home_ok} creator_ok={creator_ok}");
    if !creator_ok {
        return Err(PluginAuthError::NotLoggedIn(match login_target {
            Some("home") => "请先在打开的小红书主页完成登录。".to_string(),
            _ => "请先在打开的小红书创作中心完成登录。".to_string(),
        }));
    }

    let uid = first_string_from_values(
        &[home_data, creator_data],
        &["user_id", "red_id", "userId", "id"],
    )
    .unwrap_or_default();
    let nickname = first_string_from_values(
        &[creator_data, home_data],
        &["name", "nickname", "nickName", "user_name", "userName", "red_id"],
    )
    .unwrap_or_else(|| platform_name("xiaohongshu").to_string());
    let avatar = first_profile_image_from_values(
        &[creator_data, home_data],
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
    .unwrap_or_default();
    let avatar = materialize_account_avatar("xiaohongshu", avatar).await;
    let account = if uid.trim().is_empty() {
        nickname.clone()
    } else {
        uid.clone()
    };
    if account.trim().is_empty() {
        return Err(PluginAuthError::NotLoggedIn(
            "小红书已登录，但没有读取到账号 ID，请进入创作者中心后再检查状态。".to_string(),
        ));
    }

    Ok(PluginAccountInfo {
        relay_platform_id: "xhs".to_string(),
        uid: account.clone(),
        account,
        nickname,
        avatar,
        fans_count: first_count_from_values(&[creator_data, home_data], FOLLOWER_COUNT_KEYS),
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
        relay_platform_id: "wxSph".to_string(),
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
        login_cookie,
    })
}

async fn sync_plugin_account_to_aitoearn(
    app: &AppHandle,
    user_id: &str,
    relay: &RelaySettings,
    relay_platform_id: &str,
    account: PluginAccountInfo,
) -> Result<ChannelAccount, PluginAuthError> {
    let group_id = default_aitoearn_group_id(relay)
        .await
        .map_err(PluginAuthError::Failed)?
        .unwrap_or_default();
    let mut body = json!({
        "type": account.relay_platform_id.clone(),
        "uid": account.uid.clone(),
        "account": account.account.clone(),
        "nickname": account.nickname.clone(),
        "loginCookie": account.login_cookie.clone(),
    });
    if !account.avatar.trim().is_empty() {
        body["avatar"] = json!(account.avatar);
    }
    if let Some(fans_count) = account.fans_count {
        body["fansCount"] = json!(fans_count);
    }
    if !group_id.trim().is_empty() {
        body["groupId"] = json!(group_id);
    }

    let value = aitoearn_post_json(relay, "v2/channels/accounts", &body)
        .await
        .map_err(PluginAuthError::Failed)?;
    ensure_aitoearn_success(&value).map_err(PluginAuthError::Failed)?;
    let data = relay_response_data(&value);
    let requested_uid = plugin_account_uid(&account);
    let remote_account_ref = first_string(data, &["id", "accountId"]).filter(|ref_id| {
        !relay_ref_belongs_to_different_uid(app, user_id, relay_platform_id, ref_id, &requested_uid)
            .unwrap_or(false)
    });
    let fallback_account =
        plugin_info_to_channel_account(relay_platform_id, &account, remote_account_ref.clone());
    let synced = fetch_aitoearn_accounts(relay)
        .await
        .map_err(PluginAuthError::Failed)?;
    let synced_account = synced
        .iter()
        .find(|item| {
            aitoearn_platform_id(&item.platform_id) == Some(relay_platform_id)
                && (account_uid_matches(&item.uid, &requested_uid)
                    || (item.relay_account_ref.as_deref() == remote_account_ref.as_deref()
                        && account_uid_matches(&item.uid, &requested_uid)))
        })
        .cloned()
        .or_else(|| {
            aitoearn_account_from_value(data)
                .filter(|item| account_uid_matches(&item.uid, &requested_uid))
        })
        .unwrap_or(fallback_account);
    let enriched = enrich_plugin_synced_account(synced_account, &account);
    let enriched =
        upsert_local_account(app, user_id, enriched.clone()).map_err(PluginAuthError::Failed)?;
    upsert_account_secret(app, &enriched.id, &account.login_cookie).map_err(PluginAuthError::Failed)?;
    Ok(enriched)
}

fn plugin_account_uid(account: &PluginAccountInfo) -> String {
    if account.uid.trim().is_empty() {
        account.account.clone()
    } else {
        account.uid.clone()
    }
}

fn account_uid_matches(left: &str, right: &str) -> bool {
    let left = normalize_match_key(left);
    let right = normalize_match_key(right);
    !left.is_empty() && left == right
}

fn relay_ref_belongs_to_different_uid(
    app: &AppHandle,
    user_id: &str,
    relay_platform_id: &str,
    relay_account_ref: &str,
    requested_uid: &str,
) -> Result<bool, String> {
    let runtime = app.state::<RuntimeState>();
    let store = runtime.store.lock().map_err(lock_error)?;
    let platform_id = normalize_platform_id(relay_platform_id);
    Ok(store.accounts.iter().any(|account| {
        account_belongs_to_user(account, user_id)
            && normalize_platform_id(&account.platform_id) == platform_id
            && (account.relay_account_ref.as_deref() == Some(relay_account_ref)
                || account.id == relay_account_ref)
            && !account_uid_matches(&account.uid, requested_uid)
    }))
}

fn plugin_info_to_channel_account(
    relay_platform_id: &str,
    account: &PluginAccountInfo,
    relay_account_ref: Option<String>,
) -> ChannelAccount {
    let platform_id = normalize_platform_id(relay_platform_id);
    let uid = plugin_account_uid(account);
    let id = relay_account_ref.clone().unwrap_or_else(|| {
        format!(
            "{}_{}",
            relay_platform_id,
            stable_label_fragment(&format!("{platform_id}:{uid}:{}", account.nickname))
        )
    });
    let now = Utc::now();
    ChannelAccount {
        id,
        user_id: None,
        platform_id,
        uid,
        nickname: account.nickname.clone(),
        avatar: account.avatar.clone(),
        followers: account.fans_count,
        status: AccountStatus::Active,
        relay_account_ref,
        token: None,
        created_at: now,
        updated_at: now,
        last_sync_at: Some(now),
    }
}

fn collect_webview_cookies(
    window: &WebviewWindow<tauri::Wry>,
    urls: &[&str],
) -> Result<(String, String), String> {
    let mut cookies = Vec::new();
    let mut seen = HashMap::new();
    let mut hosts = Vec::new();
    for url in urls {
        let parsed = Url::parse(url).map_err(|error| format!("Cookie 地址无效: {error}"))?;
        if let Some(host) = parsed.host_str() {
            hosts.push(host.to_ascii_lowercase());
        }
        for cookie in window
            .cookies_for_url(parsed.clone())
            .map_err(|error| format!("读取授权窗口 Cookie 失败: {error}"))?
        {
            let name = cookie.name().to_string();
            let value = cookie.value().to_string();
            let domain = cookie.domain().unwrap_or_default().to_string();
            let path = cookie.path().unwrap_or("/").to_string();
            let key = format!("{domain}|{path}|{name}");
            if seen.contains_key(&key) {
                continue;
            }
            seen.insert(key, true);
            cookies.push(json!({
                "name": name,
                "value": value,
                "domain": domain,
                "path": path,
                "secure": cookie.secure().unwrap_or(false),
                "httpOnly": cookie.http_only().unwrap_or(false),
            }));
        }
    }
    for cookie in window
        .cookies()
        .map_err(|error| format!("读取授权窗口全量 Cookie 失败: {error}"))?
    {
        let name = cookie.name().to_string();
        let value = cookie.value().to_string();
        let domain = cookie.domain().unwrap_or_default().to_string();
        if !cookie_domain_matches_hosts(&domain, &hosts) {
            continue;
        }
        let path = cookie.path().unwrap_or("/").to_string();
        let key = format!("{domain}|{path}|{name}");
        if seen.contains_key(&key) {
            continue;
        }
        seen.insert(key, true);
        cookies.push(json!({
            "name": name,
            "value": value,
            "domain": domain,
            "path": path,
            "secure": cookie.secure().unwrap_or(false),
            "httpOnly": cookie.http_only().unwrap_or(false),
        }));
    }
    let header = cookies
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
    let json = serde_json::to_string(&cookies).map_err(|error| error.to_string())?;
    Ok((header, json))
}

fn cookie_domain_matches_hosts(domain: &str, hosts: &[String]) -> bool {
    let domain = domain.trim_start_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        return false;
    }
    hosts.iter().any(|host| {
        host == &domain
            || host.ends_with(&format!(".{domain}"))
            || domain.ends_with(&format!(".{host}"))
    })
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

fn first_string_from_values(values: &[Option<&Value>], keys: &[&str]) -> Option<String> {
    values.iter().find_map(|value| value.and_then(|value| first_string(value, keys)))
}

fn first_count_from_values(values: &[Option<&Value>], keys: &[&str]) -> Option<u64> {
    values.iter().find_map(|value| value.and_then(|value| first_count(value, keys)))
}

fn first_profile_image_from_values(values: &[Option<&Value>], keys: &[&str]) -> Option<String> {
    values
        .iter()
        .find_map(|value| value.and_then(|value| first_profile_image(value, keys)))
        .map(normalize_image_url)
        .filter(|value| !value.trim().is_empty())
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
    matches!(
        normalize_platform_id(platform_id).as_str(),
        "xiaohongshu" | "bilibili" | "kuaishou"
    ) && !value.trim().is_empty()
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
    if normalize_platform_id(platform_id) == "xiaohongshu" {
        request = request
            .header("Referer", "https://creator.xiaohongshu.com/")
            .header("Origin", "https://creator.xiaohongshu.com");
    } else if normalize_platform_id(platform_id) == "bilibili" {
        request = request
            .header("Referer", "https://www.bilibili.com/")
            .header("Origin", "https://www.bilibili.com");
    } else if normalize_platform_id(platform_id) == "kuaishou" {
        request = request
            .header("Referer", "https://www.kuaishou.com/")
            .header("Origin", "https://www.kuaishou.com");
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

async fn aitoearn_post_json(
    relay: &RelaySettings,
    path: &str,
    body: &Value,
) -> Result<Value, String> {
    let url = relay_url(relay, path)?;
    aitoearn_request(Client::new().post(url).json(body), relay).await
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
        .map_err(|error| format!("请求 AiToEarn 授权服务失败: {error}"))?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|error| format!("AiToEarn 返回不是 JSON: {error}"))?;
    if !status.is_success() {
        return Err(format!("AiToEarn 返回 HTTP {status}: {}", relay_error_message(&value)));
    }
    Ok(value)
}

fn relay_url(relay: &RelaySettings, path: &str) -> Result<Url, String> {
    let base = relay.server_url.trim().trim_end_matches('/');
    let path = path.trim().trim_start_matches('/');
    Url::parse(&format!("{base}/{path}")).map_err(|error| format!("AiToEarn 地址无效: {error}"))
}

fn ensure_aitoearn_success(value: &Value) -> Result<(), String> {
    if let Some(code) = value.get("code").and_then(Value::as_i64) {
        if code != 0 {
            return Err(match code {
                401 | 403 => "AiToEarn API Key 认证失败".to_string(),
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
    let remote_platform = first_string(value, &["type", "platform"])?;
    let platform_id = normalize_platform_id(&remote_platform);
    let id = first_string(value, &["id", "accountId"]).unwrap_or_else(|| Uuid::new_v4().to_string());
    let uid = first_string(value, &["uid", "platformUid", "platform_uid"]).unwrap_or_else(|| id.clone());
    let nickname = first_string(value, &["nickname", "name", "displayName"])
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
    .map(normalize_image_url)
    .unwrap_or_default();
    let followers = first_count(value, FOLLOWER_COUNT_KEYS);
    let status = match value.get("status").and_then(Value::as_i64) {
        Some(1) | None => AccountStatus::Active,
        Some(_) => AccountStatus::Expired,
    };
    let created_at = first_string(value, &["createdAt", "created_at"])
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let updated_at = first_string(value, &["updatedAt", "updated_at"])
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let last_sync_at = first_string(value, &["lastStatsTime", "lastSyncAt", "updatedAt"])
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
        followers,
        status,
        relay_account_ref: Some(id),
        token: None,
        created_at,
        updated_at,
        last_sync_at,
    })
}

fn enrich_plugin_synced_account(mut synced: ChannelAccount, account: &PluginAccountInfo) -> ChannelAccount {
    if !account.nickname.trim().is_empty() {
        synced.nickname = account.nickname.clone();
    }
    if !account.avatar.trim().is_empty() {
        synced.avatar = account.avatar.clone();
    }
    if let Some(fans_count) = account.fans_count {
        synced.followers = Some(fans_count);
    }
    if synced.uid.trim().is_empty() {
        synced.uid = account.uid.clone();
    }
    synced.updated_at = Utc::now();
    synced.last_sync_at = Some(Utc::now());
    synced
}

fn upsert_local_account(
    app: &AppHandle,
    user_id: &str,
    account: ChannelAccount,
) -> Result<ChannelAccount, String> {
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let mut source_secret_keys = account_secret_candidates(&account);
    let mut account = scoped_account_for_user(user_id, account);
    for key in account_secret_candidates(&account) {
        push_unique(&mut source_secret_keys, key);
    }
    if let Some(existing) = store.accounts.iter_mut().find(|item| {
        account_belongs_to_user(item, user_id)
            && item.platform_id == account.platform_id
            && (item.uid == account.uid || item.relay_account_ref == account.relay_account_ref)
    }) {
        account.id = existing.id.clone();
        *existing = ChannelAccount {
            token: existing.token.clone(),
            created_at: existing.created_at.min(account.created_at),
            ..account.clone()
        };
    } else {
        store.accounts.push(account.clone());
    }
    migrate_account_secret_from_keys(&mut store, &account.id, &source_secret_keys);
    persist_store(app, &store)?;
    Ok(account)
}

fn upsert_account_secret(app: &AppHandle, account_id: &str, login_cookie: &str) -> Result<(), String> {
    if login_cookie.trim().is_empty() {
        return Ok(());
    }
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let secret = store.account_secrets.entry(account_id.to_string()).or_default();
    secret.login_cookie = Some(login_cookie.to_string());
    persist_store(app, &store)
}

fn upsert_account_webview_session(
    app: &AppHandle,
    account_id: &str,
    webview_session_id: &str,
) -> Result<(), String> {
    if webview_session_id.trim().is_empty() {
        return Ok(());
    }
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let secret = store.account_secrets.entry(account_id.to_string()).or_default();
    secret.webview_session_id = Some(webview_session_id.to_string());
    persist_store(app, &store)
}

fn find_synced_auth_account(
    accounts: &[ChannelAccount],
    _platform_id: &str,
    status: &Value,
) -> Option<ChannelAccount> {
    let mut ids = Vec::new();
    if let Some(id) = first_string(status, &["accountId", "id"]) {
        ids.push(id);
    }
    if let Some(values) = status.get("accountIds").and_then(Value::as_array) {
        ids.extend(values.iter().filter_map(Value::as_str).map(ToString::to_string));
    }
    if let Some(values) = status.get("accounts").and_then(Value::as_array) {
        for item in values {
            if let Some(id) = first_string(item, &["accountId", "id"]) {
                ids.push(id);
            }
        }
    }
    if !ids.is_empty() {
        return accounts
            .iter()
            .find(|item| {
                ids.iter().any(|id| {
                    item.id == *id || item.relay_account_ref.as_deref() == Some(id.as_str())
                })
            })
            .cloned();
    }
    None
}

fn relay_error_message(value: &Value) -> String {
    value
        .get("message")
        .and_then(Value::as_str)
        .filter(|message| !message.trim().is_empty())
        .unwrap_or("授权服务暂时不可用，请稍后再试")
        .to_string()
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
        path if path.starts_with("/relay-callback") => finish_relay_callback(&app, &request).await,
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

async fn finish_relay_callback(
    app: &AppHandle,
    request: &HttpRequest,
) -> Result<ChannelAccount, String> {
    let mut params = parse_body_params(request);
    params.extend(request.query.clone());
    let task_id = params
        .get("taskId")
        .cloned()
        .or_else(|| params.get("task_id").cloned())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let platform_id = params
        .get("platform")
        .cloned()
        .or_else(|| params.get("accountType").cloned())
        .or_else(|| params.get("account_type").cloned())
        .unwrap_or_else(|| "unknown".to_string());
    let platform_id = normalize_platform_id(&platform_id);
    let uid = params
        .get("platformUid")
        .cloned()
        .or_else(|| params.get("platform_uid").cloned())
        .or_else(|| params.get("uid").cloned())
        .unwrap_or_else(|| task_id.clone());
    let nickname = params
        .get("nickname")
        .cloned()
        .or_else(|| params.get("name").cloned())
        .unwrap_or_else(|| platform_name(&platform_id).to_string());
    let avatar = params.get("avatar").cloned().unwrap_or_default();
    let relay_account_ref = Some(task_id.clone());
    let user_id = app
        .state::<RuntimeState>()
        .pending_auth
        .lock()
        .map_err(lock_error)?
        .get(&task_id)
        .map(|task| task.user_id.clone())
        .ok_or_else(|| "授权任务不存在或已过期".to_string())?;

    if platform_id == "douyin" {
        let account = ChannelAccount {
            id: Uuid::new_v4().to_string(),
            user_id: None,
            platform_id,
            uid,
            nickname,
            avatar,
            followers: None,
            status: AccountStatus::Active,
            relay_account_ref,
            token: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_sync_at: Some(Utc::now()),
        };
        let task = app
            .state::<RuntimeState>()
            .pending_auth
            .lock()
            .map_err(lock_error)?
            .get(&task_id)
            .cloned();
        if let Some(task) = task {
            close_auth_window_by_label(app, task.relay_window_label.as_deref());
        }
        let cookie_window_label = open_douyin_cookie_binding_window(app, &task_id, &account, None)?;
        {
            let state = app.state::<RuntimeState>();
            let mut pending = state.pending_auth.lock().map_err(lock_error)?;
            if let Some(existing) = pending.get_mut(&task_id) {
                existing.relay_session_id = None;
                existing.relay_window_label = None;
                existing.douyin_cookie_account = Some(account.clone());
                existing.douyin_cookie_window_label = Some(cookie_window_label);
                existing.douyin_cookie_window_opened_at = Some(Utc::now());
            }
        }
        return Ok(account);
    }

    upsert_account_for_user(
        app,
        &user_id,
        ChannelAccount {
            id: Uuid::new_v4().to_string(),
            user_id: None,
            platform_id,
            uid,
            nickname,
            avatar,
            followers: None,
            status: AccountStatus::Active,
            relay_account_ref,
            token: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_sync_at: Some(Utc::now()),
        },
    )
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

fn upsert_account_for_user(
    app: &AppHandle,
    user_id: &str,
    account: ChannelAccount,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(user_id)?;
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let mut source_secret_keys = account_secret_candidates(&account);
    let mut account = scoped_account_for_user(&user_id, account);
    for key in account_secret_candidates(&account) {
        push_unique(&mut source_secret_keys, key);
    }
    if let Some(existing) = store
        .accounts
        .iter_mut()
        .find(|item| {
            account_belongs_to_user(item, &user_id)
                && item.platform_id == account.platform_id
                && (item.uid == account.uid || item.relay_account_ref == account.relay_account_ref)
        })
    {
        account.id = existing.id.clone();
        account.created_at = existing.created_at;
        *existing = account.clone();
    } else {
        store.accounts.push(account.clone());
    }
    migrate_account_secret_from_keys(&mut store, &account.id, &source_secret_keys);
    let completed_task_id = account.relay_account_ref.clone();
    runtime
        .pending_auth
        .lock()
        .map_err(lock_error)?
        .retain(|task_id, task| {
            task.user_id != user_id
                || (completed_task_id.as_deref() != Some(task_id.as_str())
                    && !(completed_task_id.is_none() && task.platform_id == account.platform_id))
        });
    persist_store(app, &store)?;
    Ok(account)
}

#[derive(Debug)]
struct HttpRequest {
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: String,
}

fn parse_http_request(raw: &str) -> Result<HttpRequest, String> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| "HTTP 请求格式无效".to_string())?;
    let mut lines = head.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP 请求缺少请求行".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let _method = request_parts.next().unwrap_or_default();
    let target = request_parts.next().unwrap_or("/");
    let mut target_parts = target.splitn(2, '?');
    let path = target_parts.next().unwrap_or("/").to_string();
    let query = target_parts
        .next()
        .map(parse_query)
        .unwrap_or_else(HashMap::new);
    let mut headers = HashMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok(HttpRequest {
        path,
        query,
        headers,
        body: body.to_string(),
    })
}

fn parse_body_params(request: &HttpRequest) -> HashMap<String, String> {
    let content_type = request
        .headers
        .get("content-type")
        .map(String::as_str)
        .unwrap_or_default();
    if content_type.contains("application/json") {
        if let Ok(value) = serde_json::from_str::<Value>(&request.body) {
            return flatten_json_object(&value);
        }
    }
    parse_query(&request.body)
}

fn parse_query(value: &str) -> HashMap<String, String> {
    form_urlencoded::parse(value.as_bytes())
        .into_owned()
        .collect::<HashMap<String, String>>()
}

fn flatten_json_object(value: &Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if let Some(text) = value.as_str() {
                map.insert(key.clone(), text.to_string());
            } else if value.is_number() || value.is_boolean() {
                map.insert(key.clone(), value.to_string());
            }
        }
    }
    map
}

fn load_store(app: &AppHandle) -> Result<StoreFile, Box<dyn std::error::Error>> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(StoreFile {
            accounts: Vec::new(),
            settings: default_auth_settings(),
            account_secrets: HashMap::new(),
        });
    }
    let text = fs::read_to_string(path)?;
    let mut store: StoreFile = serde_json::from_str(&text)?;
    store.settings = normalize_settings(store.settings);
    Ok(store)
}

fn persist_store(app: &AppHandle, store: &StoreFile) -> Result<(), String> {
    let path = store_path(app).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(store).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

fn store_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    Ok(dir.join("channel-auth-store.json"))
}

fn normalize_settings(_settings: AuthSettings) -> AuthSettings {
    default_auth_settings()
}

fn default_auth_settings() -> AuthSettings {
    AuthSettings {
        relay: RelaySettings {
            enabled: true,
            server_url: RELAY_SERVER_URL.to_string(),
            api_key: RELAY_API_KEY.to_string(),
        },
        platforms: vec![
            platform_auth("xiaohongshu", "plat/xhs/auth/url/pc", HttpMethod::GET),
            platform_auth("wechat-channels", "plat/wxSph/auth/url/pc", HttpMethod::GET),
            platform_auth("douyin", "plat/douyin/auth/url", HttpMethod::GET),
            platform_auth("bilibili", "plat/bilibili/auth/url/pc", HttpMethod::GET),
            platform_auth("kuaishou", "plat/kwai/auth/url/pc", HttpMethod::GET),
        ],
    }
}

fn platform_auth(
    platform_id: &str,
    relay_path: &str,
    relay_method: HttpMethod,
) -> PlatformAuthSettings {
    PlatformAuthSettings {
        platform_id: platform_id.to_string(),
        mode: AuthMode::Relay,
        relay_path: relay_path.to_string(),
        relay_method,
        auth_url: String::new(),
        token_url: String::new(),
        profile_url: String::new(),
        client_id: String::new(),
        client_secret: String::new(),
        scopes: Vec::new(),
    }
}

fn default_platforms() -> Vec<PlatformInfo> {
    vec![
        PlatformInfo {
            id: "xiaohongshu".to_string(),
            name: "小红书".to_string(),
            slug: "XHS".to_string(),
            color: "#ff2442".to_string(),
            description: "添加并管理多个小红书账号。".to_string(),
            supports_builtin_oauth: true,
        },
        PlatformInfo {
            id: "wechat-channels".to_string(),
            name: "视频号".to_string(),
            slug: "WX".to_string(),
            color: "#ff9f2e".to_string(),
            description: "添加并管理多个微信视频号账号。".to_string(),
            supports_builtin_oauth: true,
        },
        PlatformInfo {
            id: "douyin".to_string(),
            name: "抖音".to_string(),
            slug: "DY".to_string(),
            color: "#111111".to_string(),
            description: "添加并管理多个抖音账号。".to_string(),
            supports_builtin_oauth: true,
        },
        PlatformInfo {
            id: "bilibili".to_string(),
            name: "哔哩哔哩".to_string(),
            slug: "BILI".to_string(),
            color: "#00a1d6".to_string(),
            description: "添加并管理多个 B 站账号。".to_string(),
            supports_builtin_oauth: true,
        },
        PlatformInfo {
            id: "kuaishou".to_string(),
            name: "快手".to_string(),
            slug: "KS".to_string(),
            color: "#ff4906".to_string(),
            description: "添加并管理多个快手账号。".to_string(),
            supports_builtin_oauth: true,
        },
    ]
}

fn platform_name(platform_id: &str) -> &'static str {
    match platform_id {
        "xiaohongshu" | "xhs" => "小红书",
        "wechat-channels" | "wxSph" | "wxsph" => "视频号",
        "douyin" => "抖音",
        "bilibili" => "哔哩哔哩",
        "kuaishou" | "kwai" | "KWAI" => "快手",
        _ => "渠道账号",
    }
}

fn normalize_platform_id(value: &str) -> String {
    match value {
        "xhs" | "Xhs" | "XHS" => "xiaohongshu".to_string(),
        "wxSph" | "wxsph" | "wechat" => "wechat-channels".to_string(),
        "kwai" | "KWAI" | "Kwai" => "kuaishou".to_string(),
        "BILIBILI" => "bilibili".to_string(),
        other => other.to_string(),
    }
}

fn normalize_user_id(value: &str) -> Result<String, String> {
    let user_id = value.trim();
    if user_id.is_empty() {
        return Err("当前登录状态无效，请重新登录".to_string());
    }
    Ok(user_id.to_string())
}

fn account_belongs_to_user(account: &ChannelAccount, user_id: &str) -> bool {
    account.user_id.as_deref() == Some(user_id)
}

fn user_accounts(store: &StoreFile, user_id: &str) -> Vec<ChannelAccount> {
    store
        .accounts
        .iter()
        .filter(|account| account_belongs_to_user(account, user_id))
        .cloned()
        .collect()
}

fn account_secret_for_account(store: &StoreFile, account: &ChannelAccount) -> Option<AccountSecret> {
    account_secret_candidates(account)
        .into_iter()
        .find_map(|key| store.account_secrets.get(&key).cloned())
}

fn migrate_account_secret_for_account(store: &mut StoreFile, account: &ChannelAccount) -> bool {
    let keys = account_secret_candidates(account);
    migrate_account_secret_from_keys(store, &account.id, &keys)
}

fn migrate_account_secret_from_keys(
    store: &mut StoreFile,
    target_id: &str,
    source_keys: &[String],
) -> bool {
    let mut changed = false;
    for key in source_keys {
        if key == target_id {
            continue;
        }
        let Some(source) = store.account_secrets.get(key).cloned() else {
            continue;
        };
        let target = store.account_secrets.entry(target_id.to_string()).or_default();
        if target.login_cookie.is_none() && source.login_cookie.is_some() {
            target.login_cookie = source.login_cookie.clone();
            changed = true;
        }
        if target.webview_session_id.is_none() && source.webview_session_id.is_some() {
            target.webview_session_id = source.webview_session_id.clone();
            changed = true;
        }
    }
    changed
}

fn account_secret_candidates(account: &ChannelAccount) -> Vec<String> {
    let mut values = Vec::new();
    push_unique(&mut values, account.id.clone());
    if let Some(user_id) = account.user_id.as_deref() {
        if let Some(raw_id) = unscoped_account_id(user_id, &account.id) {
            push_unique(&mut values, raw_id);
        }
    }
    if let Some(relay_account_ref) = account.relay_account_ref.as_ref() {
        push_unique(&mut values, relay_account_ref.clone());
        if let Some(user_id) = account.user_id.as_deref() {
            push_unique(&mut values, scoped_account_id(user_id, relay_account_ref));
        }
    }
    values
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.trim().is_empty() && !values.iter().any(|item| item == &value) {
        values.push(value);
    }
}

fn scoped_account_for_user(user_id: &str, mut account: ChannelAccount) -> ChannelAccount {
    account.user_id = Some(user_id.to_string());
    account.id = scoped_account_id(user_id, &account.id);
    account
}

fn scoped_account_id(user_id: &str, account_id: &str) -> String {
    let prefix = format!("u{}_", stable_label_fragment(user_id));
    if account_id.starts_with(&prefix) {
        account_id.to_string()
    } else {
        format!("{prefix}{account_id}")
    }
}

fn unscoped_account_id(user_id: &str, account_id: &str) -> Option<String> {
    let prefix = format!("u{}_", stable_label_fragment(user_id));
    account_id
        .strip_prefix(&prefix)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn aitoearn_platform_id(platform_id: &str) -> Option<&'static str> {
    match platform_id {
        "xiaohongshu" | "xhs" => Some("xhs"),
        "wechat-channels" | "wxSph" | "wxsph" => Some("wxSph"),
        "douyin" => Some("douyin"),
        "bilibili" => Some("bilibili"),
        "kuaishou" | "kwai" | "KWAI" => Some("KWAI"),
        _ => None,
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(ToString::to_string)
    })
}

fn first_string_deep(value: &Value, keys: &[&str]) -> Option<String> {
    if let Some(value) = first_string(value, keys).filter(|value| !value.trim().is_empty()) {
        return Some(value);
    }
    match value {
        Value::Array(items) => items
            .iter()
            .find_map(|item| first_string_deep(item, keys)),
        Value::Object(map) => map
            .values()
            .find_map(|value| first_string_deep(value, keys)),
        _ => None,
    }
}

fn first_count(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(count) = map.get(*key).and_then(|value| parse_count_value(value, keys)) {
                    return Some(count);
                }
            }
            map.values().find_map(|value| first_count(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_count(value, keys)),
        _ => None,
    }
}

fn parse_count_value(value: &Value, keys: &[&str]) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64().or_else(|| {
            number
                .as_f64()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value.round() as u64)
        }),
        Value::String(text) => parse_count_string(text),
        Value::Object(_) | Value::Array(_) => first_count(value, keys),
        _ => None,
    }
}

fn parse_count_string(text: &str) -> Option<u64> {
    let compact: String = text
        .trim()
        .chars()
        .filter(|ch| !matches!(ch, ',' | '，' | ' ' | '\u{00a0}'))
        .collect();
    if compact.is_empty() {
        return None;
    }

    let lower = compact.to_ascii_lowercase();
    let multiplier = if compact.contains('亿') {
        100_000_000.0
    } else if compact.contains('万') || lower.contains('w') {
        10_000.0
    } else if lower.contains('k') {
        1_000.0
    } else {
        1.0
    };

    let mut numeric = String::new();
    let mut started = false;
    let mut saw_dot = false;
    for ch in lower.chars() {
        if ch.is_ascii_digit() {
            numeric.push(ch);
            started = true;
        } else if ch == '.' && !saw_dot {
            numeric.push(ch);
            saw_dot = true;
            started = true;
        } else if started {
            break;
        }
    }

    let value = numeric.parse::<f64>().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some((value * multiplier).round() as u64)
}

fn first_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| value.get(*key).and_then(Value::as_i64))
}

fn encode_query(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn encode_path_segment(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开授权窗口失败: {error}"))
}

fn success_page(nickname: &str) -> String {
    format!(
        r#"<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>授权成功</title><body style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#07181b;color:#dff7ef;display:grid;place-items:center;min-height:100vh"><main style="text-align:center"><h1>授权成功</h1><p>{nickname} 已连接到营销大师。</p><p style="color:#7f969d">可以关闭这个窗口并回到客户端。</p></main></body></html>"#
    )
}

fn error_page(message: &str) -> String {
    format!(
        r#"<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>授权失败</title><body style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#07181b;color:#ffe3e5;display:grid;place-items:center;min-height:100vh"><main style="max-width:560px;text-align:center"><h1>授权没有完成</h1><p>{message}</p><p style="color:#7f969d">请回到客户端查看授权状态。</p></main></body></html>"#
    )
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("内部状态锁定失败: {error}")
}
