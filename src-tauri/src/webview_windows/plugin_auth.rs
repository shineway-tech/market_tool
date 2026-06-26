use super::*;

pub(crate) fn plugin_auth_window_label(platform_id: &str, task_id: &str) -> String {
    format!(
        "plugin-auth-{}-{}",
        normalize_platform_id(platform_id).replace('-', "_"),
        task_suffix(task_id)
    )
}

pub(crate) fn close_plugin_auth_windows_for_platform(app: &AppHandle, platform_id: &str, keep_label: &str) {
    let legacy_label = format!(
        "plugin-auth-{}",
        normalize_platform_id(platform_id).replace('-', "_")
    );
    if legacy_label != keep_label {
        if let Some(window) = app.get_webview_window(&legacy_label) {
            destroy_webview_window(&window);
        }
    }
    let prefix = format!(
        "plugin-auth-{}-",
        normalize_platform_id(platform_id).replace('-', "_")
    );
    for window in app.webview_windows().into_values() {
        let label = window.label();
        if label.starts_with(&prefix) && label != keep_label {
            destroy_webview_window(&window);
        }
    }
}

pub(crate) fn normalize_plugin_login_target(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    channels::normalize_plugin_login_target(platform_id, login_target)
}

pub(crate) fn plugin_login_url(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
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

pub(crate) fn open_plugin_login_window(
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
        prepare_external_webview_window(&window);
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
        .decorations(true)
        .closable(true)
        .resizable(true)
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
                    prepare_external_webview_window(&window);
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
                .decorations(true)
                .closable(true)
                .resizable(true)
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
                prepare_external_webview_window(&window);
                tauri::webview::NewWindowResponse::Create { window }
            });
    }
    let window = builder
        .build()
        .map_err(|error| format!("打开平台登录窗口失败: {error}"))?;
    prepare_external_webview_window(&window);
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

pub(crate) fn plugin_login_instructions(platform_id: &str, login_target: Option<&str>) -> String {
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
