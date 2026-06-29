use chrono::{DateTime, Utc};
use std::{collections::HashMap, sync::Mutex};

use crate::{
    domain::StoreFile,
    browser::ManagedBrowserAuthSession,
};

#[derive(Debug, Clone)]
pub(crate) struct PendingAuth {
    pub(crate) user_id: String,
    pub(crate) platform_id: String,
    pub(crate) managed_browser_session: Option<ManagedBrowserAuthSession>,
    pub(crate) plugin_login_target: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
}

pub(crate) struct RuntimeState {
    pub(crate) store: Mutex<StoreFile>,
    pub(crate) pending_auth: Mutex<HashMap<String, PendingAuth>>,
}
