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

fn destroy_window_handle(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.hide();
    let _ = window.destroy();
}

fn force_destroy_on_close(window: &WebviewWindow<tauri::Wry>) {
    let window_for_close = window.clone();
    let closing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let closing_for_event = closing.clone();

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            if closing_for_event.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }
            api.prevent_close();
            destroy_window_handle(&window_for_close);
        }
    });
}

pub(super) fn destroy_webview_window(window: &WebviewWindow<tauri::Wry>) {
    destroy_window_handle(window);
}

pub(super) fn close_creator_home_windows(app: &AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("creator-home-") {
            destroy_webview_window(&window);
        }
    }
}

pub(super) fn prepare_external_webview_window(window: &WebviewWindow<tauri::Wry>) {
    ensure_close_controls(window);
    force_destroy_on_close(window);
}

mod cookies;
mod creator_home;
mod plugin_auth;

pub(super) use cookies::*;
pub(super) use creator_home::*;
pub(super) use plugin_auth::*;
