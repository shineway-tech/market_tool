use super::*;
use std::path::PathBuf;

pub(super) fn upsert_account_secret(app: &AppHandle, account_id: &str, login_cookie: &str) -> Result<(), String> {
    if login_cookie.trim().is_empty() {
        return Ok(());
    }
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let secret = store.account_secrets.entry(account_id.to_string()).or_default();
    secret.login_cookie = Some(login_cookie.to_string());
    persist_store(app, &store)
}

pub(super) fn upsert_account_webview_session(
    app: &AppHandle,
    account_id: &str,
    webview_session_id: &str,
) -> Result<(), String> {
    if webview_session_id.trim().is_empty() {
        return Ok(());
    }
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let secret = store.account_secrets.entry(account_id.to_string()).or_default();
    secret.webview_session_id = Some(webview_session_id.to_string());
    persist_store(app, &store)
}

pub(super) fn emit_account_updated(app: &AppHandle, account: &ChannelAccount) {
    let _ = app.emit(CHANNEL_ACCOUNT_UPDATED_EVENT, account);
}

pub(super) fn mark_account_expired(app: &AppHandle, account_id: &str) -> Result<ChannelAccount, String> {
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let now = Utc::now();
    let account = store
        .accounts
        .iter_mut()
        .find(|item| item.id == account_id)
        .ok_or_else(|| "账号不存在".to_string())?;

    account.status = AccountStatus::Expired;
    account.last_sync_at = Some(now);
    account.updated_at = now;
    let cloned = account.clone();
    persist_store(app, &store)?;
    emit_account_updated(app, &cloned);
    Ok(cloned)
}

pub(super) fn update_plugin_account_profile(
    app: &AppHandle,
    user_id: &str,
    account_id: &str,
    profile: &PluginAccountInfo,
) -> Result<ChannelAccount, String> {
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let now = Utc::now();

    {
        let secret = store.account_secrets.entry(account_id.to_string()).or_default();
        if !profile.login_cookie.trim().is_empty() {
            secret.login_cookie = Some(profile.login_cookie.clone());
        }
    }

    let account = store
        .accounts
        .iter_mut()
        .find(|item| item.id == account_id && account_belongs_to_user(item, user_id))
        .ok_or_else(|| "账号不存在".to_string())?;
    if !profile.nickname.trim().is_empty() {
        account.nickname = profile.nickname.clone();
    }
    if !profile.avatar.trim().is_empty() {
        account.avatar = profile.avatar.clone();
    }
    if let Some(fans_count) = profile.fans_count {
        account.followers = Some(fans_count);
    }
    if let Some(like_count) = profile.like_count {
        account.likes = Some(like_count);
    }
    if account.uid.trim().is_empty() {
        account.uid = profile.uid.clone();
    }
    account.status = AccountStatus::Active;
    account.last_sync_at = Some(now);
    account.updated_at = now;
    let cloned = account.clone();
    persist_store(app, &store)?;
    emit_account_updated(app, &cloned);
    Ok(cloned)
}


pub(super) fn upsert_account_for_user(
    app: &AppHandle,
    user_id: &str,
    account: ChannelAccount,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(user_id)?;
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let mut source_secret_keys = account_secret_candidates(&account);
    let mut account = scoped_account_for_user(&user_id, account);
    for key in account_secret_candidates(&account) {
        push_unique(&mut source_secret_keys, key);
    }
    if let Some(existing) = store
        .accounts
        .iter_mut()
        .find(|item| {
            account_belongs_to_user(item, &user_id)
                && item.platform_id == account.platform_id
                && (item.uid == account.uid
                    || match (
                        item.relay_account_ref.as_deref(),
                        account.relay_account_ref.as_deref(),
                    ) {
                        (Some(left), Some(right)) => left == right,
                        _ => false,
                    })
        })
    {
        account.id = existing.id.clone();
        account.created_at = existing.created_at;
        *existing = account.clone();
    } else {
        store.accounts.push(account.clone());
    }
    migrate_account_secret_from_keys(&mut store, &account.id, &source_secret_keys);
    runtime
        .pending_auth
        .lock()
        .map_err(lock_error)?
        .retain(|task_id, task| {
            task.user_id != user_id
                || (task_id != &account.id && task.platform_id != account.platform_id)
        });
    persist_store(app, &store)?;
    Ok(account)
}


pub(super) fn load_store(app: &AppHandle) -> Result<StoreFile, Box<dyn std::error::Error>> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(StoreFile {
            accounts: Vec::new(),
            settings: default_auth_settings(),
            account_secrets: HashMap::new(),
        });
    }
    let text = fs::read_to_string(path)?;
    let mut store: StoreFile = serde_json::from_str(&text)?;
    store.settings = normalize_settings(store.settings);
    Ok(store)
}

pub(super) fn persist_store(app: &AppHandle, store: &StoreFile) -> Result<(), String> {
    let path = store_path(app).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(store).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

pub(super) fn store_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    Ok(dir.join("channel-auth-store.json"))
}


pub(super) fn normalize_user_id(value: &str) -> Result<String, String> {
    let user_id = value.trim();
    if user_id.is_empty() {
        return Err("当前登录状态无效，请重新登录".to_string());
    }
    Ok(user_id.to_string())
}

pub(super) fn account_belongs_to_user(account: &ChannelAccount, user_id: &str) -> bool {
    account.user_id.as_deref() == Some(user_id)
}

pub(super) fn user_accounts(store: &StoreFile, user_id: &str) -> Vec<ChannelAccount> {
    store
        .accounts
        .iter()
        .filter(|account| account_belongs_to_user(account, user_id))
        .cloned()
        .collect()
}

pub(super) fn claim_legacy_accounts_for_user(store: &mut StoreFile, user_id: &str) -> bool {
    let user_id = user_id.trim();
    if user_id.is_empty() {
        return false;
    }
    let mut changed = false;
    for account in &mut store.accounts {
        if account.user_id.is_none() {
            account.user_id = Some(user_id.to_string());
            changed = true;
        }
    }
    changed
}

pub(super) fn account_secret_for_account(store: &StoreFile, account: &ChannelAccount) -> Option<AccountSecret> {
    account_secret_candidates(account)
        .into_iter()
        .find_map(|key| store.account_secrets.get(&key).cloned())
}

pub(super) fn migrate_account_secret_for_account(store: &mut StoreFile, account: &ChannelAccount) -> bool {
    let keys = account_secret_candidates(account);
    migrate_account_secret_from_keys(store, &account.id, &keys)
}

pub(super) fn migrate_account_secret_from_keys(
    store: &mut StoreFile,
    target_id: &str,
    source_keys: &[String],
) -> bool {
    let mut changed = false;
    for key in source_keys {
        if key == target_id {
            continue;
        }
        let Some(source) = store.account_secrets.get(key).cloned() else {
            continue;
        };
        let target = store.account_secrets.entry(target_id.to_string()).or_default();
        if target.login_cookie.is_none() && source.login_cookie.is_some() {
            target.login_cookie = source.login_cookie.clone();
            changed = true;
        }
        if target.webview_session_id.is_none() && source.webview_session_id.is_some() {
            target.webview_session_id = source.webview_session_id.clone();
            changed = true;
        }
    }
    changed
}

pub(super) fn account_secret_candidates(account: &ChannelAccount) -> Vec<String> {
    let mut values = Vec::new();
    push_unique(&mut values, account.id.clone());
    if let Some(user_id) = account.user_id.as_deref() {
        if let Some(raw_id) = unscoped_account_id(user_id, &account.id) {
            push_unique(&mut values, raw_id);
        }
    }
    if let Some(relay_account_ref) = account.relay_account_ref.as_ref() {
        push_unique(&mut values, relay_account_ref.clone());
        if let Some(user_id) = account.user_id.as_deref() {
            push_unique(&mut values, scoped_account_id(user_id, relay_account_ref));
        }
    }
    values
}

pub(super) fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.trim().is_empty() && !values.iter().any(|item| item == &value) {
        values.push(value);
    }
}

pub(super) fn scoped_account_for_user(user_id: &str, mut account: ChannelAccount) -> ChannelAccount {
    account.user_id = Some(user_id.to_string());
    account.id = scoped_account_id(user_id, &account.id);
    account
}

pub(super) fn scoped_account_id(user_id: &str, account_id: &str) -> String {
    let prefix = format!("u{}_", stable_label_fragment(user_id));
    if account_id.starts_with(&prefix) {
        account_id.to_string()
    } else {
        format!("{prefix}{account_id}")
    }
}

pub(super) fn unscoped_account_id(user_id: &str, account_id: &str) -> Option<String> {
    let prefix = format!("u{}_", stable_label_fragment(user_id));
    account_id
        .strip_prefix(&prefix)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}
