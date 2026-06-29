use super::*;

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
