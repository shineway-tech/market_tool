use super::*;

fn ensure_close_controls(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.set_decorations(true);
    let _ = window.set_closable(true);
    let _ = window.set_resizable(true);
}

fn hide_window(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.hide();
}

fn destroy_window_handle(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.hide();
    let _ = window.destroy();
}

fn hide_creator_home_window_on_close(window: &WebviewWindow<tauri::Wry>) {
    let window_for_close = window.clone();

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // Keep creator-home WebView2 instances reusable on Windows. The close path must stay
            // deterministic: prevent native destruction and hide only. Navigation or destruction
            // here can race with the next open or lock the WebView2 profile.
            api.prevent_close();
            hide_window(&window_for_close);
        }
    });
}

pub(crate) fn destroy_webview_window(window: &WebviewWindow<tauri::Wry>) {
    destroy_window_handle(window);
}

pub(crate) fn hide_creator_home_windows_except(app: &AppHandle, keep_label: &str) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("creator-home-") && label != keep_label {
            hide_window(&window);
        }
    }
}

pub(crate) fn prepare_external_webview_window(window: &WebviewWindow<tauri::Wry>) {
    ensure_close_controls(window);
}

pub(crate) fn prepare_creator_home_window(window: &WebviewWindow<tauri::Wry>) {
    ensure_close_controls(window);
    hide_creator_home_window_on_close(window);
}
