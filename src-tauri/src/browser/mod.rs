use super::*;

mod cdp;
mod system_browser;

use cdp::{
    browser_debug_port_closed,
    browser_page_url,
    browser_websocket_url,
    page_websocket_url,
    wait_for_page_websocket,
    wait_for_target_page_websocket,
    DevtoolsClient,
};
use system_browser::{allocate_local_port, find_chromium_browser};
use std::{
    io::{ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
struct ManagedBrowserLaunch {
    browser_path: PathBuf,
    user_data_dir: PathBuf,
    url: String,
    platform_id: String,
    login_cookie: Option<String>,
    remote_debugging_port: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedBrowserAuthSession {
    pub(crate) session_id: String,
    pub(crate) profile_id: String,
    pub(crate) platform_id: String,
    pub(crate) login_url: String,
    pub(crate) remote_debugging_port: u16,
    pub(crate) process_id: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedBrowserCookieSnapshot {
    pub(crate) cookie_header: String,
    pub(crate) login_cookie: String,
    pub(crate) page_url: String,
}

pub(crate) fn creator_home_uses_managed_browser(platform_id: &str) -> bool {
    platforms::platform(platform_id).is_some()
}

pub(crate) fn open_managed_browser_login_session(
    app: &AppHandle,
    platform_id: &str,
    task_id: &str,
    login_target: Option<&str>,
) -> Result<CreatorLoginSession, String> {
    let platform_id = normalize_platform_id(platform_id);
    let login_url = platforms::plugin_login_url(&platform_id, login_target)
        .ok_or_else(|| "当前平台不支持浏览器授权".to_string())?;
    let platform = platforms::platform(&platform_id).ok_or_else(|| "当前平台暂不支持".to_string())?;
    let browser_path = find_chromium_browser()
        .ok_or_else(|| "未找到 Chrome、Edge 或 Chromium，无法使用浏览器模式登录。".to_string())?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建浏览器授权目录: {error}"))?;
    let user_data_dir = app_data_dir
        .join("playwright-auth")
        .join(platform.id)
        .join(task_id);
    fs::create_dir_all(&user_data_dir).map_err(|error| format!("创建浏览器授权目录失败: {error}"))?;
    let remote_debugging_port = allocate_local_port()?;
    let session_id = format!(
        "managed-auth-{}-{}",
        platform.id.replace('-', "_"),
        task_suffix(task_id)
    );
    let title = match (platform.id, login_target) {
        ("xiaohongshu", Some("home")) => "登录小红书主页 - 营销大师",
        ("xiaohongshu", Some("creator")) => "登录小红书创作中心 - 营销大师",
        _ => "登录营销平台 - 营销大师",
    };

    let mut command = Command::new(&browser_path);
    command
        .arg(format!("--user-data-dir={}", user_data_dir.display()))
        .arg(format!("--remote-debugging-port={remote_debugging_port}"))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-features=Translate")
        .arg("--new-window")
        .arg(login_url);
    let child = command
        .spawn()
        .map_err(|error| format!("启动浏览器登录窗口失败: {error}"))?;
    let process_id = child.id();

    eprintln!(
        "[managed-auth:{}] opened session={} pid={} port={} url={}",
        platform.id,
        session_id,
        process_id,
        remote_debugging_port,
        login_url
    );

    let managed_browser_session = ManagedBrowserAuthSession {
        session_id: session_id.clone(),
        profile_id: task_id.to_string(),
        platform_id: platform.id.to_string(),
        login_url: login_url.to_string(),
        remote_debugging_port,
        process_id,
    };

    Ok(CreatorLoginSession {
        url: login_url.to_string(),
        session_id,
        managed_browser_session: Some(managed_browser_session),
        expires_at: None,
        instructions: Some(format!(
            "{title}。请在打开的浏览器窗口完成登录，登录成功后客户端会自动同步账号资料。"
        )),
        auth_type: "managed-browser".to_string(),
    })
}

pub(crate) fn managed_browser_cookie_snapshot(
    session: &ManagedBrowserAuthSession,
) -> Result<Option<ManagedBrowserCookieSnapshot>, String> {
    let Some(platform) = platforms::platform(&session.platform_id) else {
        return Err("当前平台暂不支持".to_string());
    };
    let websocket_url = match page_websocket_url(session.remote_debugging_port, &session.login_url) {
        Ok(url) => url,
        Err(error) if browser_debug_port_closed(&error) => {
            return Err("授权浏览器已关闭，请重新打开并完成登录。".to_string());
        }
        Err(error) => return Err(error),
    };
    let mut client = DevtoolsClient::connect(&websocket_url)?;
    client.call("Network.enable", serde_json::json!({}))?;
    let page_url = browser_page_url(&mut client).unwrap_or_default();
    let value = client.call(
        "Network.getCookies",
        serde_json::json!({ "urls": platform.cookie_urls }),
    )?;
    let cookies = value
        .get("cookies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|cookie| {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                return false;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                return false;
            };
            if name.trim().is_empty() || value.trim().is_empty() {
                return false;
            }
            cookie
                .get("domain")
                .and_then(Value::as_str)
                .map(|domain| platform.allows_cookie_domain(domain))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if cookies.is_empty() {
        return Ok(None);
    }

    let cookie_header = cookies
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
    if cookie_header.trim().is_empty() {
        return Ok(None);
    }
    let login_cookie = serde_json::to_string(&cookies)
        .map_err(|error| format!("序列化浏览器 Cookie 失败: {error}"))?;
    let names = cookies
        .iter()
        .filter_map(|cookie| cookie.get("name").and_then(Value::as_str))
        .take(12)
        .collect::<Vec<_>>()
        .join(",");
    eprintln!(
        "[managed-auth:{}] cookie_snapshot count={} names={names}",
        session.platform_id,
        cookies.len()
    );
    Ok(Some(ManagedBrowserCookieSnapshot {
        cookie_header,
        login_cookie,
        page_url,
    }))
}

pub(crate) fn managed_browser_navigate(session: &ManagedBrowserAuthSession, url: &str) -> Result<(), String> {
    let websocket_url = page_websocket_url(session.remote_debugging_port, &session.login_url)?;
    let mut client = DevtoolsClient::connect(&websocket_url)?;
    client.call("Page.navigate", serde_json::json!({ "url": url }))?;
    let _ = client.call("Page.bringToFront", serde_json::json!({}));
    Ok(())
}

pub(crate) fn managed_browser_fetch_kuaishou_home_info(
    session: &ManagedBrowserAuthSession,
) -> Result<Value, String> {
    let api_url = serde_json::to_string(platforms::kuaishou_home_info_api_url())
        .map_err(|error| format!("快手创作者中心接口地址序列化失败: {error}"))?;
    let script = r#"
        (async () => {
          const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
          function getWebpackRequire() {
            if (!window.webpackChunkks_fe_creator_platform) {
              return null;
            }
            let req = null;
            window.webpackChunkks_fe_creator_platform.push([[Date.now()], {}, function (r) { req = r; }]);
            return req;
          }
          async function getKuaishouClient() {
            for (let index = 0; index < 80; index += 1) {
              const req = getWebpackRequire();
              if (req) {
                try {
                  const module = req(37282);
                  const client = module && (module.K || module.A || module.default);
                  if (client && typeof client.post === 'function') {
                    return { client, source: 'module:37282' };
                  }
                } catch (error) {
                  // The module id is build-specific. Fall back to scanning loaded modules below.
                }
                const cache = req.c || {};
                for (const key of Object.keys(cache)) {
                  const exports = cache[key] && cache[key].exports;
                  const client = exports && (exports.K || exports.A || exports.default);
                  if (client && typeof client.post === 'function') {
                    return { client, source: `cache:${key}` };
                  }
                }
              }
              await sleep(250);
            }
            return null;
          }
          try {
            const api = __KUAISHOU_HOME_INFO_API__;
            const signedClient = await getKuaishouClient();
            if (signedClient) {
              const response = await signedClient.client.post(api, {}, {
                universalErrorHandler: false,
                universalLoading: false
              });
              let userInfo = null;
              try {
                const req = getWebpackRequire();
                if (req) {
                  const userModule = req(68135);
                  if (userModule && typeof userModule.ug === 'function') {
                    userInfo = await userModule.ug();
                  }
                }
              } catch (error) {
                userInfo = { fetchError: String(error) };
              }
              const homeData = response && Object.prototype.hasOwnProperty.call(response, 'data')
                ? response.data
                : response;
              return {
                ok: true,
                status: response && response.status ? response.status : 200,
                source: signedClient.source,
                url: location.href,
                data: Object.assign({}, homeData || {}, { userInfo })
              };
            }
            const response = await fetch(api, {
              method: 'POST',
              credentials: 'include',
              headers: {
                'Accept': 'application/json, text/plain, */*',
                'Content-Type': 'application/json;charset=utf-8'
              },
              body: '{}'
            });
            const text = await response.text();
            const data = JSON.parse(text);
            return {
              ok: response.ok,
              status: response.status,
              source: 'fetch',
              url: location.href,
              data
            };
          } catch (error) {
            return { ok: false, status: 0, url: location.href, error: String(error) };
          }
        })()
    "#
    .replace("__KUAISHOU_HOME_INFO_API__", &api_url);
    let wrapped = managed_browser_eval_json(session, &script)?;
    let status = first_i64(&wrapped, &["status"]).unwrap_or(0);
    let ok = wrapped
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(status >= 200 && status < 300);
    let url = wrapped.get("url").and_then(Value::as_str).unwrap_or_default();
    let source = wrapped.get("source").and_then(Value::as_str).unwrap_or_default();
    let result_code = wrapped
        .get("data")
        .and_then(|data| first_i64(data, &["result", "code", "errCode", "errcode"]));
    eprintln!(
        "[managed-auth:kuaishou] browser fetch source={source} status={status} result={:?} url={}",
        result_code,
        sanitize_sensitive_url_for_log(url)
    );
    if !ok {
        return Err("请先在打开的快手创作者中心完成登录。".to_string());
    }
    wrapped
        .get("data")
        .cloned()
        .ok_or_else(|| "快手浏览器状态缺少账号资料".to_string())
}

pub(crate) fn managed_browser_fetch_kuaishou_api(
    session: &ManagedBrowserAuthSession,
    api_url: &str,
    body: Value,
) -> Result<Value, String> {
    let script = kuaishou_client_post_script(api_url, body)?;
    let wrapped = managed_browser_eval_json(session, &script)?;
    let status = first_i64(&wrapped, &["status"]).unwrap_or(0);
    let ok = wrapped
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(status >= 200 && status < 300);
    let url = wrapped.get("url").and_then(Value::as_str).unwrap_or_default();
    let source = wrapped.get("source").and_then(Value::as_str).unwrap_or_default();
    let result_code = wrapped
        .get("data")
        .and_then(|data| first_i64(data, &["result", "code", "errCode", "errcode"]));
    eprintln!(
        "[managed-api:kuaishou] browser fetch source={source} status={status} result={:?} url={}",
        result_code,
        sanitize_sensitive_url_for_log(url)
    );
    if ok {
        wrapped
            .get("data")
            .cloned()
            .ok_or_else(|| "快手页面客户端请求缺少数据".to_string())
    } else {
        let message = wrapped
            .get("error")
            .and_then(Value::as_str)
            .or_else(|| wrapped.get("message").and_then(Value::as_str))
            .unwrap_or("快手页面客户端请求失败");
        Err(message.to_string())
    }
}

pub(crate) fn managed_browser_fetch_kuaishou_api_with_cookie_headless(
    login_cookie: &str,
    page_url: &str,
    api_url: &str,
    body: Value,
) -> Result<Value, String> {
    let platform = platforms::platform("kuaishou").ok_or_else(|| "当前平台暂不支持".to_string())?;
    let browser_path = find_chromium_browser()
        .ok_or_else(|| "未找到 Chrome、Edge 或 Chromium，无法使用浏览器模式同步快手数据。".to_string())?;
    let user_data_dir = std::env::temp_dir().join(format!("market-tool-ks-sync-{}", Uuid::new_v4()));
    fs::create_dir_all(&user_data_dir).map_err(|error| format!("创建快手临时浏览器目录失败: {error}"))?;

    let remote_debugging_port = allocate_local_port()?;
    let mut command = Command::new(&browser_path);
    command
        .arg(format!("--user-data-dir={}", user_data_dir.display()))
        .arg(format!("--remote-debugging-port={remote_debugging_port}"))
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-features=Translate")
        .arg("--window-size=1280,900")
        .arg("about:blank");
    let child = command
        .spawn()
        .map_err(|error| format!("启动快手后台数据同步失败: {error}"))?;
    let session = ManagedBrowserAuthSession {
        session_id: format!("managed-api-kuaishou-cookie-{}", task_suffix(&Uuid::new_v4().to_string())),
        profile_id: String::new(),
        platform_id: platform.id.to_string(),
        login_url: page_url.to_string(),
        remote_debugging_port,
        process_id: child.id(),
    };

    let result = (|| {
        let websocket_url = wait_for_page_websocket(remote_debugging_port, "about:blank")?;
        let mut client = DevtoolsClient::connect(&websocket_url)?;
        client.call("Network.enable", serde_json::json!({}))?;
        client.call("Page.enable", serde_json::json!({}))?;
        let cookies = login_cookie_to_cdp_cookies(platform.id, login_cookie)?;
        let (written, failed) = set_cdp_cookies(&mut client, &cookies);
        eprintln!(
            "[managed-api:kuaishou] temp cookie_write written={} failed={}",
            written, failed
        );
        if !cookies.is_empty() && written == 0 {
            return Err("快手登录 Cookie 写入后台浏览器失败".to_string());
        }
        client.call("Page.navigate", serde_json::json!({ "url": page_url }))?;
        wait_for_target_page_websocket(remote_debugging_port, page_url)?;
        std::thread::sleep(Duration::from_millis(1_500));
        managed_browser_fetch_kuaishou_api(&session, api_url, body)
    })();
    close_managed_browser_auth_session(&session);
    let _ = fs::remove_dir_all(&user_data_dir);
    result
}

fn kuaishou_client_post_script(api_url: &str, body: Value) -> Result<String, String> {
    let api_url = serde_json::to_string(api_url)
        .map_err(|error| format!("快手接口地址序列化失败: {error}"))?;
    let body = serde_json::to_string(&body)
        .map_err(|error| format!("快手接口参数序列化失败: {error}"))?;
    Ok(r#"
        (async () => {
          const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
          function getWebpackRequire() {
            if (!window.webpackChunkks_fe_creator_platform) {
              return null;
            }
            let req = null;
            window.webpackChunkks_fe_creator_platform.push([[Date.now()], {}, function (r) { req = r; }]);
            return req;
          }
          async function getKuaishouClient() {
            for (let index = 0; index < 100; index += 1) {
              const req = getWebpackRequire();
              if (req) {
                try {
                  const module = req(37282);
                  const client = module && (module.K || module.A || module.default);
                  if (client && typeof client.post === 'function') {
                    return { client, source: 'module:37282' };
                  }
                } catch (error) {
                  // Module ids can change between builds; scan the loaded cache below.
                }
                const cache = req.c || {};
                for (const key of Object.keys(cache)) {
                  const exports = cache[key] && cache[key].exports;
                  const client = exports && (exports.K || exports.A || exports.default);
                  if (client && typeof client.post === 'function') {
                    return { client, source: `cache:${key}` };
                  }
                }
              }
              await sleep(250);
            }
            return null;
          }
          try {
            const api = __KUAISHOU_API_URL__;
            let signedApi = api;
            try {
              const parsed = new URL(api, location.href);
              if (parsed.origin === location.origin) {
                signedApi = `${parsed.pathname}${parsed.search}`;
              }
            } catch (error) {
              // Keep the original URL when URL parsing is unavailable or unexpected.
            }
            const payload = __KUAISHOU_API_BODY__;
            const signedClient = await getKuaishouClient();
            if (signedClient) {
              const response = await signedClient.client.post(signedApi, payload, {
                universalErrorHandler: false,
                universalLoading: false
              });
              return {
                ok: true,
                status: response && response.status ? response.status : 200,
                source: signedClient.source,
                url: location.href,
                data: response && Object.prototype.hasOwnProperty.call(response, 'data') ? response.data : response
              };
            }
            const response = await fetch(api, {
              method: 'POST',
              credentials: 'include',
              headers: {
                'Accept': 'application/json, text/plain, */*',
                'Content-Type': 'application/json;charset=utf-8'
              },
              body: JSON.stringify(payload)
            });
            const text = await response.text();
            return {
              ok: response.ok,
              status: response.status,
              source: 'fetch',
              url: location.href,
              data: JSON.parse(text)
            };
          } catch (error) {
            return { ok: false, status: 0, url: location.href, error: String(error) };
          }
        })()
    "#
    .replace("__KUAISHOU_API_URL__", &api_url)
    .replace("__KUAISHOU_API_BODY__", &body))
}

pub(crate) fn managed_browser_fetch_kuaishou_home_info_headless(
    app: &AppHandle,
    profile_id: &str,
) -> Result<(Value, Option<ManagedBrowserCookieSnapshot>), String> {
    let platform = platforms::platform("kuaishou").ok_or_else(|| "当前平台暂不支持".to_string())?;
    let browser_path = find_chromium_browser()
        .ok_or_else(|| "未找到 Chrome、Edge 或 Chromium，无法使用浏览器模式同步快手账号。".to_string())?;
    let user_data_dir = managed_browser_auth_profile_dir(app, platform, profile_id)?;
    if !user_data_dir.exists() {
        return Err("快手浏览器登录目录不存在，请重新登录。".to_string());
    }

    let remote_debugging_port = allocate_local_port()?;
    let mut command = Command::new(&browser_path);
    command
        .arg(format!("--user-data-dir={}", user_data_dir.display()))
        .arg(format!("--remote-debugging-port={remote_debugging_port}"))
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-features=Translate")
        .arg("--window-size=1280,900")
        .arg(platform.creator_home_url);
    let child = command
        .spawn()
        .map_err(|error| format!("启动快手后台资料同步失败: {error}"))?;
    let session = ManagedBrowserAuthSession {
        session_id: format!("managed-sync-kuaishou-{}", task_suffix(profile_id)),
        profile_id: profile_id.to_string(),
        platform_id: platform.id.to_string(),
        login_url: platform.creator_home_url.to_string(),
        remote_debugging_port,
        process_id: child.id(),
    };

    let result = (|| {
        wait_for_target_page_websocket(remote_debugging_port, platform.creator_home_url)?;
        let value = managed_browser_fetch_kuaishou_home_info_with_retry(&session)?;
        let snapshot = managed_browser_cookie_snapshot(&session).ok().flatten();
        Ok((value, snapshot))
    })();
    close_managed_browser_auth_session(&session);
    result
}

fn managed_browser_eval_json(session: &ManagedBrowserAuthSession, expression: &str) -> Result<Value, String> {
    let websocket_url = page_websocket_url(session.remote_debugging_port, &session.login_url)?;
    let mut client = DevtoolsClient::connect(&websocket_url)?;
    let result = client.call(
        "Runtime.evaluate",
        serde_json::json!({
            "expression": expression,
            "awaitPromise": true,
            "returnByValue": true,
        }),
    )?;
    if let Some(exception) = result.get("exceptionDetails") {
        return Err(format!("浏览器脚本执行失败: {exception}"));
    }
    result
        .get("result")
        .and_then(|value| value.get("value"))
        .cloned()
        .ok_or_else(|| "浏览器脚本没有返回结果".to_string())
}

pub(crate) fn managed_browser_fetch_kuaishou_home_info_with_retry(
    session: &ManagedBrowserAuthSession,
) -> Result<Value, String> {
    let mut last_error = String::new();
    for attempt in 0..4 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(1_200));
        }
        match managed_browser_fetch_kuaishou_home_info(session) {
            Ok(value) => return Ok(value),
            Err(error) if browser_page_eval_retryable(&error) => {
                last_error = error;
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

fn browser_page_eval_retryable(error: &str) -> bool {
    error.contains("Execution context was destroyed")
        || error.contains("Cannot find context with specified id")
        || error.contains("Inspected target navigated or closed")
        || error.contains("浏览器脚本没有返回结果")
}

fn sanitize_sensitive_url_for_log(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw) else {
        return raw.to_string();
    };
    let pairs = url
        .query_pairs()
        .map(|(key, value)| {
            let sensitive = [
                "authToken",
                "token",
                "access_token",
                "refresh_token",
                "passToken",
                "captchaToken",
            ]
            .iter()
            .any(|item| key.eq_ignore_ascii_case(item));
            if sensitive {
                (key.into_owned(), "***".to_string())
            } else {
                (key.into_owned(), value.into_owned())
            }
        })
        .collect::<Vec<_>>();
    url.set_query(None);
    if !pairs.is_empty() {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(&key, &value);
        }
    }
    url.to_string()
}

pub(crate) fn close_managed_browser_auth_session(session: &ManagedBrowserAuthSession) {
    eprintln!(
        "[managed-auth:{}] closing session={} pid={} port={}",
        session.platform_id,
        session.session_id,
        session.process_id,
        session.remote_debugging_port
    );
    if let Ok(websocket_url) = browser_websocket_url(session.remote_debugging_port) {
        if let Ok(mut client) = DevtoolsClient::connect(&websocket_url) {
            let _ = client.call("Browser.close", serde_json::json!({}));
        }
    }
    if wait_for_browser_debug_port_closed(session.remote_debugging_port, Duration::from_secs(4)) {
        return;
    }
    terminate_managed_browser_process(session.process_id, &session.platform_id);
    let _ = wait_for_browser_debug_port_closed(session.remote_debugging_port, Duration::from_secs(2));
}

fn wait_for_browser_debug_port_closed(port: u16, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if browser_websocket_url(port).is_err() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    false
}

#[cfg(target_os = "windows")]
fn terminate_managed_browser_process(process_id: u32, platform_id: &str) {
    if process_id == 0 {
        return;
    }
    let status = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .status();
    eprintln!("[managed-auth:{platform_id}] taskkill pid={process_id} status={status:?}");
}

#[cfg(not(target_os = "windows"))]
fn terminate_managed_browser_process(process_id: u32, platform_id: &str) {
    if process_id == 0 {
        return;
    }
    let status = Command::new("kill")
        .args(["-TERM", &process_id.to_string()])
        .status();
    eprintln!("[managed-auth:{platform_id}] kill -TERM pid={process_id} status={status:?}");
    std::thread::sleep(Duration::from_millis(500));
    let _ = Command::new("kill")
        .args(["-KILL", &process_id.to_string()])
        .status();
}

pub(crate) fn open_creator_homepage_managed_browser(
    app: AppHandle,
    account: ChannelAccount,
    saved_login_cookie: Option<String>,
    saved_browser_profile_id: Option<String>,
) -> Result<(), String> {
    let platform_id = normalize_platform_id(&account.platform_id);
    let platform = platforms::platform(&platform_id).ok_or_else(|| "当前平台暂不支持".to_string())?;
    let browser_path = find_chromium_browser()
        .ok_or_else(|| "未找到 Chrome、Edge 或 Chromium，无法使用浏览器模式打开主页。".to_string())?;
    let profile_dir = saved_browser_profile_id
        .as_deref()
        .and_then(|profile_id| managed_browser_auth_profile_dir(&app, platform, profile_id).ok())
        .filter(|path| path.exists());
    let profile_reused = profile_dir.is_some();
    let user_data_dir = match profile_dir {
        Some(path) => path,
        None => managed_browser_runtime_dir(&app, platform, &account)?,
    };
    fs::create_dir_all(&user_data_dir).map_err(|error| format!("创建浏览器用户目录失败: {error}"))?;

    let launch = ManagedBrowserLaunch {
        browser_path,
        user_data_dir,
        url: platform.creator_home_url.to_string(),
        platform_id,
        login_cookie: if profile_reused {
            None
        } else {
            saved_login_cookie.filter(|value| !value.trim().is_empty())
        },
        remote_debugging_port: allocate_local_port()?,
    };

    std::thread::spawn(move || {
        if let Err(error) = launch_managed_browser(launch) {
            eprintln!("[managed-browser] open creator homepage failed: {error}");
        }
    });

    Ok(())
}

fn managed_browser_auth_profile_dir(
    app: &AppHandle,
    platform: &platforms::ChannelPlatform,
    profile_id: &str,
) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法读取浏览器授权目录: {error}"))?;
    Ok(app_data_dir
        .join("playwright-auth")
        .join(platform.id)
        .join(profile_id))
}

pub(crate) fn delete_managed_browser_account_data(
    app: &AppHandle,
    account: &ChannelAccount,
    profile_ids: &[String],
) -> Result<(), String> {
    let platform_id = normalize_platform_id(&account.platform_id);
    let Some(platform) = platforms::platform(&platform_id) else {
        return Ok(());
    };

    let mut paths = Vec::new();
    for profile_id in profile_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        push_unique_path(
            &mut paths,
            managed_browser_auth_profile_dir(app, platform, profile_id)?,
        );
    }
    push_unique_path(
        &mut paths,
        managed_browser_runtime_account_dir(app, platform, account)?,
    );

    for path in paths {
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|error| {
                format!("清理{}浏览器本地数据失败: {error}", platform.name)
            })?;
        }
    }
    Ok(())
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|item| item == &path) {
        paths.push(path);
    }
}

fn managed_browser_runtime_account_dir(
    app: &AppHandle,
    platform: &platforms::ChannelPlatform,
    account: &ChannelAccount,
) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建浏览器数据目录: {error}"))?;
    Ok(app_data_dir
        .join("playwright-browser-runtime")
        .join(platform.id)
        .join(stable_label_fragment(&account.id)))
}

fn managed_browser_runtime_dir(
    app: &AppHandle,
    platform: &platforms::ChannelPlatform,
    account: &ChannelAccount,
) -> Result<PathBuf, String> {
    Ok(managed_browser_runtime_account_dir(app, platform, account)?
        .join(Uuid::new_v4().to_string()))
}

fn launch_managed_browser(launch: ManagedBrowserLaunch) -> Result<(), String> {
    eprintln!(
        "[managed-browser:{}] open url={} cookie_present={} cookie_chars={}",
        launch.platform_id,
        launch.url,
        launch.login_cookie.is_some(),
        launch.login_cookie.as_ref().map(|value| value.len()).unwrap_or(0)
    );
    let mut command = Command::new(&launch.browser_path);
    command
        .arg(format!("--user-data-dir={}", launch.user_data_dir.display()))
        .arg(format!("--remote-debugging-port={}", launch.remote_debugging_port))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-features=Translate")
        .arg("--new-window")
        .arg(&launch.url);

    command
        .spawn()
        .map_err(|error| format!("启动浏览器失败: {error}"))?;

    if let Some(login_cookie) = launch.login_cookie.as_deref() {
        inject_cookies_and_navigate(&launch, login_cookie)?;
    }

    Ok(())
}

fn inject_cookies_and_navigate(launch: &ManagedBrowserLaunch, login_cookie: &str) -> Result<(), String> {
    let websocket_url = wait_for_page_websocket(launch.remote_debugging_port, &launch.url)?;
    let mut client = DevtoolsClient::connect(&websocket_url)?;
    let cookies = login_cookie_to_cdp_cookies(&launch.platform_id, login_cookie)?;
    eprintln!(
        "[managed-browser:{}] cdp connected cookie_candidates={}",
        launch.platform_id,
        cookies.len()
    );

    client.call("Network.enable", serde_json::json!({}))?;
    if !cookies.is_empty() {
        let (written, failed) = set_cdp_cookies(&mut client, &cookies);
        eprintln!(
            "[managed-browser:{}] cookie_write written={} failed={}",
            launch.platform_id,
            written,
            failed
        );
        if written == 0 {
            return Err("登录 Cookie 写入浏览器失败".to_string());
        }
        log_browser_cookie_snapshot(&mut client, &launch.platform_id);
    }
    client.call("Page.navigate", serde_json::json!({ "url": launch.url }))?;
    let _ = client.call("Page.bringToFront", serde_json::json!({}));
    Ok(())
}

fn set_cdp_cookies(client: &mut DevtoolsClient, cookies: &[Value]) -> (usize, usize) {
    let mut written = 0;
    let mut failed = 0;
    for cookie in cookies {
        match client.call("Network.setCookie", cookie.clone()) {
            Ok(result) if result.get("success").and_then(Value::as_bool).unwrap_or(true) => {
                written += 1;
            }
            Ok(_) | Err(_) => {
                failed += 1;
            }
        }
    }
    (written, failed)
}

fn log_browser_cookie_snapshot(client: &mut DevtoolsClient, platform_id: &str) {
    let Some(platform) = platforms::platform(platform_id) else {
        return;
    };
    let result = client.call(
        "Network.getCookies",
        serde_json::json!({ "urls": platform.cookie_urls }),
    );
    let Ok(value) = result else {
        eprintln!("[managed-browser:{platform_id}] cookie_snapshot failed");
        return;
    };
    let names = value
        .get("cookies")
        .and_then(Value::as_array)
        .map(|items| {
            let mut names = Vec::new();
            for item in items {
                let Some(name) = item.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if !names.iter().any(|existing| existing == name) {
                    names.push(name.to_string());
                }
            }
            names
        })
        .unwrap_or_default();
    let preview = names.iter().take(12).cloned().collect::<Vec<_>>().join(",");
    eprintln!(
        "[managed-browser:{platform_id}] cookie_snapshot count={} names={preview}",
        names.len()
    );
}

fn login_cookie_to_cdp_cookies(platform_id: &str, login_cookie: &str) -> Result<Vec<Value>, String> {
    let Some(platform) = platforms::platform(platform_id) else {
        return Ok(Vec::new());
    };
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') {
        let Value::Array(cookies) =
            serde_json::from_str::<Value>(trimmed).map_err(|error| format!("登录态 Cookie 格式无效: {error}"))?
        else {
            return Ok(Vec::new());
        };

        let mut result = Vec::new();
        for cookie in cookies {
            let Some(name) = cookie.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = cookie.get("value").and_then(Value::as_str) else {
                continue;
            };
            if name.trim().is_empty() {
                continue;
            }
            let raw_domain = cookie.get("domain").and_then(Value::as_str).unwrap_or("");
            if !raw_domain.trim().is_empty() && !platform.allows_cookie_domain(raw_domain) {
                continue;
            }
            let mut item = serde_json::json!({
                "url": platform.creator_home_url,
                "name": name,
                "value": value,
                "domain": if raw_domain.trim().is_empty() { platform.default_cookie_domain } else { raw_domain },
                "path": cookie.get("path").and_then(Value::as_str).unwrap_or("/"),
                "secure": cookie.get("secure").and_then(Value::as_bool).unwrap_or(true),
                "httpOnly": cookie
                    .get("httpOnly")
                    .or_else(|| cookie.get("http_only"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
            if let Some(expires) = cookie_number(&cookie, &["expires", "expirationDate", "expiry"]) {
                if let Some(expires) = normalize_cookie_expires(expires) {
                    item["expires"] = serde_json::json!(expires);
                }
            }
            result.push(item);
        }
        return Ok(result);
    }

    let mut result = Vec::new();
    for pair in trimmed.split(';') {
        let pair = pair.trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        if name.trim().is_empty() {
            continue;
        }
        result.push(serde_json::json!({
            "url": platform.creator_home_url,
            "name": name.trim(),
            "value": value.trim(),
            "domain": platform.default_cookie_domain,
            "path": "/",
            "secure": true,
            "httpOnly": false,
        }));
    }
    Ok(result)
}

fn normalize_cookie_expires(value: f64) -> Option<f64> {
    if value <= 0.0 {
        return None;
    }
    let seconds = if value > 10_000_000_000.0 { value / 1000.0 } else { value };
    if seconds > 0.0 && seconds < 253_402_300_799.0 {
        Some(seconds)
    } else {
        None
    }
}

fn cookie_number(cookie: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        cookie
            .get(*key)
            .and_then(|value| value.as_f64().or_else(|| value.as_i64().map(|item| item as f64)))
    })
}
