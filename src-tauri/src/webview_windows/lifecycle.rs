use super::*;

fn ensure_close_controls(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.set_decorations(true);
    let _ = window.set_closable(true);
    let _ = window.set_resizable(true);
}

fn blank_url() -> Option<Url> {
    Url::parse("about:blank").ok()
}

fn destroy_window_handle(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.hide();
    let _ = window.destroy();
}

fn hide_reusable_window(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.hide();
    if let Some(url) = blank_url() {
        let _ = window.navigate(url);
    }
}

fn hide_reusable_window_on_close(window: &WebviewWindow<tauri::Wry>) {
    let window_for_close = window.clone();
    let closing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let closing_for_event = closing.clone();

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // Keep creator-home WebView2 instances reusable on Windows; destroying them can leave
            // the profile locked long enough that the next open hangs.
            api.prevent_close();
            if !closing_for_event.swap(true, std::sync::atomic::Ordering::SeqCst) {
                hide_reusable_window(&window_for_close);
            }
            closing_for_event.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });
}

pub(crate) fn destroy_webview_window(window: &WebviewWindow<tauri::Wry>) {
    destroy_window_handle(window);
}

pub(crate) fn hide_creator_home_windows_except(app: &AppHandle, keep_label: &str) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("creator-home-") && label != keep_label {
            hide_reusable_window(&window);
        }
    }
}

pub(crate) fn prepare_external_webview_window(window: &WebviewWindow<tauri::Wry>) {
    ensure_close_controls(window);
}

pub(crate) fn prepare_creator_home_window(window: &WebviewWindow<tauri::Wry>) {
    ensure_close_controls(window);
    hide_reusable_window_on_close(window);
}
