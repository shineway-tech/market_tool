use super::*;

pub(crate) fn normalize_plugin_login_target(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    channels::normalize_plugin_login_target(platform_id, login_target)
}

pub(crate) fn plugin_login_url(platform_id: &str, login_target: Option<&str>) -> Option<&'static str> {
    channels::plugin_login_url(platform_id, login_target)
}
