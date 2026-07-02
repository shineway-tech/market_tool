use super::*;

pub(super) fn allocate_local_port() -> Result<u16, String> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("分配浏览器调试端口失败: {error}"))
}

pub(super) fn find_chromium_browser() -> Option<PathBuf> {
    browser_candidates()
        .into_iter()
        .find(|path| path.exists() && is_executable_file(path))
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(target_os = "macos")]
fn browser_candidates() -> Vec<PathBuf> {
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    let mut paths = vec![
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
        "/Applications/Chromium.app/Contents/MacOS/Chromium".into(),
        "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary".into(),
    ];
    if let Some(home) = home {
        paths.extend([
            home.join("Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            home.join("Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
            home.join("Applications/Chromium.app/Contents/MacOS/Chromium"),
        ]);
    }
    paths
}

#[cfg(target_os = "windows")]
fn browser_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for env_name in ["LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)"] {
        if let Some(root) = std::env::var_os(env_name).map(PathBuf::from) {
            paths.extend([
                root.join("Google/Chrome/Application/chrome.exe"),
                root.join("Microsoft/Edge/Application/msedge.exe"),
                root.join("Chromium/Application/chrome.exe"),
            ]);
        }
    }
    paths
}

#[cfg(all(unix, not(target_os = "macos")))]
fn browser_candidates() -> Vec<PathBuf> {
    vec![
        "/usr/bin/google-chrome".into(),
        "/usr/bin/google-chrome-stable".into(),
        "/usr/bin/microsoft-edge".into(),
        "/usr/bin/chromium".into(),
        "/usr/bin/chromium-browser".into(),
    ]
}
