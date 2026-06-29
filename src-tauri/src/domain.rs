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

#[derive(Debug)]
pub(crate) struct CreatorLoginSession {
    pub(crate) url: String,
    pub(crate) session_id: String,
    pub(crate) managed_browser_session: Option<ManagedBrowserAuthSession>,
    pub(crate) expires_at: Option<String>,
    pub(crate) instructions: Option<String>,
    pub(crate) auth_type: String,
}
