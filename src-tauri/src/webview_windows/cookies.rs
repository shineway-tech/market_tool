use super::*;

pub(crate) fn inject_douyin_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "douyin", login_cookie, "抖音")
}

pub(crate) fn inject_xhs_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "xiaohongshu", login_cookie, "小红书")
}

pub(crate) fn inject_wx_channels_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "wechat-channels", login_cookie, "视频号")
}

pub(crate) fn inject_bilibili_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "bilibili", login_cookie, "B 站")
}

pub(crate) fn inject_kuaishou_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "kuaishou", login_cookie, "快手")
}

pub(crate) fn inject_platform_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    platform_id: &str,
    login_cookie: &str,
    platform_name: &str,
) -> Result<(), String> {
    let platform = channel_platform(platform_id)?;
    let trimmed = login_cookie.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    if trimmed.starts_with('[') {
        let Value::Array(cookies) =
            serde_json::from_str::<Value>(trimmed).map_err(|error| format!("{platform_name}登录态格式无效: {error}"))?
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
            if !raw_domain.trim().is_empty() && !platform.allows_cookie_domain(raw_domain) {
                continue;
            }
            let domain = if raw_domain.trim().is_empty() {
                platform.default_cookie_domain
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
            platform.default_cookie_domain,
            "/",
            true,
            false,
        )?;
    }
    Ok(())
}

pub(crate) fn navigate_xhs_after_cookie_ready(window: WebviewWindow<tauri::Wry>, url: Url) {
    tauri::async_runtime::spawn(async move {
        let mut ready = false;
        for _ in 0..12 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if xhs_webview_has_login_cookie(&window) {
                ready = true;
                break;
            }
        }
        if !ready {
            eprintln!("[cookie:xhs] key login cookies were not visible before navigation");
        }
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
    });
}

fn xhs_webview_has_login_cookie(window: &WebviewWindow<tauri::Wry>) -> bool {
    let Ok((cookie_header, _)) = collect_webview_cookies(window, channel_cookie_urls("xiaohongshu")) else {
        return false;
    };
    has_xhs_login_cookie_header(&cookie_header)
}

fn has_xhs_login_cookie_header(cookie_header: &str) -> bool {
    cookie_header.split(';').any(|pair| {
        let Some((name, value)) = pair.trim().split_once('=') else {
            return false;
        };
        let name = name.trim();
        !value.trim().is_empty()
            && matches!(
                name,
                "customer-sso-sid"
                    | "access-token-creator.xiaohongshu.com"
                    | "galaxy_creator_session_id"
                    | "galaxy.creator.beaker.session.id"
                    | "x-user-id-creator.xiaohongshu.com"
            )
    })
}

pub(crate) fn set_webview_cookie(
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
        .map_err(|error| format!("注入网页登录态失败: {error}"))
}

pub(crate) fn persist_webview_account_cookies(
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

pub(crate) fn persist_webview_account_cookies_any(
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

pub(crate) fn collect_webview_login_cookie(
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

pub(crate) fn has_douyin_login_cookie(login_cookie: &str) -> bool {
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

pub(crate) fn is_douyin_login_cookie_name(name: &str) -> bool {
    channels::platform("douyin")
        .map(|platform| platform.is_login_cookie_name(name))
        .unwrap_or(false)
}

pub(crate) fn should_inject_douyin_cookie_domain(domain: &str) -> bool {
    channels::platform("douyin")
        .map(|platform| platform.allows_cookie_domain(domain))
        .unwrap_or(false)
}

pub(crate) fn collect_webview_cookies(
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
            cookies.push(serde_json::json!({
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
        cookies.push(serde_json::json!({
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

pub(crate) fn cookie_domain_matches_hosts(domain: &str, hosts: &[String]) -> bool {
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
