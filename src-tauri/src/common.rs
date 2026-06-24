use super::*;

pub(super) fn task_suffix(task_id: &str) -> String {
    task_id.chars().take(8).collect()
}

pub(super) fn task_data_store_identifier(task_id: &str) -> [u8; 16] {
    Uuid::parse_str(task_id)
        .map(|uuid| *uuid.as_bytes())
        .unwrap_or_else(|_| *Uuid::new_v4().as_bytes())
}

pub(super) fn stable_label_fragment(value: &str) -> String {
    format!("{:016x}", stable_hash(value, 0xcbf29ce484222325))
}

pub(super) fn stable_data_store_identifier(value: &str) -> [u8; 16] {
    let first = stable_hash(value, 0xcbf29ce484222325);
    let second = stable_hash(value, 0x84222325cbf29ce4);
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&first.to_le_bytes());
    bytes[8..].copy_from_slice(&second.to_le_bytes());
    bytes
}

pub(super) fn stable_hash(value: &str, seed: u64) -> u64 {
    value.as_bytes().iter().fold(seed, |hash, byte| {
        (hash ^ (*byte as u64)).wrapping_mul(0x100000001b3)
    })
}


pub(super) fn encode_query(value: &str) -> String {
    form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

pub(super) fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开授权窗口失败: {error}"))
}

pub(super) fn success_page(nickname: &str) -> String {
    format!(
        r#"<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>授权成功</title><body style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#07181b;color:#dff7ef;display:grid;place-items:center;min-height:100vh"><main style="text-align:center"><h1>授权成功</h1><p>{nickname} 已连接到营销大师。</p><p style="color:#7f969d">可以关闭这个窗口并回到客户端。</p></main></body></html>"#
    )
}

pub(super) fn error_page(message: &str) -> String {
    format!(
        r#"<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>授权失败</title><body style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#07181b;color:#ffe3e5;display:grid;place-items:center;min-height:100vh"><main style="max-width:560px;text-align:center"><h1>授权没有完成</h1><p>{message}</p><p style="color:#7f969d">请回到客户端查看授权状态。</p></main></body></html>"#
    )
}

pub(super) fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("内部状态锁定失败: {error}")
}
