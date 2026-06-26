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
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let window = window_for_close.clone();
            tauri::async_runtime::spawn(async move {
                let _ = window.destroy();
            });
        }
    });
}

mod cookies;
mod creator_home;
mod plugin_auth;

pub(super) use cookies::*;
pub(super) use creator_home::*;
pub(super) use plugin_auth::*;
