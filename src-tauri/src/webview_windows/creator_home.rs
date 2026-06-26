use super::*;

pub(crate) fn open_douyin_creator_webview(
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

pub(crate) fn navigate_webview_after_delay(window: WebviewWindow<tauri::Wry>, url: Url) {
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

pub(crate) fn open_xhs_creator_webview(
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

pub(crate) fn open_wx_channels_webview(
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

pub(crate) fn open_bilibili_creator_webview(
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

pub(crate) fn open_kuaishou_creator_webview(
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
