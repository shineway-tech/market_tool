use super::*;

#[derive(Clone, Copy)]
enum CreatorHomeSessionStore {
    AuthSessions,
    PluginAuth,
    RelayAuth,
}

#[derive(Clone, Copy)]
struct CreatorHomeSpec {
    platform_id: &'static str,
    label_segment: &'static str,
    title_fallback: &'static str,
    title_suffix: &'static str,
    data_key: &'static str,
    session_store: CreatorHomeSessionStore,
}

pub(crate) fn open_creator_homepage_webview(
    app: AppHandle,
    account: ChannelAccount,
    saved_login_cookie: Option<String>,
    saved_webview_session_id: Option<String>,
) -> Result<(), String> {
    let platform_id = normalize_platform_id(&account.platform_id);
    if creator_home_spec(&platform_id).is_none() {
        return Err("当前平台暂不支持内置主页窗口".to_string());
    }

    let app_for_open = app.clone();
    app.run_on_main_thread(move || {
        if let Err(error) = open_creator_home_window(
            &app_for_open,
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        ) {
            eprintln!("[creator-home:{platform_id}] open failed: {error}");
        }
    })
    .map_err(|error| format!("打开主页窗口失败: {error}"))
}

pub(crate) fn creator_home_uses_webview(platform_id: &str) -> bool {
    creator_home_spec(platform_id).is_some()
}

fn creator_home_spec(platform_id: &str) -> Option<CreatorHomeSpec> {
    match normalize_platform_id(platform_id).as_str() {
        "douyin" => Some(CreatorHomeSpec {
            platform_id: "douyin",
            label_segment: "douyin",
            title_fallback: "抖音账号",
            title_suffix: "抖音创作者中心",
            data_key: "douyin",
            session_store: CreatorHomeSessionStore::AuthSessions,
        }),
        "xiaohongshu" => Some(CreatorHomeSpec {
            platform_id: "xiaohongshu",
            label_segment: "xhs",
            title_fallback: "小红书账号",
            title_suffix: "小红书创作中心",
            data_key: "xiaohongshu",
            session_store: CreatorHomeSessionStore::PluginAuth,
        }),
        "wechat-channels" => Some(CreatorHomeSpec {
            platform_id: "wechat-channels",
            label_segment: "wx-sph",
            title_fallback: "视频号账号",
            title_suffix: "视频号后台",
            data_key: "wechat-channels",
            session_store: CreatorHomeSessionStore::PluginAuth,
        }),
        "bilibili" => Some(CreatorHomeSpec {
            platform_id: "bilibili",
            label_segment: "bilibili",
            title_fallback: "B 站账号",
            title_suffix: "B 站创作中心",
            data_key: "bilibili",
            session_store: CreatorHomeSessionStore::PluginAuth,
        }),
        "kuaishou" => Some(CreatorHomeSpec {
            platform_id: "kuaishou",
            label_segment: "kuaishou",
            title_fallback: "快手账号",
            title_suffix: "快手创作者中心",
            data_key: "kuaishou",
            session_store: CreatorHomeSessionStore::RelayAuth,
        }),
        _ => None,
    }
}

fn open_creator_home_window(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<(), String> {
    let spec = creator_home_spec(&account.platform_id).ok_or_else(|| "当前平台暂不支持内置主页窗口".to_string())?;
    let url = creator_home_url(spec.platform_id, spec.title_suffix)?;
    let title_name = if account.nickname.trim().is_empty() {
        spec.title_fallback
    } else {
        account.nickname.trim()
    };
    let title = format!("{title_name} - {}", spec.title_suffix);
    let label = creator_home_window_label(spec, account, saved_webview_session_id);
    let (data_dir, data_store_identifier) =
        creator_home_data_store(app, spec, account, saved_webview_session_id)?;
    let account_id = account.id.clone();
    let app_for_load = app.clone();
    let platform_id = spec.platform_id;
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
                handle_creator_home_page_load(&app_for_load, &window, &account_id, platform_id, payload.url());
            }
        })
        .center()
        .build()
        .map_err(|error| format!("打开{}失败: {error}", spec.title_suffix))?;

    prepare_creator_home_window(&window);

    if let Some(login_cookie) = saved_login_cookie {
        if let Err(error) = inject_creator_home_login_cookie(&window, spec.platform_id, login_cookie) {
            eprintln!("[creator-home:{}] cookie injection failed: {error}", spec.platform_id);
        }
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.navigate(url);
    }

    Ok(())
}

fn creator_home_window_label(
    spec: CreatorHomeSpec,
    account: &ChannelAccount,
    saved_webview_session_id: Option<&str>,
) -> String {
    let account_key = stable_label_fragment(&account.id);
    let session_key = saved_webview_session_id
        .filter(|value| !value.trim().is_empty())
        .map(stable_label_fragment)
        .unwrap_or_else(|| "local".to_string());
    let open_key = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("creator-home-{}-{account_key}-{session_key}-{open_key}", spec.label_segment)
}

fn creator_home_data_store(
    app: &AppHandle,
    spec: CreatorHomeSpec,
    account: &ChannelAccount,
    saved_webview_session_id: Option<&str>,
) -> Result<(std::path::PathBuf, [u8; 16]), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法创建{}数据目录: {error}", spec.title_suffix))?;

    if let Some(session_id) = saved_webview_session_id {
        let root = match spec.session_store {
            CreatorHomeSessionStore::AuthSessions => "auth-sessions",
            CreatorHomeSessionStore::PluginAuth => "plugin-auth",
            CreatorHomeSessionStore::RelayAuth => "relay-auth",
        };
        let session_segment = if matches!(spec.session_store, CreatorHomeSessionStore::AuthSessions) {
            stable_label_fragment(session_id)
        } else {
            session_id.to_string()
        };
        return Ok((
            app_data_dir.join(root).join(spec.data_key).join(session_segment),
            task_data_store_identifier(session_id),
        ));
    }

    Ok((
        app_data_dir
            .join("creator-home")
            .join(spec.data_key)
            .join(stable_label_fragment(&account.id)),
        stable_data_store_identifier(&format!("creator-home:{}:{}", spec.label_segment, account.id)),
    ))
}

fn inject_creator_home_login_cookie(
    window: &WebviewWindow<tauri::Wry>,
    platform_id: &str,
    login_cookie: &str,
) -> Result<(), String> {
    match normalize_platform_id(platform_id).as_str() {
        "douyin" => inject_douyin_login_cookie(window, login_cookie),
        "xiaohongshu" => inject_xhs_login_cookie(window, login_cookie),
        "wechat-channels" => inject_wx_channels_login_cookie(window, login_cookie),
        "bilibili" => inject_bilibili_login_cookie(window, login_cookie),
        "kuaishou" => inject_kuaishou_login_cookie(window, login_cookie),
        _ => Ok(()),
    }
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
