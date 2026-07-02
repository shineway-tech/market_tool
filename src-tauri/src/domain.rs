use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::browser::ManagedBrowserAuthSession;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlatformInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) color: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlatformAuthSettings {
    pub(crate) platform_id: String,
    pub(crate) mode: AuthMode,
    pub(crate) auth_url: String,
    pub(crate) token_url: String,
    pub(crate) profile_url: String,
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthSettings {
    pub(crate) platforms: Vec<PlatformAuthSettings>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AuthMode {
    Creator,
}

impl<'de> Deserialize<'de> for AuthMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let _ = String::deserialize(deserializer)?;
        Ok(Self::Creator)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelAccount {
    pub(crate) id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) user_id: Option<String>,
    pub(crate) platform_id: String,
    pub(crate) uid: String,
    pub(crate) nickname: String,
    pub(crate) avatar: String,
    pub(crate) followers: Option<u64>,
    #[serde(default)]
    pub(crate) following: Option<u64>,
    pub(crate) likes: Option<u64>,
    pub(crate) status: AccountStatus,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) last_sync_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AccountStatus {
    Active,
    Expired,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoreFile {
    pub(crate) accounts: Vec<ChannelAccount>,
    pub(crate) settings: AuthSettings,
    #[serde(default)]
    pub(crate) account_secrets: HashMap<String, AccountSecret>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountSecret {
    pub(crate) login_cookie: Option<String>,
    pub(crate) webview_session_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Bootstrap {
    pub(crate) platforms: Vec<PlatformInfo>,
    pub(crate) accounts: Vec<ChannelAccount>,
    pub(crate) settings: AuthSettings,
    pub(crate) callback_base_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartLoginResponse {
    pub(crate) task_id: String,
    pub(crate) url: String,
    pub(crate) callback_url: String,
    pub(crate) mode: AuthMode,
    pub(crate) auth_type: String,
    pub(crate) session_id: Option<String>,
    pub(crate) expires_at: Option<String>,
    pub(crate) instructions: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartLoginRequest {
    pub(crate) user_id: String,
    pub(crate) platform_id: String,
    pub(crate) login_target: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveSettingsRequest {
    pub(crate) settings: AuthSettings,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthTaskStatus {
    pub(crate) task_id: String,
    pub(crate) status: String,
    pub(crate) account: Option<ChannelAccount>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelAccountContentRequest {
    pub(crate) account_id: String,
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelWorksPageRequest {
    pub(crate) account_id: String,
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) page_key: Option<String>,
    #[serde(default)]
    pub(crate) work_type: Option<String>,
    #[serde(default)]
    pub(crate) force: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelAccountProfileSnapshot {
    pub(crate) account_id: String,
    pub(crate) platform_id: String,
    pub(crate) followers: Option<u64>,
    pub(crate) following: Option<u64>,
    pub(crate) likes: Option<u64>,
    pub(crate) last_sync_at: Option<DateTime<Utc>>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(crate) sync_status: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelOverviewMetric {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) value: Option<String>,
    pub(crate) compare_label: Option<String>,
    pub(crate) trend: Option<String>,
    pub(crate) tone: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelAccountOverview {
    pub(crate) account_id: String,
    pub(crate) platform_id: String,
    pub(crate) period_days: u16,
    pub(crate) metrics: Vec<ChannelOverviewMetric>,
    pub(crate) summary: Option<String>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(crate) sync_status: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelWorkMetric {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) value: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelContentWork {
    pub(crate) id: String,
    pub(crate) platform_id: String,
    pub(crate) account_id: String,
    pub(crate) title: String,
    pub(crate) cover_url: Option<String>,
    pub(crate) link: Option<String>,
    pub(crate) published_at: Option<DateTime<Utc>>,
    pub(crate) status: String,
    pub(crate) views: Option<u64>,
    pub(crate) impressions: Option<u64>,
    pub(crate) likes: Option<u64>,
    pub(crate) collects: Option<u64>,
    pub(crate) comments: Option<u64>,
    pub(crate) shares: Option<u64>,
    pub(crate) cover_click_rate: Option<String>,
    pub(crate) avg_view_time: Option<String>,
    pub(crate) gained_followers: Option<i64>,
    pub(crate) data_updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(crate) metrics: Vec<ChannelWorkMetric>,
    #[serde(default)]
    pub(crate) badges: Vec<String>,
    #[serde(default)]
    pub(crate) work_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelWorksPage {
    pub(crate) account_id: String,
    pub(crate) platform_id: String,
    pub(crate) page_key: String,
    #[serde(default)]
    pub(crate) work_type: Option<String>,
    pub(crate) next_page_key: Option<String>,
    pub(crate) has_more: bool,
    pub(crate) works: Vec<ChannelContentWork>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(crate) sync_status: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelAccountContent {
    pub(crate) account_id: String,
    pub(crate) platform_id: String,
    pub(crate) profile: Option<ChannelAccountProfileSnapshot>,
    #[serde(default)]
    pub(crate) overview_yesterday: Option<ChannelAccountOverview>,
    pub(crate) overview_seven: Option<ChannelAccountOverview>,
    pub(crate) overview_thirty: Option<ChannelAccountOverview>,
    #[serde(default)]
    pub(crate) overview_ninety: Option<ChannelAccountOverview>,
    #[serde(default)]
    pub(crate) overview_history: Option<ChannelAccountOverview>,
    #[serde(default)]
    pub(crate) overview_total: Option<ChannelAccountOverview>,
    pub(crate) latest_work: Option<ChannelContentWork>,
    #[serde(default)]
    pub(crate) latest_work_seven: Option<ChannelContentWork>,
    #[serde(default)]
    pub(crate) latest_work_thirty: Option<ChannelContentWork>,
    #[serde(default)]
    pub(crate) sync_status: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug)]
pub(crate) struct CreatorLoginSession {
    pub(crate) url: String,
    pub(crate) session_id: String,
    pub(crate) managed_browser_session: Option<ManagedBrowserAuthSession>,
    pub(crate) expires_at: Option<String>,
    pub(crate) instructions: Option<String>,
    pub(crate) auth_type: String,
}
