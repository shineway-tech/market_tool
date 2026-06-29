use super::*;

pub(super) fn task_suffix(task_id: &str) -> String {
    task_id.chars().take(8).collect()
}

pub(super) fn stable_label_fragment(value: &str) -> String {
    format!("{:016x}", stable_hash(value, 0xcbf29ce484222325))
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

pub(super) fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("内部状态锁定失败: {error}")
}
