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

pub(super) fn account_homepage_url(account: &ChannelAccount) -> Result<String, String> {
    channel_platform(&account.platform_id)?.homepage_url(&account.uid, &account.nickname)
}

mod cookies;
mod lifecycle;
mod plugin_auth;

pub(super) use cookies::*;
pub(super) use lifecycle::*;
pub(super) use plugin_auth::*;
