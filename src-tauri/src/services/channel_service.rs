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

    match managed_browser_fetch_kuaishou_home_info_with_retry(managed_session) {
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
                .and_then(reject_kuaishou_cookie_fallback_profile)
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
            .and_then(reject_kuaishou_cookie_fallback_profile)
        }
    }
}

fn reject_kuaishou_cookie_fallback_profile(profile: PluginAccountInfo) -> Result<PluginAccountInfo, PluginAuthError> {
    if is_kuaishou_cookie_fallback_nickname(&profile.nickname) {
        return Err(PluginAuthError::NotLoggedIn(
            "已检测到快手登录态，但还没有读取到真实昵称，请稍后再试或重新登录。".to_string(),
        ));
    }
    Ok(profile)
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
        if !profile.nickname.trim().is_empty() && should_update_account_nickname(account, &profile.nickname) {
            account.nickname = profile.nickname.clone();
        }
        if !profile.avatar.trim().is_empty() {
            account.avatar = profile.avatar.clone();
        }
        if let Some(fans_count) = profile.fans_count {
            account.followers = Some(fans_count);
        }
        if let Some(following_count) = profile.following_count {
            account.following = Some(following_count);
        }
        if let Some(like_count) = profile.like_count {
            account.likes = Some(like_count);
        }
        if account.uid.trim().is_empty()
            || matches!(
                normalize_platform_id(&account.platform_id).as_str(),
                "douyin" | "xiaohongshu"
            )
        {
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

pub(crate) async fn sync_channel_account_content(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: ChannelAccountContentRequest,
) -> Result<ChannelAccountContent, String> {
    let user_id = normalize_user_id(&request.user_id)?;
    let (account, saved_login_cookie, saved_webview_session_id) =
        account_with_secrets(&app, &state, &request.account_id, &user_id)?;
    let cached = read_channel_account_content_cache(&app, &account.id, &account.platform_id)?;
    if !request.force && account_content_cache_is_fresh(&cached) {
        return Ok(cached);
    }
    if !platform_supports_account_content(&account.platform_id) {
        return Ok(cached_with_error(cached, "当前平台的数据看板暂未接入。"));
    }

    let session = match creator_session_for_data_sync(
        &app,
        &account,
        saved_login_cookie.as_deref(),
        saved_webview_session_id.as_deref(),
    )
    .await
    {
        Ok(session) => session,
        Err(error) => {
            if is_login_expired_message(&error) {
                let _ = mark_account_expired(&app, &account.id);
            }
            return Ok(cached_with_error(cached, &error));
        }
    };
    let login_cookie = session
        .login_cookie
        .or(saved_login_cookie)
        .ok_or_else(|| format!("{}登录已失效，请重新登录后再同步。", platform_name(&account.platform_id)))?;
    let cookie_header = plugin_cookie_header(&login_cookie);
    if cookie_header.trim().is_empty() {
        return Ok(cached_with_error(
            cached,
            &format!("{}登录已失效，请重新登录后再同步。", platform_name(&account.platform_id)),
        ));
    }

    let content_result = if normalize_platform_id(&account.platform_id) == "kuaishou" {
        fetch_kuaishou_account_content_with_profile(
            &cookie_header,
            login_cookie,
            &account.id,
            session.profile.clone(),
        )
        .await
    } else {
        fetch_platform_account_content(&account.platform_id, &cookie_header, login_cookie, &account.id).await
    };

    match content_result {
        Ok(content) => {
            write_channel_account_content_cache(&app, &content)?;
            if let Some(profile) = content.profile.as_ref() {
                update_account_from_content_profile(&app, &user_id, &account.id, profile)?;
            }
            Ok(content)
        }
        Err(error) => {
            if is_login_expired_message(&error) {
                let _ = mark_account_expired(&app, &account.id);
            }
            Ok(cached_with_error(cached, &error))
        }
    }
}

pub(crate) async fn load_channel_account_works_page(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: ChannelWorksPageRequest,
) -> Result<ChannelWorksPage, String> {
    let user_id = normalize_user_id(&request.user_id)?;
    let (account, saved_login_cookie, saved_webview_session_id) =
        account_with_secrets(&app, &state, &request.account_id, &user_id)?;
    let work_type = normalize_works_work_type(&account.platform_id, request.work_type.as_deref());
    let requested_page_key = request.page_key.as_deref().unwrap_or("").trim().to_string();
    let page_key = raw_works_page_key(&requested_page_key, work_type.as_deref());
    let cache_page_key = works_cache_page_key(&page_key, work_type.as_deref());
    let cached = read_channel_works_page_cache(&app, &account.id, &account.platform_id, &cache_page_key)?;
    if !request.force && works_page_cache_is_fresh(&cached) {
        return Ok(cached);
    }
    if !platform_supports_works_pages(&account.platform_id) {
        return Ok(works_page_with_error(cached, "当前平台的作品列表暂未接入。"));
    }

    let session = match creator_session_for_data_sync(
        &app,
        &account,
        saved_login_cookie.as_deref(),
        saved_webview_session_id.as_deref(),
    )
    .await
    {
        Ok(session) => session,
        Err(error) => {
            if is_login_expired_message(&error) {
                let _ = mark_account_expired(&app, &account.id);
            }
            return Ok(works_page_with_error(cached, &error));
        }
    };
    let login_cookie = session
        .login_cookie
        .or(saved_login_cookie)
        .ok_or_else(|| format!("{}登录已失效，请重新登录后再同步。", platform_name(&account.platform_id)))?;
    let cookie_header = plugin_cookie_header(&login_cookie);
    if cookie_header.trim().is_empty() {
        return Ok(works_page_with_error(
            cached,
            &format!("{}登录已失效，请重新登录后再同步。", platform_name(&account.platform_id)),
        ));
    }

    let works_result = if normalize_platform_id(&account.platform_id) == "kuaishou" {
        fetch_kuaishou_works_page_via_browser(&login_cookie, &account.id, &page_key)
    } else {
        fetch_platform_works_page(
            &account.platform_id,
            &cookie_header,
            &login_cookie,
            &account.id,
            &page_key,
            work_type.as_deref(),
        )
        .await
    };

    match works_result {
        Ok(mut page) => {
            apply_works_cache_keys(&mut page, work_type.as_deref());
            write_channel_works_page_cache(&app, &page)?;
            Ok(page)
        }
        Err(error) => {
            if is_login_expired_message(&error) {
                let _ = mark_account_expired(&app, &account.id);
            }
            Ok(works_page_with_error(cached, &error))
        }
    }
}

fn platform_supports_account_content(platform_id: &str) -> bool {
    matches!(
        normalize_platform_id(platform_id).as_str(),
        "xiaohongshu" | "wechat-channels" | "douyin" | "bilibili" | "kuaishou"
    )
}

fn platform_supports_works_pages(platform_id: &str) -> bool {
    matches!(
        normalize_platform_id(platform_id).as_str(),
        "xiaohongshu" | "wechat-channels" | "douyin" | "bilibili" | "kuaishou"
    )
}

async fn creator_session_for_data_sync(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<CreatorSessionStatus, String> {
    if normalize_platform_id(&account.platform_id) == "kuaishou" {
        let _ = app;
        kuaishou_saved_creator_session(account, saved_login_cookie, saved_webview_session_id)
    } else {
        check_creator_session(account, saved_login_cookie, saved_webview_session_id).await
    }
}

fn kuaishou_saved_creator_session(
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<CreatorSessionStatus, String> {
    let login_cookie = saved_login_cookie
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "快手网页登录态已失效，请重新登录后再同步。".to_string())?
        .to_string();
    Ok(CreatorSessionStatus {
        login_cookie: Some(login_cookie.clone()),
        webview_session_id: saved_webview_session_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        profile: Some(kuaishou_local_account_profile(account, login_cookie)),
    })
}

fn fetch_kuaishou_works_page_via_browser(
    login_cookie: &str,
    account_id: &str,
    page_key: &str,
) -> Result<ChannelWorksPage, String> {
    let value = managed_browser_fetch_kuaishou_api_with_cookie_headless(
        login_cookie,
        kuaishou_article_manage_video_url(),
        kuaishou_article_manage_video_list_api_url(),
        kuaishou_management_works_body(page_key),
    )?;
    parse_kuaishou_management_works_page(value, account_id, page_key)
}

fn normalize_works_work_type(platform_id: &str, value: Option<&str>) -> Option<String> {
    if !matches!(
        normalize_platform_id(platform_id).as_str(),
        "wechat-channels" | "bilibili" | "kuaishou"
    ) {
        return None;
    }
    let normalized = match normalize_platform_id(platform_id).as_str() {
        "kuaishou" => "video",
        _ => match value.unwrap_or_default().trim() {
            "article" | "photo" | "image" | "new-life" | "newlife" => "article",
            _ => "video",
        },
    };
    Some(normalized.to_string())
}

fn raw_works_page_key(page_key: &str, work_type: Option<&str>) -> String {
    let page_key = page_key.trim();
    let Some(work_type) = work_type else {
        return page_key.to_string();
    };
    page_key
        .strip_prefix(&format!("{work_type}:"))
        .unwrap_or(page_key)
        .to_string()
}

fn works_cache_page_key(page_key: &str, work_type: Option<&str>) -> String {
    match work_type {
        Some(work_type) => format!("{work_type}:{}", page_key.trim()),
        None => page_key.trim().to_string(),
    }
}

fn apply_works_cache_keys(page: &mut ChannelWorksPage, work_type: Option<&str>) {
    page.page_key = works_cache_page_key(&page.page_key, work_type);
    page.work_type = work_type.map(ToString::to_string);
    if let Some(next_page_key) = page.next_page_key.take() {
        page.next_page_key = Some(works_cache_page_key(&next_page_key, work_type));
    }
}

fn account_with_secrets(
    app: &AppHandle,
    state: &State<'_, RuntimeState>,
    account_id: &str,
    user_id: &str,
) -> Result<(ChannelAccount, Option<String>, Option<String>), String> {
    let mut store = state.store.lock().map_err(lock_error)?;
    let account = store
        .accounts
        .iter()
        .find(|item| item.id == account_id && account_belongs_to_user(item, user_id))
        .cloned()
        .ok_or_else(|| "账号不存在".to_string())?;
    let migrated = migrate_account_secret_for_account(&mut store, &account);
    let secret = account_secret_for_account(&store, &account);
    let saved_login_cookie = secret.as_ref().and_then(|secret| secret.login_cookie.clone());
    let saved_webview_session_id = secret.as_ref().and_then(|secret| secret.webview_session_id.clone());
    if migrated {
        persist_store(app, &store)?;
    }
    Ok((account, saved_login_cookie, saved_webview_session_id))
}

fn account_content_cache_is_fresh(content: &ChannelAccountContent) -> bool {
    if content.platform_id == "xiaohongshu"
        && content.latest_work.is_some()
        && content.latest_work_thirty.is_none()
    {
        return false;
    }
    if content.platform_id == "xiaohongshu" && !xhs_latest_work_metrics_are_fresh(content) {
        return false;
    }
    if content.platform_id == "douyin" && !douyin_account_content_cache_is_current(content) {
        return false;
    }
    if content.platform_id == "wechat-channels"
        && content.latest_work.is_none()
        && content.latest_work_seven.is_none()
    {
        return false;
    }
    if content.platform_id == "bilibili"
        && (content.overview_ninety.is_none()
            || content.overview_history.is_none()
            || content.overview_total.is_none()
            || (content.latest_work.is_none() && content.latest_work_seven.is_none()))
    {
        return false;
    }
    if content.platform_id == "kuaishou" && content.overview_ninety.is_none() {
        return false;
    }
    [content.profile.as_ref().and_then(|value| value.updated_at),
     content.overview_yesterday.as_ref().and_then(|value| value.updated_at),
     content.overview_seven.as_ref().and_then(|value| value.updated_at),
     content.overview_thirty.as_ref().and_then(|value| value.updated_at),
     content.overview_ninety.as_ref().and_then(|value| value.updated_at),
     content.overview_history.as_ref().and_then(|value| value.updated_at),
     content.overview_total.as_ref().and_then(|value| value.updated_at)]
        .into_iter()
        .flatten()
        .min()
        .map(is_cache_time_fresh)
        .unwrap_or(false)
}

fn xhs_latest_work_metrics_are_fresh(content: &ChannelAccountContent) -> bool {
    let has_latest = content.latest_work.is_some()
        || content.latest_work_seven.is_some()
        || content.latest_work_thirty.is_some();
    if !has_latest {
        return true;
    }
    [
        &content.latest_work,
        &content.latest_work_seven,
        &content.latest_work_thirty,
    ]
        .into_iter()
        .flatten()
        .any(xhs_work_has_detail_metrics)
}

fn xhs_work_has_detail_metrics(work: &ChannelContentWork) -> bool {
    work.views.is_some()
        && work.impressions.is_some()
        && work.cover_click_rate.is_some()
        && work.data_updated_at.is_some()
        && work.avg_view_time.is_some()
        && [
            work.likes,
            work.comments,
            work.collects,
            work.shares,
        ]
        .into_iter()
        .any(|value| value.is_some())
}

const DOUYIN_LATEST_VIDEO_REQUIRED_METRICS: &[&str] = &[
    "danmaku",
    "avgViewSecond",
    "completionRate",
    "bounceRate",
    "completionRate5s",
    "avgViewProportion",
    "subscribe",
    "subscribeRate",
    "unsubscribe",
    "unsubscribeRate",
    "dislike",
    "dislikeRate",
];
const DOUYIN_ARTICLE_DETAIL_METRICS: &[&str] = &["descriptionSpreadRate", "imageAvgViewCount"];
const DOUYIN_WORK_PAGE_VIDEO_DETAIL_METRICS: &[&str] = &["avgViewSecond", "completionRate"];

fn douyin_account_content_cache_is_current(content: &ChannelAccountContent) -> bool {
    let Some(work) = content.latest_work.as_ref() else {
        return false;
    };
    let Some(work_type) = work.work_type.as_deref() else {
        return false;
    };
    if work
        .published_at
        .map(|published_at| Utc::now().signed_duration_since(published_at) > chrono::Duration::days(35))
        .unwrap_or(false)
    {
        return false;
    }
    let has_type_metric = match work_type {
        "video" => work_has_all_metrics(work, DOUYIN_LATEST_VIDEO_REQUIRED_METRICS),
        "article" | "image" | "note" => work_has_any_metric(work, DOUYIN_ARTICLE_DETAIL_METRICS),
        _ => false,
    };
    has_type_metric && work.views.is_some()
}

fn work_has_all_metrics(work: &ChannelContentWork, keys: &[&str]) -> bool {
    keys.iter().all(|key| work_has_metric(work, key))
}

fn work_has_any_metric(work: &ChannelContentWork, keys: &[&str]) -> bool {
    keys.iter().any(|key| work_has_metric(work, key))
}

fn work_has_metric(work: &ChannelContentWork, key: &str) -> bool {
    work.metrics.iter().any(|metric| metric.key == key)
}

fn works_page_cache_is_fresh(page: &ChannelWorksPage) -> bool {
    if page.platform_id == "douyin" && !douyin_works_page_cache_is_current(page) {
        return false;
    }
    if page.platform_id == "kuaishou" && page.work_type.as_deref() != Some("video") {
        return false;
    }
    page.updated_at.map(is_cache_time_fresh).unwrap_or(false)
}

fn douyin_works_page_cache_is_current(page: &ChannelWorksPage) -> bool {
    page.works.is_empty()
        || page.works.iter().any(douyin_work_page_item_has_detail_metrics)
}

fn douyin_work_page_item_has_detail_metrics(work: &ChannelContentWork) -> bool {
    match work.work_type.as_deref() {
        Some("video") => work_has_any_metric(work, DOUYIN_WORK_PAGE_VIDEO_DETAIL_METRICS),
        Some("article" | "image" | "note") => work_has_any_metric(work, DOUYIN_ARTICLE_DETAIL_METRICS),
        _ => false,
    }
}

fn is_cache_time_fresh(updated_at: DateTime<Utc>) -> bool {
    Utc::now().signed_duration_since(updated_at).num_seconds() < 300
}

fn cached_with_error(mut content: ChannelAccountContent, error: &str) -> ChannelAccountContent {
    content.sync_status = "failed".to_string();
    content.error = Some(error.to_string());
    if let Some(profile) = content.profile.as_mut() {
        profile.sync_status = "failed".to_string();
        profile.error = Some(error.to_string());
    }
    for overview in [
        &mut content.overview_yesterday,
        &mut content.overview_seven,
        &mut content.overview_thirty,
        &mut content.overview_ninety,
        &mut content.overview_history,
        &mut content.overview_total,
    ]
        .into_iter()
        .flatten()
    {
        overview.sync_status = "failed".to_string();
        overview.error = Some(error.to_string());
    }
    content
}

fn works_page_with_error(mut page: ChannelWorksPage, error: &str) -> ChannelWorksPage {
    page.sync_status = "failed".to_string();
    page.error = Some(error.to_string());
    page
}

fn update_account_from_content_profile(
    app: &AppHandle,
    user_id: &str,
    account_id: &str,
    profile: &ChannelAccountProfileSnapshot,
) -> Result<ChannelAccount, String> {
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let account = store
        .accounts
        .iter_mut()
        .find(|item| item.id == account_id && account_belongs_to_user(item, user_id))
        .ok_or_else(|| "账号不存在".to_string())?;
    let preserve_missing_metrics = normalize_platform_id(&account.platform_id) == "kuaishou";
    if profile.followers.is_some() || !preserve_missing_metrics {
        account.followers = profile.followers;
    }
    if profile.following.is_some() || !preserve_missing_metrics {
        account.following = profile.following;
    }
    if profile.likes.is_some() || !preserve_missing_metrics {
        account.likes = profile.likes;
    }
    account.status = AccountStatus::Active;
    account.last_sync_at = profile.last_sync_at.or(Some(Utc::now()));
    account.updated_at = Utc::now();
    let cloned = account.clone();
    persist_store(app, &store)?;
    emit_account_updated(app, &cloned);
    Ok(cloned)
}

fn is_login_expired_message(message: &str) -> bool {
    message.contains("登录已失效")
        || message.contains("登录已过期")
        || message.contains("请重新登录")
        || message.contains("请先在打开的")
}

async fn refresh_account_creator_session(
    app: &AppHandle,
    account: &ChannelAccount,
    saved_login_cookie: Option<&str>,
    saved_webview_session_id: Option<&str>,
) -> Result<CreatorSessionStatus, String> {
    if normalize_platform_id(&account.platform_id) == "kuaishou" {
        if !is_kuaishou_cookie_fallback_nickname(&account.nickname) {
            if let Ok(status) =
                kuaishou_saved_creator_session(account, saved_login_cookie, saved_webview_session_id)
            {
                return Ok(status);
            }
        }
        let mut headless_error = None;
        if let Some(profile_id) = saved_webview_session_id.map(str::trim).filter(|value| !value.is_empty()) {
            match managed_browser_fetch_kuaishou_home_info_headless(app, profile_id) {
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
                    eprintln!("[creator-session:kuaishou] headless profile api probe failed: {error}");
                    headless_error = Some(error);
                }
            }
        }
        let fallback_status =
            match check_creator_session(account, saved_login_cookie, saved_webview_session_id).await {
                Ok(status) => status,
                Err(error) if headless_error.is_some() && !is_login_expired_message(&error) => {
                    if let Some(login_cookie) = saved_login_cookie
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string)
                    {
                        eprintln!(
                            "[creator-session:kuaishou] using local account profile after cookie probe failed: {error}"
                        );
                        return Ok(CreatorSessionStatus {
                            login_cookie: Some(login_cookie.clone()),
                            webview_session_id: saved_webview_session_id.map(ToString::to_string),
                            profile: Some(kuaishou_local_account_profile(account, login_cookie)),
                        });
                    }
                    return Err(error);
                }
                Err(error) => return Err(error),
            };
        if fallback_status
            .profile
            .as_ref()
            .map(|profile| is_kuaishou_cookie_fallback_nickname(&profile.nickname))
            .unwrap_or(false)
        {
            if let Some(login_cookie) = fallback_status
                .login_cookie
                .clone()
                .or_else(|| saved_login_cookie.map(ToString::to_string))
            {
                if let Some(error) = headless_error {
                    eprintln!(
                        "[creator-session:kuaishou] using local account profile after headless probe failed: {error}"
                    );
                }
                return Ok(CreatorSessionStatus {
                    login_cookie: Some(login_cookie.clone()),
                    webview_session_id: fallback_status.webview_session_id,
                    profile: Some(kuaishou_local_account_profile(account, login_cookie)),
                });
            }
            return Err("快手账号资料未能读取到真实昵称，请重新登录后再同步。".to_string());
        }
        return Ok(fallback_status);
    }

    check_creator_session(account, saved_login_cookie, saved_webview_session_id).await
}

fn kuaishou_local_account_profile(account: &ChannelAccount, login_cookie: String) -> PluginAccountInfo {
    PluginAccountInfo {
        uid: account.uid.clone(),
        account: account.uid.clone(),
        nickname: account.nickname.clone(),
        avatar: account.avatar.clone(),
        fans_count: account.followers,
        following_count: account.following,
        like_count: account.likes,
        login_cookie,
    }
}

fn should_update_account_nickname(account: &ChannelAccount, nickname: &str) -> bool {
    let nickname = nickname.trim();
    if nickname.is_empty() {
        return false;
    }
    if normalize_platform_id(&account.platform_id) == "kuaishou"
        && !account.nickname.trim().is_empty()
        && is_kuaishou_cookie_fallback_nickname(nickname)
    {
        return false;
    }
    true
}

fn is_kuaishou_cookie_fallback_nickname(nickname: &str) -> bool {
    nickname == "快手账号"
        || nickname
            .strip_prefix("快手账号 ")
            .map(|suffix| suffix.chars().all(|ch| ch.is_ascii_digit()))
            .unwrap_or(false)
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
    let _ = delete_channel_account_content_cache(&app, &account_id);
    Ok(())
}
