use super::*;

#[tauri::command]
pub(crate) async fn get_bootstrap(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    user_id: String,
) -> Result<Bootstrap, String> {
    services::channel_service::get_bootstrap(app, state, user_id).await
}

#[tauri::command]
pub(crate) async fn list_channel_accounts(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    user_id: String,
) -> Result<Vec<ChannelAccount>, String> {
    services::channel_service::list_channel_accounts(app, state, user_id).await
}

#[tauri::command]
pub(crate) fn save_auth_settings(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: SaveSettingsRequest,
) -> Result<AuthSettings, String> {
    services::channel_service::save_auth_settings(app, state, request)
}

#[tauri::command]
pub(crate) async fn start_channel_login(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    request: StartLoginRequest,
) -> Result<StartLoginResponse, String> {
    services::channel_service::start_channel_login(app, state, request).await
}

#[tauri::command]
pub(crate) async fn get_auth_task_status(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    task_id: String,
    user_id: String,
) -> Result<AuthTaskStatus, String> {
    services::channel_service::get_auth_task_status(app, state, task_id, user_id).await
}

#[tauri::command]
pub(crate) async fn refresh_channel_account(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    services::channel_service::refresh_channel_account(app, state, account_id, user_id).await
}

#[tauri::command]
pub(crate) async fn mark_channel_account_unavailable(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    services::channel_service::mark_channel_account_unavailable(app, state, account_id, user_id).await
}

#[tauri::command]
pub(crate) async fn open_account_homepage(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<ChannelAccount, String> {
    services::channel_service::open_account_homepage(app, state, account_id, user_id).await
}

#[tauri::command]
pub(crate) async fn delete_channel_account(
    app: AppHandle,
    state: State<'_, RuntimeState>,
    account_id: String,
    user_id: String,
) -> Result<(), String> {
    services::channel_service::delete_channel_account(app, state, account_id, user_id).await
}
