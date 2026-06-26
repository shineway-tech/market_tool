use super::*;
use chrono::Duration;
use reqwest::Client;
use serde_json::json;
use std::{io, sync::atomic::Ordering};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const CALLBACK_PORT_START: u16 = 17654;
const CALLBACK_PORT_END: u16 = 17674;

pub(crate) fn create_direct_oauth_url(
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

pub(crate) async fn ensure_callback_server(
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
