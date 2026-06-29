use super::*;
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
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedBrowserCookieSnapshot {
    pub(crate) cookie_header: String,
    pub(crate) login_cookie: String,
    pub(crate) page_url: String,
}

pub(crate) fn creator_home_uses_managed_browser(platform_id: &str) -> bool {
    channels::platform(platform_id).is_some()
}

pub(crate) fn open_managed_browser_login_session(
    app: &AppHandle,
    platform_id: &str,
    task_id: &str,
    login_target: Option<&str>,
) -> Result<CreatorLoginSession, String> {
    let platform_id = normalize_platform_id(platform_id);
    let login_url = plugin_login_url(&platform_id, login_target)
        .ok_or_else(|| "当前平台不支持浏览器授权".to_string())?;
    let platform = channels::platform(&platform_id).ok_or_else(|| "当前平台暂不支持".to_string())?;
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
    command
        .spawn()
        .map_err(|error| format!("启动浏览器登录窗口失败: {error}"))?;

    eprintln!(
        "[managed-auth:{}] opened session={} port={} url={}",
        platform.id,
        session_id,
        remote_debugging_port,
        login_url
    );

    let managed_browser_session = ManagedBrowserAuthSession {
        session_id: session_id.clone(),
        profile_id: task_id.to_string(),
        platform_id: platform.id.to_string(),
        login_url: login_url.to_string(),
        remote_debugging_port,
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
    let Some(platform) = channels::platform(&session.platform_id) else {
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
    let script = r#"
        (async () => {
          try {
            const response = await fetch('https://cp.kuaishou.com/rest/cp/creator/pc/home/infoV2', {
              method: 'POST',
              credentials: 'include',
              headers: {
                'Accept': 'application/json, text/plain, */*',
                'Content-Type': 'application/json;charset=utf-8'
              },
              body: '{}'
            });
            const text = await response.text();
            let data = null;
            try {
              data = JSON.parse(text);
            } catch (error) {
              data = { parseError: String(error), text: text.slice(0, 400) };
            }
            return { ok: response.ok, status: response.status, url: location.href, data };
          } catch (error) {
            return { ok: false, status: 0, url: location.href, error: String(error) };
          }
        })()
    "#;
    let wrapped = managed_browser_eval_json(session, script)?;
    let status = first_i64(&wrapped, &["status"]).unwrap_or(0);
    let ok = wrapped
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(status >= 200 && status < 300);
    let url = wrapped.get("url").and_then(Value::as_str).unwrap_or_default();
    let result_code = wrapped
        .get("data")
        .and_then(|data| first_i64(data, &["result", "code", "errCode", "errcode"]));
    eprintln!(
        "[managed-auth:kuaishou] browser fetch status={status} result={:?} url={}",
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

pub(crate) fn managed_browser_kuaishou_profile_snapshot(
    session: &ManagedBrowserAuthSession,
) -> Result<Value, String> {
    let script = r#"
        (() => {
          const safeText = (value, limit = 160) => String(value || '').replace(/\s+/g, ' ').trim().slice(0, limit);
          const readStorage = (store) => {
            const items = [];
            try {
              for (const key of Object.keys(store || {}).slice(0, 80)) {
                const raw = store.getItem(key);
                if (!raw) continue;
                const item = { key, value: safeText(raw, 500) };
                const trimmed = raw.trim();
                if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
                  try { item.json = JSON.parse(trimmed); } catch (_) {}
                }
                items.push(item);
              }
            } catch (_) {}
            return items;
          };
          const textNodes = Array.from(document.querySelectorAll('[class*="name" i], [class*="nick" i], [class*="user" i], [class*="profile" i], [class*="author" i], [class*="fan" i], [class*="follow" i], [class*="stat" i], [class*="count" i], [class*="num" i], [class*="data" i]'))
            .map((node) => ({
              text: safeText(node.textContent, 120),
              className: safeText(node.className, 160)
            }))
            .filter((item) => item.text && item.text.length <= 80)
            .slice(0, 80);
          const visibleTexts = [];
          try {
            const walker = document.createTreeWalker(document.body || document.documentElement, NodeFilter.SHOW_TEXT);
            let node = null;
            while ((node = walker.nextNode()) && visibleTexts.length < 200) {
              const text = safeText(node.nodeValue, 120);
              if (!text || text.length > 80) continue;
              const parent = node.parentElement;
              if (!parent) continue;
              const style = getComputedStyle(parent);
              if (style.display === 'none' || style.visibility === 'hidden' || Number(style.opacity || 1) === 0) continue;
              visibleTexts.push(text);
            }
          } catch (_) {}
          const images = Array.from(document.images || [])
            .map((img) => ({
              src: img.currentSrc || img.src || '',
              alt: safeText(img.alt, 80),
              className: safeText(img.className, 160)
            }))
            .filter((item) => item.src)
            .slice(0, 80);
          return {
            url: location.href,
            title: document.title,
            bodyText: safeText(document.body && document.body.innerText, 8000),
            textNodes,
            visibleTexts,
            images,
            localStorage: readStorage(localStorage),
            sessionStorage: readStorage(sessionStorage)
          };
        })()
    "#;
    managed_browser_eval_json(session, script)
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

fn browser_page_url(client: &mut DevtoolsClient) -> Result<String, String> {
    let result = client.call(
        "Runtime.evaluate",
        serde_json::json!({
            "expression": "location.href",
            "returnByValue": true,
        }),
    )?;
    Ok(result
        .get("result")
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
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
        "[managed-auth:{}] closing session={}",
        session.platform_id,
        session.session_id
    );
    let Ok(websocket_url) = browser_websocket_url(session.remote_debugging_port) else {
        return;
    };
    let Ok(mut client) = DevtoolsClient::connect(&websocket_url) else {
        return;
    };
    let _ = client.call("Browser.close", serde_json::json!({}));
}

pub(crate) fn open_creator_homepage_managed_browser(
    app: AppHandle,
    account: ChannelAccount,
    saved_login_cookie: Option<String>,
    saved_browser_profile_id: Option<String>,
) -> Result<(), String> {
    let platform_id = normalize_platform_id(&account.platform_id);
    let platform = channels::platform(&platform_id).ok_or_else(|| "当前平台暂不支持".to_string())?;
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
    platform: &channels::ChannelPlatform,
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

fn managed_browser_runtime_dir(
    app: &AppHandle,
    platform: &channels::ChannelPlatform,
    account: &ChannelAccount,
) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建浏览器数据目录: {error}"))?;
    Ok(app_data_dir
        .join("playwright-browser-runtime")
        .join(platform.id)
        .join(stable_label_fragment(&account.id))
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
    let Some(platform) = channels::platform(platform_id) else {
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
    let Some(platform) = channels::platform(platform_id) else {
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

fn wait_for_page_websocket(port: u16, target_url: &str) -> Result<String, String> {
    let started = Instant::now();
    let mut last_error = String::new();
    while started.elapsed() < Duration::from_secs(25) {
        match page_websocket_url(port, target_url) {
            Ok(url) => return Ok(url),
            Err(error) => {
                last_error = error;
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    Err(format!("浏览器调试端口启动超时: {last_error}"))
}

fn page_websocket_url(port: u16, target_url: &str) -> Result<String, String> {
    let body = devtools_http(port, "GET", "/json/list")?;
    let pages = serde_json::from_str::<Value>(&body).map_err(|error| format!("读取浏览器页面失败: {error}"))?;
    let Some(items) = pages.as_array() else {
        return Err("浏览器页面列表格式无效".to_string());
    };
    let target_host = Url::parse(target_url)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()));
    let mut first_page = None;
    let mut blank_page = None;

    for item in items {
        if item.get("type").and_then(Value::as_str) != Some("page") {
            continue;
        }
        let Some(websocket_url) = item.get("webSocketDebuggerUrl").and_then(Value::as_str) else {
            continue;
        };
        let page_url = item.get("url").and_then(Value::as_str).unwrap_or_default();
        if first_page.is_none() {
            first_page = Some(websocket_url.to_string());
        }
        if page_url == "about:blank" && blank_page.is_none() {
            blank_page = Some(websocket_url.to_string());
        }
        if page_url_matches_target(page_url, target_host.as_deref()) {
            return Ok(websocket_url.to_string());
        }
    }

    blank_page
        .or(first_page)
        .ok_or_else(|| "没有找到可控制的浏览器页面".to_string())
}

fn browser_websocket_url(port: u16) -> Result<String, String> {
    let body = devtools_http(port, "GET", "/json/version")?;
    let value = serde_json::from_str::<Value>(&body)
        .map_err(|error| format!("读取浏览器版本信息失败: {error}"))?;
    value
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "浏览器版本信息缺少调试地址".to_string())
}

fn browser_debug_port_closed(error: &str) -> bool {
    error.contains("Connection refused")
        || error.contains("connection refused")
        || error.contains("actively refused")
        || error.contains("No connection")
        || error.contains("连接浏览器失败")
}

fn page_url_matches_target(page_url: &str, target_host: Option<&str>) -> bool {
    let Some(target_host) = target_host else {
        return false;
    };
    let Ok(url) = Url::parse(page_url) else {
        return page_url.contains(target_host);
    };
    let Some(host) = url.host_str().map(|value| value.to_ascii_lowercase()) else {
        return false;
    };
    host == target_host || host.ends_with(&format!(".{target_host}"))
}

fn devtools_http(port: u16, method: &str, path: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).map_err(|error| format!("连接浏览器失败: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| format!("设置浏览器读取超时失败: {error}"))?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("请求浏览器失败: {error}"))?;

    let response = read_http_response(&mut stream)?;
    let Some(header_end) = response.windows(4).position(|item| item == b"\r\n\r\n") else {
        return Err("浏览器响应格式无效".to_string());
    };
    let head = String::from_utf8_lossy(&response[..header_end]);
    if !head.contains(" 200 ") {
        return Err(format!("浏览器响应失败: {}", head.lines().next().unwrap_or_default()));
    }
    let raw_body = &response[header_end + 4..];
    let body = if head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        decode_chunked_body(raw_body)?
    } else {
        raw_body.to_vec()
    };
    String::from_utf8(body).map_err(|error| format!("浏览器响应不是 UTF-8: {error}"))
}

fn read_http_response(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut header_end = None;
    let mut expected_total_len = None;
    let started = Instant::now();

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => {
                response.extend_from_slice(&buffer[..size]);
                if header_end.is_none() {
                    header_end = response.windows(4).position(|item| item == b"\r\n\r\n");
                    if let Some(end) = header_end {
                        let head = String::from_utf8_lossy(&response[..end]);
                        if !head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
                            if let Some(content_len) = http_content_length(&head) {
                                expected_total_len = Some(end + 4 + content_len);
                            }
                        }
                    }
                }

                if let Some(total_len) = expected_total_len {
                    if response.len() >= total_len {
                        break;
                    }
                } else if let Some(end) = header_end {
                    let head = String::from_utf8_lossy(&response[..end]);
                    if head.to_ascii_lowercase().contains("transfer-encoding: chunked")
                        && chunked_body_complete(&response[end + 4..])
                    {
                        break;
                    }
                }
            }
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                if header_end.is_some() {
                    break;
                }
                if started.elapsed() > Duration::from_secs(5) {
                    return Err(format!("读取浏览器响应超时: {error}"));
                }
            }
            Err(error) => return Err(format!("读取浏览器响应失败: {error}")),
        }
    }

    Ok(response)
}

fn http_content_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

fn chunked_body_complete(raw: &[u8]) -> bool {
    let mut offset = 0;
    loop {
        let Some(line_end) = raw[offset..].windows(2).position(|item| item == b"\r\n") else {
            return false;
        };
        let line = String::from_utf8_lossy(&raw[offset..offset + line_end]);
        let size_text = line.split(';').next().unwrap_or("").trim();
        let Ok(size) = usize::from_str_radix(size_text, 16) else {
            return false;
        };
        offset += line_end + 2;
        if size == 0 {
            return raw.len() >= offset + 2;
        }
        if raw.len() < offset + size + 2 {
            return false;
        }
        offset += size + 2;
    }
}

fn decode_chunked_body(raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut offset = 0;

    loop {
        let Some(line_end) = raw[offset..].windows(2).position(|item| item == b"\r\n") else {
            return Err("浏览器 chunked 响应长度格式无效".to_string());
        };
        let line = String::from_utf8_lossy(&raw[offset..offset + line_end]);
        let size_text = line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|error| format!("浏览器 chunked 响应长度无效: {error}"))?;
        offset += line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if raw.len() < offset + size + 2 {
            return Err("浏览器 chunked 响应内容不完整".to_string());
        }
        output.extend_from_slice(&raw[offset..offset + size]);
        offset += size + 2;
    }
}

struct DevtoolsClient {
    stream: TcpStream,
    next_id: u64,
}

impl DevtoolsClient {
    fn connect(websocket_url: &str) -> Result<Self, String> {
        let url = Url::parse(websocket_url).map_err(|error| format!("浏览器调试地址无效: {error}"))?;
        let host = url.host_str().ok_or_else(|| "浏览器调试地址缺少主机".to_string())?;
        let port = url.port().unwrap_or(80);
        let path = if let Some(query) = url.query() {
            format!("{}?{query}", url.path())
        } else {
            url.path().to_string()
        };
        let mut stream = TcpStream::connect((host, port)).map_err(|error| format!("连接浏览器 WebSocket 失败: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| format!("设置浏览器 WebSocket 超时失败: {error}"))?;
        let key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, Uuid::new_v4().as_bytes());
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|error| format!("发送浏览器 WebSocket 握手失败: {error}"))?;

        let mut response = Vec::new();
        let mut buffer = [0_u8; 512];
        loop {
            let size = stream
                .read(&mut buffer)
                .map_err(|error| format!("读取浏览器 WebSocket 握手失败: {error}"))?;
            if size == 0 {
                break;
            }
            response.extend_from_slice(&buffer[..size]);
            if response.windows(4).any(|item| item == b"\r\n\r\n") {
                break;
            }
        }
        let response_text = String::from_utf8_lossy(&response);
        if !response_text.contains(" 101 ") {
            return Err(format!(
                "浏览器 WebSocket 握手失败: {}",
                response_text.lines().next().unwrap_or_default()
            ));
        }

        Ok(Self { stream, next_id: 1 })
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let message = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_text(&message.to_string())?;

        loop {
            let text = self.read_text()?;
            let value = serde_json::from_str::<Value>(&text).map_err(|error| format!("浏览器调试响应无效: {error}"))?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(format!("浏览器调试命令失败 {method}: {error}"));
            }
            return Ok(value.get("result").cloned().unwrap_or_else(|| serde_json::json!({})));
        }
    }

    fn send_text(&mut self, text: &str) -> Result<(), String> {
        let payload = text.as_bytes();
        let mut frame = Vec::new();
        frame.push(0x81);
        if payload.len() <= 125 {
            frame.push(0x80 | payload.len() as u8);
        } else if payload.len() <= u16::MAX as usize {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        let mask = *Uuid::new_v4().as_bytes().first_chunk::<4>().unwrap_or(&[0, 0, 0, 0]);
        frame.extend_from_slice(&mask);
        for (index, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[index % 4]);
        }
        self.stream
            .write_all(&frame)
            .map_err(|error| format!("发送浏览器调试命令失败: {error}"))
    }

    fn read_text(&mut self) -> Result<String, String> {
        loop {
            let mut header = [0_u8; 2];
            self.stream
                .read_exact(&mut header)
                .map_err(|error| format!("读取浏览器调试响应失败: {error}"))?;
            let opcode = header[0] & 0x0f;
            let masked = header[1] & 0x80 != 0;
            let mut len = (header[1] & 0x7f) as u64;
            if len == 126 {
                let mut bytes = [0_u8; 2];
                self.stream
                    .read_exact(&mut bytes)
                    .map_err(|error| format!("读取浏览器调试响应长度失败: {error}"))?;
                len = u16::from_be_bytes(bytes) as u64;
            } else if len == 127 {
                let mut bytes = [0_u8; 8];
                self.stream
                    .read_exact(&mut bytes)
                    .map_err(|error| format!("读取浏览器调试响应长度失败: {error}"))?;
                len = u64::from_be_bytes(bytes);
            }

            let mut mask = [0_u8; 4];
            if masked {
                self.stream
                    .read_exact(&mut mask)
                    .map_err(|error| format!("读取浏览器调试响应掩码失败: {error}"))?;
            }

            let mut payload = vec![0_u8; len as usize];
            self.stream
                .read_exact(&mut payload)
                .map_err(|error| format!("读取浏览器调试响应内容失败: {error}"))?;
            if masked {
                for (index, byte) in payload.iter_mut().enumerate() {
                    *byte ^= mask[index % 4];
                }
            }

            match opcode {
                0x1 => return String::from_utf8(payload).map_err(|error| format!("浏览器调试响应不是 UTF-8: {error}")),
                0x8 => return Err("浏览器调试连接已关闭".to_string()),
                0x9 => self.send_pong(&payload)?,
                _ => {}
            }
        }
    }

    fn send_pong(&mut self, payload: &[u8]) -> Result<(), String> {
        let mut frame = Vec::new();
        frame.push(0x8a);
        frame.push(0x80 | payload.len() as u8);
        let mask = *Uuid::new_v4().as_bytes().first_chunk::<4>().unwrap_or(&[0, 0, 0, 0]);
        frame.extend_from_slice(&mask);
        for (index, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[index % 4]);
        }
        self.stream
            .write_all(&frame)
            .map_err(|error| format!("发送浏览器心跳响应失败: {error}"))
    }
}

fn allocate_local_port() -> Result<u16, String> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("分配浏览器调试端口失败: {error}"))
}

fn find_chromium_browser() -> Option<PathBuf> {
    browser_candidates()
        .into_iter()
        .find(|path| path.exists() && is_executable_file(path))
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(target_os = "macos")]
fn browser_candidates() -> Vec<PathBuf> {
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    let mut paths = vec![
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
        "/Applications/Chromium.app/Contents/MacOS/Chromium".into(),
        "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary".into(),
    ];
    if let Some(home) = home {
        paths.extend([
            home.join("Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            home.join("Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
            home.join("Applications/Chromium.app/Contents/MacOS/Chromium"),
        ]);
    }
    paths
}

#[cfg(target_os = "windows")]
fn browser_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for env_name in ["LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)"] {
        if let Some(root) = std::env::var_os(env_name).map(PathBuf::from) {
            paths.extend([
                root.join("Google/Chrome/Application/chrome.exe"),
                root.join("Microsoft/Edge/Application/msedge.exe"),
                root.join("Chromium/Application/chrome.exe"),
            ]);
        }
    }
    paths
}

#[cfg(all(unix, not(target_os = "macos")))]
fn browser_candidates() -> Vec<PathBuf> {
    vec![
        "/usr/bin/google-chrome".into(),
        "/usr/bin/google-chrome-stable".into(),
        "/usr/bin/microsoft-edge".into(),
        "/usr/bin/chromium".into(),
        "/usr/bin/chromium-browser".into(),
    ]
}
