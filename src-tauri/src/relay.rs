use super::*;
use crate::plugin_accounts::{first_profile_image, normalize_platform_image_url};
use reqwest::Client;
use std::{env, fs, path::PathBuf};

const RELAY_SERVER_URL: &str = "https://aitoearn.cn/api";

#[derive(Debug, Clone)]
pub(crate) struct RelaySettings {
    enabled: bool,
    server_url: String,
    api_key: String,
}

#[derive(Debug)]
pub(crate) struct AitoearnAuthSession {
    pub(crate) url: String,
    pub(crate) session_id: String,
    pub(crate) expires_at: Option<String>,
    pub(crate) instructions: Option<String>,
    pub(crate) auth_type: String,
}

pub(crate) fn aitoearn_relay_settings(app: &AppHandle) -> RelaySettings {
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

pub(crate) async fn create_aitoearn_auth_session(
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

pub(crate) async fn fetch_aitoearn_auth_status(
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

pub(crate) async fn fetch_aitoearn_accounts(relay: &RelaySettings) -> Result<Vec<ChannelAccount>, String> {
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

pub(crate) async fn delete_aitoearn_account(
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

pub(crate) fn find_synced_auth_account(
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

pub(crate) fn should_open_relay_auth_in_app(platform_id: &str, auth_type: &str, url: &str) -> bool {
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
            destroy_webview_window(&window);
        }
    }
}

pub(crate) fn close_auth_window_by_label(app: &AppHandle, label: Option<&str>) {
    if let Some(label) = label {
        if let Some(window) = app.get_webview_window(label) {
            destroy_webview_window(&window);
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

pub(crate) fn open_relay_auth_window(
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
        prepare_external_webview_window(&window);
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
        .decorations(true)
        .closable(true)
        .resizable(true)
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
    prepare_external_webview_window(&window);
    let _ = window.clear_all_browsing_data();
    let _ = window.navigate(url);
    Ok(label)
}

pub(crate) fn capture_auth_window_cookies_any(
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

pub(crate) fn aitoearn_platform_id(platform_id: &str) -> Option<&'static str> {
    match normalize_platform_id(platform_id).as_str() {
        "kuaishou" => Some("KWAI"),
        _ => None,
    }
}

fn encode_path_segment(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}
