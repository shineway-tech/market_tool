use crate::*;

pub(crate) async fn get_bootstrap(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    user_id: String,
) -> Result<Bootstrap, String> {
    let user_id = normalize_user_id(&user_id)?;
    let (settings, accounts) = {
        let mut store = state.store.lock().map_err(lock_error)?;
        if claim_legacy_accounts_for_user(&mut store, &user_id) {
            persist_store(&app, &store)?;
        }
        (store.settings.clone(), user_accounts(&store, &user_id))
    };
    Ok(Bootstrap {
        platforms: default_platforms(),
        accounts,
        settings,
        callback_base_url: None,
    })
}

pub(crate) async fn list_channel_accounts(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    user_id: String,
) -> Result<Vec<ChannelAccount>, String> {
    let user_id = normalize_user_id(&user_id)?;
    let mut store = state.store.lock().map_err(lock_error)?;
    if claim_legacy_accounts_for_user(&mut store, &user_id) {
        persist_store(&app, &store)?;
    }
    Ok(user_accounts(&store, &user_id))
}

pub(crate) fn save_auth_settings(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: SaveSettingsRequest,
) -> Result<AuthSettings, String> {
    let mut store = state.store.lock().map_err(lock_error)?;
    store.settings = normalize_settings(request.settings);
    persist_store(&app, &store)?;
    Ok(store.settings.clone())
}

pub(crate) async fn start_channel_login(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: StartLoginRequest,
) -> Result<StartLoginResponse, String> {
    let user_id = normalize_user_id(&request.user_id)?;
    let task_id = Uuid::new_v4().to_string();
    let normalized_platform_id = normalize_platform_id(&request.platform_id);
    let settings = {
        let store = state.store.lock().map_err(lock_error)?;
        store.settings.clone()
    };
    let platform_settings = settings
        .platforms
        .iter()
        .find(|item| normalize_platform_id(&item.platform_id) == normalized_platform_id)
        .cloned()
        .ok_or_else(|| "未找到平台授权参数".to_string())?;

    let mode = AuthMode::Creator;
    let callback_base = "creator://channel-auth".to_string();
    let callback_path = "creator-callback";
    let callback_url = format!(
        "{callback_base}/{callback_path}?platform={}&taskId={}",
        encode_query(&request.platform_id),
        encode_query(&task_id)
    );

    if !is_plugin_auth_platform(&platform_settings.platform_id) {
        return Err("当前平台暂不支持创作中心登录".to_string());
    }
    let target = platforms::normalize_plugin_login_target(
        &platform_settings.platform_id,
        request.login_target.as_deref(),
    );
    let session = open_managed_browser_login_session(&app, &platform_settings.platform_id, &task_id, target)?;
    let plugin_login_target = target.map(ToString::to_string);
    let login_session_id = Some(session.session_id.clone());
    let expires_at = session.expires_at.clone();
    let instructions = session.instructions.clone();
    let auth_type = session.auth_type.clone();
    let managed_browser_session = session.managed_browser_session.clone();
    let auth_url = session.url;

    {
        let mut pending = state.pending_auth.lock().map_err(lock_error)?;
        pending.insert(
            task_id.clone(),
                PendingAuth {
                    user_id,
                    platform_id: request.platform_id.clone(),
                    managed_browser_session: managed_browser_session.clone(),
                    plugin_login_target: plugin_login_target.clone(),
                    created_at: Utc::now(),
                },
        );
    }

    Ok(StartLoginResponse {
        task_id,
        url: auth_url,
        callback_url,
        mode,
        auth_type,
        session_id: login_session_id,
        expires_at,
        instructions,
    })
}

pub(crate) async fn get_auth_task_status(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    task_id: String,
    user_id: String,
) -> Result<AuthTaskStatus, String> {
    let user_id = normalize_user_id(&user_id)?;
    let pending_task = state
        .pending_auth
        .lock()
        .map_err(lock_error)?
        .get(&task_id)
        .filter(|task| task.user_id == user_id)
        .cloned();

    if let Some(task) = pending_task.clone() {
        if let Some(managed_session) = task.managed_browser_session.clone() {
            match managed_browser_cookie_snapshot(&managed_session) {
                Ok(Some(snapshot)) => {
                    let profile_result = if normalize_platform_id(&task.platform_id) == "kuaishou" {
                        collect_kuaishou_account_from_managed_browser(&managed_session, snapshot).await
                    } else {
                        collect_plugin_account_info_from_cookie(
                            &task.platform_id,
                            snapshot.cookie_header,
                            snapshot.login_cookie,
                            task.plugin_login_target.as_deref(),
                        )
                        .await
                    };

                    match profile_result {
                        Ok(profile) => {
                            if let Some(existing) =
                                existing_plugin_account_for_profile(&app, &task.user_id, &task.platform_id, &profile)?
                            {
                                let account =
                                    update_plugin_account_profile(&app, &task.user_id, &existing.id, &profile)?;
                                upsert_account_secret(&app, &account.id, &profile.login_cookie)?;
                                let _ = upsert_account_webview_session(&app, &account.id, &managed_session.profile_id);
                                close_managed_browser_auth_session(&managed_session);
                                state
                                    .pending_auth
                                    .lock()
                                    .map_err(lock_error)?
                                    .remove(&task_id);
                                return Ok(AuthTaskStatus {
                                    task_id,
                                    status: "success".to_string(),
                                    account: Some(account),
                                    message: None,
                                });
                            }

                            let account = plugin_info_to_channel_account(&task.platform_id, &profile);
                            let account = upsert_account_for_user(&app, &task.user_id, account)?;
                            upsert_account_secret(&app, &account.id, &profile.login_cookie)?;
                            let _ = upsert_account_webview_session(&app, &account.id, &managed_session.profile_id);
                            close_managed_browser_auth_session(&managed_session);
                            state
                                .pending_auth
                                .lock()
                                .map_err(lock_error)?
                                .remove(&task_id);
                            return Ok(AuthTaskStatus {
                                task_id,
                                status: "success".to_string(),
                                account: Some(account),
                                message: None,
                            });
                        }
                        Err(PluginAuthError::NotLoggedIn(message)) => {
                            return Ok(AuthTaskStatus {
                                task_id,
                                status: "pending".to_string(),
                                account: None,
                                message: Some(message),
                            });
                        }
                        Err(PluginAuthError::Failed(message)) => {
                            close_managed_browser_auth_session(&managed_session);
                            state
                                .pending_auth
                                .lock()
                                .map_err(lock_error)?
                                .remove(&task_id);
                            return Ok(AuthTaskStatus {
                                task_id,
                                status: "failed".to_string(),
                                account: None,
                                message: Some(message),
                            });
                        }
                    }
                }
                Ok(None) => {
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "pending".to_string(),
                        account: None,
                        message: Some("请先在打开的浏览器窗口完成登录。".to_string()),
                    });
                }
                Err(message) => {
                    let browser_closed = message.contains("授权浏览器已关闭")
                        || message.contains("没有找到可控制的浏览器页面");
                    let task_age_seconds = Utc::now()
                        .signed_duration_since(task.created_at)
                        .num_seconds();
                    if browser_closed && task_age_seconds > 8 {
                        close_managed_browser_auth_session(&managed_session);
                        state
                            .pending_auth
                            .lock()
                            .map_err(lock_error)?
                            .remove(&task_id);
                        return Ok(AuthTaskStatus {
                            task_id,
                            status: "failed".to_string(),
                            account: None,
                            message: Some(message),
                        });
                    }
                    return Ok(AuthTaskStatus {
                        task_id,
                        status: "pending".to_string(),
                        account: None,
                        message: Some(message),
                    });
                }
            }
        }

    }

    let store = state.store.lock().map_err(lock_error)?;
    let accounts = user_accounts(&store, &user_id);
    let account = accounts
        .iter()
        .find(|item| {
            item.id == task_id
                || pending_task
                    .as_ref()
                    .map(|task| item.platform_id == task.platform_id && item.created_at >= task.created_at)
                    .unwrap_or(false)
        })
        .cloned();

    let status = if account.is_some() {
        "success"
    } else if pending_task.is_some() {
        "pending"
    } else {
        "unknown"
    };

    Ok(AuthTaskStatus {
        task_id,
        status: status.to_string(),
        account,
        message: None,
    })
}

async fn collect_kuaishou_account_from_managed_browser(
    managed_session: &ManagedBrowserAuthSession,
    snapshot: ManagedBrowserCookieSnapshot,
) -> Result<PluginAccountInfo, PluginAuthError> {
    if !has_kuaishou_creator_login_cookie_header(&snapshot.cookie_header) {
        return Err(PluginAuthError::NotLoggedIn(
            "请先在打开的快手窗口完成登录。".to_string(),
        ));
    }

    if !page_url_is_kuaishou_creator_home(&snapshot.page_url) {
        let url = creator_home_url("kuaishou", "快手创作者中心").map_err(PluginAuthError::Failed)?;
        managed_browser_navigate(managed_session, url.as_str())
            .map_err(|error| PluginAuthError::NotLoggedIn(format!("正在进入快手创作者中心，请稍候。{error}")))?;
        return Err(PluginAuthError::NotLoggedIn(
            "已检测到快手登录态，正在进入快手创作者中心，请稍候。".to_string(),
        ));
    }

    match managed_browser_fetch_kuaishou_home_info(managed_session) {
        Ok(value) => match collect_kuaishou_plugin_account_from_browser_context(
            value,
            snapshot.login_cookie.clone(),
        )
        .await
        {
            Ok(mut profile) => {
                if let Ok(Some(updated_snapshot)) = managed_browser_cookie_snapshot(managed_session) {
                    if !updated_snapshot.login_cookie.trim().is_empty() {
                        profile.login_cookie = updated_snapshot.login_cookie;
                    }
                }
                Ok(profile)
            }
            Err(PluginAuthError::NotLoggedIn(message)) => {
                eprintln!("[managed-auth:kuaishou] creator api not logged in, retrying with stored cookie: {message}");
                collect_plugin_account_info_from_cookie(
                    "kuaishou",
                    snapshot.cookie_header,
                    snapshot.login_cookie,
                    None,
                )
                .await
            }
            Err(error) => Err(error),
        },
        Err(message) => {
            eprintln!("[managed-auth:kuaishou] creator api request failed, retrying with stored cookie: {message}");
            collect_plugin_account_info_from_cookie(
                "kuaishou",
                snapshot.cookie_header,
                snapshot.login_cookie,
                None,
            )
            .await
        }
    }
}

fn page_url_is_kuaishou_creator_home(raw_url: &str) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };
    let Some(host) = url.host_str().map(|host| host.to_ascii_lowercase()) else {
        return false;
    };
    host == "cp.kuaishou.com" && url.path().starts_with("/profile")
}

pub(crate) async fn refresh_channel_account(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(&user_id)?;
    let existing = {
        let store = state.store.lock().map_err(lock_error)?;
        store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .cloned()
    };
    let account = existing
        .as_ref()
        .ok_or_else(|| "账号不存在".to_string())?;
    let (secret_cookie, secret_webview_session_id) = {
        let mut store = state.store.lock().map_err(lock_error)?;
        let migrated = migrate_account_secret_for_account(&mut store, account);
        let value = (
            account_secret_for_account(&store, account).and_then(|secret| secret.login_cookie),
            account_secret_for_account(&store, account).and_then(|secret| secret.webview_session_id),
        );
        if migrated {
            persist_store(&app, &store)?;
        }
        value
    };
    let creator_status = match refresh_account_creator_session(
        &app,
        account,
        secret_cookie.as_deref(),
        secret_webview_session_id.as_deref(),
    )
    .await
    {
        Ok(status) => status,
        Err(error) => {
            let _ = mark_account_expired(&app, &account.id);
            return Err(error);
        }
    };

    let mut store = state.store.lock().map_err(lock_error)?;
    if creator_status.login_cookie.is_some() || creator_status.webview_session_id.is_some() {
        let secret = store.account_secrets.entry(account_id.clone()).or_default();
        if let Some(login_cookie) = creator_status.login_cookie.as_ref() {
            if !login_cookie.trim().is_empty() {
                secret.login_cookie = Some(login_cookie.clone());
            }
        }
        if let Some(webview_session_id) = creator_status.webview_session_id.as_ref() {
            if !webview_session_id.trim().is_empty() {
                secret.webview_session_id = Some(webview_session_id.clone());
            }
        }
    }
    let account = store
        .accounts
        .iter_mut()
        .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
        .ok_or_else(|| "账号不存在".to_string())?;
    if let Some(profile) = creator_status.profile.as_ref() {
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
        if account.uid.trim().is_empty() || normalize_platform_id(&account.platform_id) == "douyin" {
            account.uid = profile.uid.clone();
        }
    }
    account.status = AccountStatus::Active;
    account.last_sync_at = Some(Utc::now());
    account.updated_at = Utc::now();
    let cloned = account.clone();
    persist_store(&app, &store)?;
    Ok(cloned)
}

async fn refresh_account_creator_session(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<CreatorSessionStatus, String> {
    if normalize_platform_id(&account.platform_id) == "kuaishou" {
        if let Some(profile_id) = saved_webview_session_id.map(str::trim).filter(|value| !value.is_empty()) {
            match managed_browser_fetch_kuaishou_home_info_from_profile(app, profile_id) {
                Ok((value, snapshot)) => {
                    let login_cookie = snapshot
                        .as_ref()
                        .map(|item| item.login_cookie.clone())
                        .filter(|value| !value.trim().is_empty())
                        .or_else(|| saved_login_cookie.map(ToString::to_string))
                        .ok_or_else(|| "快手网页登录态已失效，请重新登录后再同步。".to_string())?;
                    let profile = collect_kuaishou_plugin_account_from_browser_context(value, login_cookie)
                        .await
                        .map_err(|error| plugin_error_message(&error))?;
                    return Ok(CreatorSessionStatus {
                        login_cookie: Some(profile.login_cookie.clone()),
                        webview_session_id: Some(profile_id.to_string()),
                        profile: Some(profile),
                    });
                }
                Err(error) => {
                    eprintln!("[creator-session:kuaishou] browser profile api probe failed: {error}");
                }
            }
        }
    }

    check_creator_session(account, saved_login_cookie, saved_webview_session_id).await
}

pub(crate) async fn mark_channel_account_unavailable(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(&user_id)?;
    {
        let store = state.store.lock().map_err(lock_error)?;
        store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .ok_or_else(|| "账号不存在".to_string())?;
    }
    mark_account_expired(&app, &account_id)
}

pub(crate) async fn open_account_homepage(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    let user_id = normalize_user_id(&user_id)?;
    let (mut account, mut saved_login_cookie, mut saved_webview_session_id) = {
        let mut store = state.store.lock().map_err(lock_error)?;
        let account = store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .cloned()
            .ok_or_else(|| "账号不存在".to_string())?;
        let migrated = migrate_account_secret_for_account(&mut store, &account);
        let secret = account_secret_for_account(&store, &account);
        let saved_login_cookie = secret.as_ref().and_then(|secret| secret.login_cookie.clone());
        let saved_webview_session_id = secret.as_ref().and_then(|secret| secret.webview_session_id.clone());
        if migrated {
            persist_store(&app, &store)?;
        }
        Ok::<_, String>((account, saved_login_cookie, saved_webview_session_id))
    }
    ?;

    if creator_home_uses_managed_browser(&account.platform_id) {
        match check_creator_session(
            &account,
            saved_login_cookie.as_deref(),
            saved_webview_session_id.as_deref(),
        )
        .await
        {
            Ok(session) => {
                if let Some(profile) = session.profile.as_ref() {
                    account = update_plugin_account_profile(&app, &user_id, &account.id, profile)?;
                }
                if let Some(login_cookie) = session.login_cookie {
                    saved_login_cookie = Some(login_cookie);
                }
                if let Some(webview_session_id) = session.webview_session_id {
                    saved_webview_session_id = Some(webview_session_id.clone());
                    let _ = upsert_account_webview_session(&app, &account.id, &webview_session_id);
                }
            }
            Err(error) => {
                let _ = mark_account_expired(&app, &account.id);
                return Err(error);
            }
        }
        open_creator_homepage_managed_browser(
            app.clone(),
            account.clone(),
            saved_login_cookie,
            saved_webview_session_id,
        )?;
        Ok(account)
    } else {
        let url = account_homepage_url(&account)?;
        open_external_url(&url)?;
        Ok(account)
    }
}

pub(crate) async fn delete_channel_account(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<(), String> {
    let user_id = normalize_user_id(&user_id)?;
    let account = {
        let store = state.store.lock().map_err(lock_error)?;
        store
            .accounts
            .iter()
            .find(|item| item.id == account_id && account_belongs_to_user(item, &user_id))
            .cloned()
    };
    let mut store = state.store.lock().map_err(lock_error)?;
    let original_len = store.accounts.len();
    store
        .accounts
        .retain(|item| !(item.id == account_id && account_belongs_to_user(item, &user_id)));
    if store.accounts.len() == original_len {
        return Err("账号不存在".to_string());
    }
    for secret_key in account
        .as_ref()
        .map(account_secret_candidates)
        .unwrap_or_default()
    {
        store.account_secrets.remove(&secret_key);
    }
    persist_store(&app, &store)?;
    Ok(())
}
