use chrono::{DateTime, Utc};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    process::Command,
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager, State};
use url::{form_urlencoded, Url};
use uuid::Uuid;

mod channel_urls;
mod browser;
mod commands;
mod common;
mod domain;
mod json_ext;
mod platforms;
mod services;
mod settings;
mod state;
mod storage;

use channel_urls::*;
use browser::*;
use common::*;
use domain::*;
use json_ext::*;
use platforms::*;
use settings::*;
use state::*;
use storage::local_store::*;

const CHANNEL_ACCOUNT_UPDATED_EVENT: &str = "channel-account-updated";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let store = load_store(&app.handle())?;
            app.manage(RuntimeState {
                store: Mutex::new(store),
                pending_auth: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::channel::get_bootstrap,
            commands::channel::list_channel_accounts,
            commands::channel::save_auth_settings,
            commands::channel::start_channel_login,
            commands::channel::get_auth_task_status,
            commands::channel::refresh_channel_account,
            commands::channel::sync_channel_account_content,
            commands::channel::load_channel_account_works_page,
            commands::channel::mark_channel_account_unavailable,
            commands::channel::open_account_homepage,
            commands::channel::delete_channel_account
        ])
        .run(tauri::generate_context!())
        .expect("error while running marketing master");
}
