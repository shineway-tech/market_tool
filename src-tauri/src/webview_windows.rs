use super::*;

pub(super) fn channel_platform(platform_id: &str) -> Result<&'static channels::ChannelPlatform, String> {
    channels::platform(platform_id).ok_or_else(|| "当前平台暂不支持".to_string())
}

pub(super) fn creator_home_url(platform_id: &str, label: &str) -> Result<Url, String> {
    let platform = channel_platform(platform_id)?;
    Url::parse(platform.creator_home_url).map_err(|error| format!("{label}地址无效: {error}"))
}

pub(super) fn channel_cookie_urls(platform_id: &str) -> &'static [&'static str] {
    channels::platform(platform_id)
        .map(|item| item.cookie_urls)
        .unwrap_or(&[])
}

pub(super) fn channel_web_url(platform_id: &str, url: &Url) -> bool {
    channels::platform(platform_id)
        .map(|item| item.matches_web_url(url))
        .unwrap_or(false)
}

pub(super) fn account_homepage_url(account: &ChannelAccount) -> Result<String, String> {
    channel_platform(&account.platform_id)?.homepage_url(&account.uid, &account.nickname)
}

fn ensure_close_controls(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.set_decorations(true);
    let _ = window.set_closable(true);
    let _ = window.set_resizable(true);
}

fn force_destroy_on_close(window: &WebviewWindow<tauri::Wry>) {
    let window_for_close = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
            let _ = window_for_close.destroy();
        }
    });
}

pub(super) fn open_douyin_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = creator_home_url("douyin", "抖音创作者中心")?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-douyin-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "抖音账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 抖音创作者中心");

    if let Some(window) = app.get_webview_window(&label) {
        ensure_close_controls(&window);
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
        .decorations(true)
        .closable(true)
        .resizable(true)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                handle_creator_home_page_load(&app_for_load, &window, &account_id, "douyin", payload.url());
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开抖音创作者中心失败: {error}"))?;
    ensure_close_controls(&window);
    force_destroy_on_close(&window);

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_douyin_login_cookie(&window, login_cookie);
        navigate_webview_after_delay(window.clone(), url);
    }

    Ok(())
}

pub(super) fn navigate_webview_after_delay(window: WebviewWindow<tauri::Wry>, url: Url) {
    tauri::async_runtime::spawn(async move {
        std::thread::sleep(std::time::Duration::from_millis(600));
        let _ = window.navigate(url);
        let _ = window.show();
        let _ = window.set_focus();
    });
}

fn handle_creator_home_page_load(
    app: &AppHandle,
    window: &WebviewWindow<tauri::Wry>,
    account_id: &str,
    platform_id: &str,
    url: &Url,
) {
    if creator_home_requires_login(platform_id, url) {
        if normalize_platform_id(platform_id) != "xiaohongshu" {
            let _ = mark_account_expired(app, account_id);
        }
        return;
    }

    if !channel_web_url(platform_id, url) {
        return;
    }

    let result = if normalize_platform_id(platform_id) == "douyin" {
        persist_webview_account_cookies(app, window, account_id, channel_cookie_urls(platform_id))
    } else {
        persist_webview_account_cookies_any(app, window, account_id, channel_cookie_urls(platform_id))
    };
    if let Err(error) = result {
        eprintln!("[creator-home:{platform_id}] cookie persistence failed for {account_id}: {error}");
    }
}

fn creator_home_requires_login(platform_id: &str, url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let path = url.path().to_ascii_lowercase();
    let query = url.query().unwrap_or_default().to_ascii_lowercase();
    let text = format!("{host}{path}?{query}");

    match normalize_platform_id(platform_id).as_str() {
        "douyin" => {
            host.contains("passport.douyin.com")
                || host.contains("sso.douyin.com")
                || text.contains("login")
                || text.contains("passport")
        }
        "xiaohongshu" => {
            host.contains("login.xiaohongshu.com")
                || text.contains("login")
                || text.contains("signin")
        }
        "wechat-channels" => {
            text.contains("login")
                || text.contains("qrcode")
                || text.contains("not_login")
                || text.contains("notlogin")
        }
        "bilibili" => {
            host.contains("passport.bilibili.com")
                || host.contains("account.bilibili.com")
                || text.contains("login")
        }
        "kuaishou" => {
            host.contains("passport.kuaishou.com")
                || host.contains("id.kuaishou.com")
                || text.contains("login")
                || text.contains("passport")
        }
        _ => false,
    }
}

pub(super) fn open_xhs_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = creator_home_url("xiaohongshu", "小红书创作中心")?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-xhs-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "小红书账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 小红书创作中心");

    if let Some(window) = app.get_webview_window(&label) {
        ensure_close_controls(&window);
        if let Some(login_cookie) = saved_login_cookie {
            let _ = inject_xhs_login_cookie(&window, login_cookie);
        }
        let _ = window.set_title(&title);
        if saved_login_cookie.is_some() {
            navigate_xhs_after_cookie_ready(window.clone(), url.clone());
        } else {
            let _ = window.navigate(url);
        }
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
    let initial_url = if saved_login_cookie.is_some() {
        Url::parse("about:blank").map_err(|error| format!("空白页地址无效: {error}"))?
    } else {
        url.clone()
    };

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(initial_url))
        .title(&title)
        .decorations(true)
        .closable(true)
        .resizable(true)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(saved_login_cookie.is_none())
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                handle_creator_home_page_load(&app_for_load, &window, &account_id, "xiaohongshu", payload.url());
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开小红书创作中心失败: {error}"))?;
    ensure_close_controls(&window);
    force_destroy_on_close(&window);
    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_xhs_login_cookie(&window, login_cookie);
        navigate_xhs_after_cookie_ready(window.clone(), url);
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

pub(super) fn open_wx_channels_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = creator_home_url("wechat-channels", "视频号后台")?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-wx-sph-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "视频号账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 视频号后台");

    if let Some(window) = app.get_webview_window(&label) {
        ensure_close_controls(&window);
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
        .decorations(true)
        .closable(true)
        .resizable(true)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                handle_creator_home_page_load(&app_for_load, &window, &account_id, "wechat-channels", payload.url());
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开视频号后台失败: {error}"))?;
    ensure_close_controls(&window);
    force_destroy_on_close(&window);
    let _ = window.show();
    let _ = window.set_focus();

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_wx_channels_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(())
}

pub(super) fn open_bilibili_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = creator_home_url("bilibili", "B 站创作中心")?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-bilibili-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "B 站账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - B 站创作中心");

    if let Some(window) = app.get_webview_window(&label) {
        ensure_close_controls(&window);
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
                .join("plugin-auth")
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
        .decorations(true)
        .closable(true)
        .resizable(true)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                handle_creator_home_page_load(&app_for_load, &window, &account_id, "bilibili", payload.url());
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开 B 站创作中心失败: {error}"))?;
    ensure_close_controls(&window);
    force_destroy_on_close(&window);
    let _ = window.show();
    let _ = window.set_focus();

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_bilibili_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(())
}

pub(super) fn open_kuaishou_creator_webview(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let url = creator_home_url("kuaishou", "快手创作者中心")?;
    let window_key = stable_label_fragment(&account.id);
    let label = format!("creator-home-kuaishou-{window_key}");
    let title_name = if account.nickname.trim().is_empty() {
        "快手账号"
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - 快手创作者中心");

    if let Some(window) = app.get_webview_window(&label) {
        ensure_close_controls(&window);
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
        .decorations(true)
        .closable(true)
        .resizable(true)
        .inner_size(1180.0, 820.0)
        .min_inner_size(980.0, 680.0)
        .visible(true)
        .focused(true)
        .focusable(true)
        .data_directory(data_dir)
        .data_store_identifier(data_store_identifier)
        .user_agent(DESKTOP_CHROME_UA)
        .on_page_load(move |window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                handle_creator_home_page_load(&app_for_load, &window, &account_id, "kuaishou", payload.url());
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开快手创作者中心失败: {error}"))?;
    ensure_close_controls(&window);
    force_destroy_on_close(&window);
    let _ = window.show();
    let _ = window.set_focus();

    if let Some(login_cookie) = saved_login_cookie {
        let _ = inject_kuaishou_login_cookie(&window, login_cookie);
        let _ = window.navigate(url);
    }

    Ok(())
}

pub(super) fn inject_douyin_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "douyin", login_cookie, "抖音")
}

pub(super) fn inject_xhs_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "xiaohongshu", login_cookie, "小红书")
}

pub(super) fn inject_wx_channels_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "wechat-channels", login_cookie, "视频号")
}

pub(super) fn inject_bilibili_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "bilibili", login_cookie, "B 站")
}

pub(super) fn inject_kuaishou_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    login_cookie: &str,
) -> Result<(), String> {
    inject_platform_login_cookie(window, "kuaishou", login_cookie, "快手")
}

pub(super) fn inject_platform_login_cookie(
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

fn navigate_xhs_after_cookie_ready(window: WebviewWindow<tauri::Wry>, url: Url) {
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

pub(super) fn set_webview_cookie(
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

pub(super) fn persist_webview_account_cookies(
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

pub(super) fn persist_webview_account_cookies_any(
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

pub(super) fn collect_webview_login_cookie(
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

pub(super) fn has_douyin_login_cookie(login_cookie: &str) -> bool {
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

pub(super) fn is_douyin_login_cookie_name(name: &str) -> bool {
    channels::platform("douyin")
        .map(|platform| platform.is_login_cookie_name(name))
        .unwrap_or(false)
}

pub(super) fn should_inject_douyin_cookie_domain(domain: &str) -> bool {
    channels::platform("douyin")
        .map(|platform| platform.allows_cookie_domain(domain))
        .unwrap_or(false)
}

pub(super) fn plugin_auth_window_label(platform_id: &str, task_id: &str) -> String {
    format!(
        "plugin-auth-{}-{}",
        normalize_platform_id(platform_id).replace('-', "_"),
        task_suffix(task_id)
    )
}

pub(super) fn close_plugin_auth_windows_for_platform(app: &AppHandle, platform_id: &str, keep_label: &str) {
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

pub(super) fn normalize_plugin_login_target(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    channels::normalize_plugin_login_target(platform_id, login_target)
}

pub(super) fn plugin_login_url(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    channels::plugin_login_url(platform_id, login_target)
}

fn kuaishou_login_window_script() -> &'static str {
    r#"
        (() => {
          if (window.__channelNestKuaishouPatch) return;
          window.__channelNestKuaishouPatch = true;
          const fallbackCallback = 'https://cp.kuaishou.com/rest/infra/sts?followUrl=https%3A%2F%2Fcp.kuaishou.com%2Fprofile&setRootDomain=true';
          const readQuery = () => {
            try {
              return new URLSearchParams(location.search || '');
            } catch (_) {
              return new URLSearchParams('');
            }
          };
          const walk = (value, predicate, seen = new Set()) => {
            if (!value || typeof value !== 'object' || seen.has(value)) return null;
            seen.add(value);
            if (predicate(value)) return value;
            if (Array.isArray(value)) {
              for (const item of value) {
                const match = walk(item, predicate, seen);
                if (match) return match;
              }
              return null;
            }
            for (const key of Object.keys(value)) {
              const match = walk(value[key], predicate, seen);
              if (match) return match;
            }
            return null;
          };
          const normalizePayload = (payload) => {
            if (!payload) return null;
            if (typeof payload === 'string') {
              try {
                return JSON.parse(payload);
              } catch (_) {
                return null;
              }
            }
            return payload;
          };
          const extractAuthPayload = (payload) => {
            const data = normalizePayload(payload);
            const querySid = readQuery().get('sid') || 'kuaishou.web.cp.api';
            const match = walk(data, (item) => {
              const sid = item.sid || querySid;
              const dynamicKey = Object.keys(item).find((key) => key.endsWith('.at') && typeof item[key] === 'string' && item[key].length > 8);
              const token = item.authToken || item[`${sid}.at`] || item[`${querySid}.at`] || (dynamicKey ? item[dynamicKey] : '');
              if (typeof token === 'string') return token.length > 8;
              return false;
            });
            if (!match) return null;
            const sid = match.sid || querySid;
            const dynamicKey = Object.keys(match).find((key) => key.endsWith('.at') && typeof match[key] === 'string' && match[key].length > 8);
            const authToken = match.authToken || match[`${sid}.at`] || match[`${querySid}.at`] || (dynamicKey ? match[dynamicKey] : '');
            if (typeof authToken !== 'string' || authToken.length <= 8) return null;
            return {
              authToken,
              sid,
              stsUrl: match.stsUrl,
              followUrl: match.followUrl
            };
          };
          const redirectWithAuthToken = (payload) => {
            if (window.__channelNestKuaishouRedirecting) return false;
            const auth = extractAuthPayload(payload);
            if (!auth) return false;
            const query = readQuery();
            const base = query.get('callback') || auth.stsUrl || fallbackCallback;
            let target;
            try {
              target = new URL(base, location.href);
            } catch (_) {
              target = new URL(fallbackCallback);
            }
            const followUrl = auth.followUrl || target.searchParams.get('followUrl') || 'https://cp.kuaishou.com/profile';
            target.searchParams.set('sid', auth.sid || query.get('sid') || 'kuaishou.web.cp.api');
            target.searchParams.set('authToken', auth.authToken);
            if (!target.searchParams.get('followUrl')) {
              target.searchParams.set('followUrl', followUrl);
            }
            if (!target.searchParams.get('setRootDomain')) {
              target.searchParams.set('setRootDomain', 'true');
            }
            window.__channelNestKuaishouRedirecting = true;
            try {
              if (window.top && window.top !== window) {
                window.top.location.href = target.toString();
              } else {
                window.location.href = target.toString();
              }
            } catch (_) {
              window.location.href = target.toString();
            }
            return true;
          };
          window.addEventListener('message', (event) => {
            const payload = normalizePayload(event.data);
            if (!payload) return;
            const type = String(payload.type || payload.msgType || '');
            if (type.includes('passport-login-iframe-msg-success') || type.includes('success')) {
              redirectWithAuthToken(payload);
            }
          }, true);
          const originalXhrOpen = XMLHttpRequest.prototype.open;
          const originalXhrSend = XMLHttpRequest.prototype.send;
          XMLHttpRequest.prototype.open = function(method, url) {
            this.__channelNestKuaishouUrl = url;
            return originalXhrOpen.apply(this, arguments);
          };
          XMLHttpRequest.prototype.send = function() {
            this.addEventListener('loadend', () => {
              try {
                const text = this.responseText || '';
                if (text && (text.includes('authToken') || text.includes('.at') || text.includes('stsUrl'))) {
                  redirectWithAuthToken(JSON.parse(text));
                }
              } catch (_) {}
            });
            return originalXhrSend.apply(this, arguments);
          };
          if (window.fetch) {
            const originalFetch = window.fetch.bind(window);
            window.fetch = function() {
              return originalFetch.apply(window, arguments).then((response) => {
                try {
                  const clone = response.clone();
                  clone.text().then((text) => {
                    if (text && (text.includes('authToken') || text.includes('.at') || text.includes('stsUrl'))) {
                      try {
                        redirectWithAuthToken(JSON.parse(text));
                      } catch (_) {}
                    }
                  }).catch(() => {});
                } catch (_) {}
                return response;
              });
            };
          }
        })();
    "#
}

pub(super) fn open_plugin_login_window(
    app: &AppHandle,
    platform_id: &str,
    task_id: &str,
    login_target: Option<&str>,
) -> Result<CreatorLoginSession, String> {
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
        return Ok(CreatorLoginSession {
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

    let mut builder = WebviewWindowBuilder::new(app, label.clone(), WebviewUrl::External(url.clone()))
        .title(&title)
        .inner_size(1120.0, 780.0)
        .min_inner_size(960.0, 640.0)
        .data_directory(data_dir)
        .data_store_identifier(task_data_store_identifier(task_id))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
        .center();
    if normalize_platform_id(platform_id) == "kuaishou" {
        let app_for_popup = app.clone();
        let popup_parent_label = label.clone();
        let popup_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("无法创建快手弹窗数据目录: {error}"))?
            .join("plugin-auth")
            .join(normalize_platform_id(platform_id))
            .join(task_id);
        let popup_store_id = task_data_store_identifier(task_id);
        builder = builder
            .initialization_script_for_all_frames(kuaishou_login_window_script())
            .on_new_window(move |popup_url, features| {
                let popup_label = format!(
                    "{popup_parent_label}-popup-{}",
                    stable_label_fragment(popup_url.as_str())
                );
                if let Some(window) = app_for_popup.get_webview_window(&popup_label) {
                    let _ = window.navigate(popup_url);
                    let _ = window.show();
                    let _ = window.set_focus();
                    return tauri::webview::NewWindowResponse::Create { window };
                }
                let window = match WebviewWindowBuilder::new(
                    &app_for_popup,
                    popup_label,
                    WebviewUrl::External(popup_url.clone()),
                )
                .title("快手登录 - 营销大师")
                .inner_size(760.0, 720.0)
                .min_inner_size(520.0, 560.0)
                .data_directory(popup_data_dir.clone())
                .data_store_identifier(popup_store_id.clone())
                .window_features(features)
                .initialization_script_for_all_frames(kuaishou_login_window_script())
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
                .on_document_title_changed(|window, title| {
                    let _ = window.set_title(&title);
                })
                .center()
                .build()
                {
                    Ok(window) => window,
                    Err(error) => {
                        eprintln!("[plugin-auth:kuaishou] failed to open popup: {error}");
                        return tauri::webview::NewWindowResponse::Deny;
                    }
                };
                tauri::webview::NewWindowResponse::Create { window }
            });
    }
    let window = builder
        .build()
        .map_err(|error| format!("打开平台登录窗口失败: {error}"))?;
    if matches!(
        normalize_platform_id(platform_id).as_str(),
        "xiaohongshu" | "wechat-channels" | "bilibili" | "kuaishou"
    ) {
        let _ = window.clear_all_browsing_data();
        let _ = window.navigate(url);
    }

    Ok(CreatorLoginSession {
        url: login_url.to_string(),
        session_id: label,
        expires_at: None,
        instructions: Some(plugin_login_instructions(platform_id, login_target)),
        auth_type: "plugin".to_string(),
    })
}

pub(super) fn plugin_login_instructions(platform_id: &str, login_target: Option<&str>) -> String {
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


pub(super) fn collect_webview_cookies(
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

pub(super) fn cookie_domain_matches_hosts(domain: &str, hosts: &[String]) -> bool {
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
