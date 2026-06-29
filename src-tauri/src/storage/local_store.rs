use crate::*;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const SETTINGS_KEY: &str = "auth_settings";
const LEGACY_STORE_FILE: &str = "channel-auth-store.json";
const LEGACY_BACKUP_FILE: &str = "channel-auth-store.legacy.json";
const LOCAL_DB_FILE: &str = "local.db";

pub(crate) fn upsert_account_secret(app: &AppHandle, account_id: &str, login_cookie: &str) -> Result<(), String> {
    if login_cookie.trim().is_empty() {
        return Ok(());
    }
    let runtime = app.state::<RuntimeState>();
    let mut store = runtime.store.lock().map_err(lock_error)?;
    let secret = store.account_secrets.entry(account_id.to_string()).or_default();
    secret.login_cookie = Some(login_cookie.to_string());
    persist_store(app, &store)
}

pub(crate) fn upsert_account_webview_session(
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

pub(crate) fn emit_account_updated(app: &AppHandle, account: &ChannelAccount) {
    let _ = app.emit(CHANNEL_ACCOUNT_UPDATED_EVENT, account);
}

pub(crate) fn mark_account_expired(app: &AppHandle, account_id: &str) -> Result<ChannelAccount, String> {
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

pub(crate) fn update_plugin_account_profile(
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


pub(crate) fn upsert_account_for_user(
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
                && item.uid == account.uid
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


pub(crate) fn load_store(app: &AppHandle) -> Result<StoreFile, Box<dyn std::error::Error>> {
    let db_path = local_db_path(app)?;
    let legacy_path = store_path(app)?;
    let is_new_database = !db_path.exists();
    let mut conn = open_local_db(&db_path)?;
    init_local_db(&conn)?;

    if is_new_database && legacy_path.exists() {
        let text = fs::read_to_string(&legacy_path)?;
        let mut store: StoreFile = serde_json::from_str(&text)?;
        store.settings = normalize_settings(store.settings);
        write_store_to_db(&mut conn, &store)?;
        backup_legacy_store(&legacy_path)?;
        return Ok(store);
    }

    let mut store = read_store_from_db(&conn)?;
    store.settings = normalize_settings(store.settings);
    Ok(store)
}

pub(crate) fn persist_store(app: &AppHandle, store: &StoreFile) -> Result<(), String> {
    let db_path = local_db_path(app).map_err(|error| error.to_string())?;
    let mut conn = open_local_db(&db_path).map_err(|error| error.to_string())?;
    init_local_db(&conn).map_err(|error| error.to_string())?;
    write_store_to_db(&mut conn, store).map_err(|error| error.to_string())
}

pub(crate) fn store_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    Ok(dir.join(LEGACY_STORE_FILE))
}

fn local_db_path(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    Ok(dir.join(LOCAL_DB_FILE))
}

fn open_local_db(path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(Connection::open(path)?)
}

fn init_local_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS app_migrations (
          name TEXT PRIMARY KEY,
          applied_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS app_kv (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS platform_accounts (
          id TEXT PRIMARY KEY,
          user_id TEXT,
          platform_id TEXT NOT NULL,
          uid TEXT NOT NULL,
          nickname TEXT NOT NULL,
          avatar TEXT NOT NULL,
          followers INTEGER,
          likes INTEGER,
          status TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          last_sync_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_platform_accounts_user_platform
          ON platform_accounts(user_id, platform_id, updated_at);

        CREATE TABLE IF NOT EXISTS platform_sessions (
          account_id TEXT PRIMARY KEY,
          login_cookie TEXT,
          browser_profile_id TEXT,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS account_sync_logs (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          user_id TEXT,
          account_id TEXT NOT NULL,
          action TEXT NOT NULL,
          status TEXT NOT NULL,
          message TEXT,
          created_at TEXT NOT NULL
        );
        "#,
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO app_migrations(name, applied_at) VALUES(?1, ?2)",
        params!["initial_sqlite_store", Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn read_store_from_db(conn: &Connection) -> Result<StoreFile, Box<dyn std::error::Error>> {
    let settings = conn
        .query_row(
            "SELECT value FROM app_kv WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|text| serde_json::from_str::<AuthSettings>(&text).ok())
        .map(normalize_settings)
        .unwrap_or_else(default_auth_settings);

    let mut account_statement = conn.prepare(
        r#"
        SELECT id, user_id, platform_id, uid, nickname, avatar, followers, likes,
               status, created_at, updated_at, last_sync_at
          FROM platform_accounts
         ORDER BY platform_id ASC, updated_at DESC
        "#,
    )?;
    let accounts = account_statement
        .query_map([], |row| {
            let followers = row.get::<_, Option<i64>>(6)?.and_then(i64_to_u64);
            let likes = row.get::<_, Option<i64>>(7)?.and_then(i64_to_u64);
            let created_at = parse_db_time(row.get::<_, String>(9)?);
            let updated_at = parse_db_time(row.get::<_, String>(10)?);
            let last_sync_at = row
                .get::<_, Option<String>>(11)?
                .map(parse_db_time);

            Ok(ChannelAccount {
                id: row.get(0)?,
                user_id: row.get(1)?,
                platform_id: row.get(2)?,
                uid: row.get(3)?,
                nickname: row.get(4)?,
                avatar: row.get(5)?,
                followers,
                likes,
                status: account_status_from_db(row.get::<_, String>(8)?.as_str()),
                created_at,
                updated_at,
                last_sync_at,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut secret_statement = conn.prepare(
        "SELECT account_id, login_cookie, browser_profile_id FROM platform_sessions",
    )?;
    let account_secrets = secret_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                AccountSecret {
                    login_cookie: row.get(1)?,
                    webview_session_id: row.get(2)?,
                },
            ))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;

    Ok(StoreFile {
        accounts,
        settings,
        account_secrets,
    })
}

fn write_store_to_db(conn: &mut Connection, store: &StoreFile) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO app_kv(key, value, updated_at) VALUES(?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![
            SETTINGS_KEY,
            serde_json::to_string(&normalize_settings(store.settings.clone()))?,
            Utc::now().to_rfc3339(),
        ],
    )?;
    tx.execute("DELETE FROM platform_accounts", [])?;
    tx.execute("DELETE FROM platform_sessions", [])?;

    for account in &store.accounts {
        tx.execute(
            r#"
            INSERT INTO platform_accounts(
              id, user_id, platform_id, uid, nickname, avatar, followers, likes,
              status, created_at, updated_at, last_sync_at
            ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                &account.id,
                account.user_id.as_deref(),
                &account.platform_id,
                &account.uid,
                &account.nickname,
                &account.avatar,
                account.followers.map(u64_to_i64),
                account.likes.map(u64_to_i64),
                account_status_to_db(&account.status),
                account.created_at.to_rfc3339(),
                account.updated_at.to_rfc3339(),
                account.last_sync_at.as_ref().map(DateTime::to_rfc3339),
            ],
        )?;
    }

    for (account_id, secret) in &store.account_secrets {
        tx.execute(
            r#"
            INSERT INTO platform_sessions(account_id, login_cookie, browser_profile_id, updated_at)
            VALUES(?1, ?2, ?3, ?4)
            "#,
            params![
                account_id,
                secret.login_cookie.as_deref(),
                secret.webview_session_id.as_deref(),
                Utc::now().to_rfc3339(),
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn backup_legacy_store(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let backup_path = path.with_file_name(LEGACY_BACKUP_FILE);
    if !backup_path.exists() {
        fs::copy(path, backup_path)?;
    }
    Ok(())
}

fn parse_db_time(value: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn account_status_to_db(status: &AccountStatus) -> &'static str {
    match status {
        AccountStatus::Active => "active",
        AccountStatus::Expired => "expired",
        AccountStatus::Pending => "pending",
    }
}

fn account_status_from_db(status: &str) -> AccountStatus {
    match status {
        "expired" => AccountStatus::Expired,
        "pending" => AccountStatus::Pending,
        _ => AccountStatus::Active,
    }
}

fn i64_to_u64(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}


pub(crate) fn normalize_user_id(value: &str) -> Result<String, String> {
    let user_id = value.trim();
    if user_id.is_empty() {
        return Err("当前登录状态无效，请重新登录".to_string());
    }
    Ok(user_id.to_string())
}

pub(crate) fn account_belongs_to_user(account: &ChannelAccount, user_id: &str) -> bool {
    account.user_id.as_deref() == Some(user_id)
}

pub(crate) fn user_accounts(store: &StoreFile, user_id: &str) -> Vec<ChannelAccount> {
    store
        .accounts
        .iter()
        .filter(|account| account_belongs_to_user(account, user_id))
        .cloned()
        .collect()
}

pub(crate) fn claim_legacy_accounts_for_user(store: &mut StoreFile, user_id: &str) -> bool {
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

pub(crate) fn account_secret_for_account(store: &StoreFile, account: &ChannelAccount) -> Option<AccountSecret> {
    account_secret_candidates(account)
        .into_iter()
        .find_map(|key| store.account_secrets.get(&key).cloned())
}

pub(crate) fn migrate_account_secret_for_account(store: &mut StoreFile, account: &ChannelAccount) -> bool {
    let keys = account_secret_candidates(account);
    migrate_account_secret_from_keys(store, &account.id, &keys)
}

pub(crate) fn migrate_account_secret_from_keys(
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

pub(crate) fn account_secret_candidates(account: &ChannelAccount) -> Vec<String> {
    let mut values = Vec::new();
    push_unique(&mut values, account.id.clone());
    if let Some(user_id) = account.user_id.as_deref() {
        if let Some(raw_id) = unscoped_account_id(user_id, &account.id) {
            push_unique(&mut values, raw_id);
        }
    }
    values
}

pub(crate) fn push_unique(values: &mut Vec<String>, value: String) {
    if !value.trim().is_empty() && !values.iter().any(|item| item == &value) {
        values.push(value);
    }
}

pub(crate) fn scoped_account_for_user(user_id: &str, mut account: ChannelAccount) -> ChannelAccount {
    account.user_id = Some(user_id.to_string());
    account.id = scoped_account_id(user_id, &account.id);
    account
}

pub(crate) fn scoped_account_id(user_id: &str, account_id: &str) -> String {
    let prefix = format!("u{}_", stable_label_fragment(user_id));
    if account_id.starts_with(&prefix) {
        account_id.to_string()
    } else {
        format!("{prefix}{account_id}")
    }
}

pub(crate) fn unscoped_account_id(user_id: &str, account_id: &str) -> Option<String> {
    let prefix = format!("u{}_", stable_label_fragment(user_id));
    account_id
        .strip_prefix(&prefix)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_store_roundtrips_accounts_and_sessions() {
        let mut conn = Connection::open_in_memory().expect("open sqlite memory database");
        init_local_db(&conn).expect("initialize local database");
        let now = Utc::now();
        let account = ChannelAccount {
            id: "account-1".to_string(),
            user_id: Some("user-1".to_string()),
            platform_id: "xiaohongshu".to_string(),
            uid: "xhs-user".to_string(),
            nickname: "小红书账号".to_string(),
            avatar: "https://example.test/avatar.png".to_string(),
            followers: Some(123),
            likes: Some(45),
            status: AccountStatus::Active,
            created_at: now,
            updated_at: now,
            last_sync_at: Some(now),
        };
        let mut account_secrets = HashMap::new();
        account_secrets.insert(
            account.id.clone(),
            AccountSecret {
                login_cookie: Some("a=b".to_string()),
                webview_session_id: Some("profile-1".to_string()),
            },
        );
        let store = StoreFile {
            accounts: vec![account],
            settings: default_auth_settings(),
            account_secrets,
        };

        write_store_to_db(&mut conn, &store).expect("write sqlite store");
        let loaded = read_store_from_db(&conn).expect("read sqlite store");

        assert_eq!(loaded.accounts.len(), 1);
        assert_eq!(loaded.accounts[0].user_id.as_deref(), Some("user-1"));
        assert_eq!(loaded.accounts[0].platform_id, "xiaohongshu");
        assert_eq!(loaded.accounts[0].followers, Some(123));
        assert_eq!(loaded.account_secrets["account-1"].login_cookie.as_deref(), Some("a=b"));
        assert_eq!(
            loaded.account_secrets["account-1"].webview_session_id.as_deref(),
            Some("profile-1")
        );
    }
}
