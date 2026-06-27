use super::*;

fn ensure_close_controls(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.set_decorations(true);
    let _ = window.set_closable(true);
    let _ = window.set_resizable(true);
}

fn destroy_window_handle(window: &WebviewWindow<tauri::Wry>) {
    let _ = window.hide();
    let _ = window.destroy();
}

pub(crate) fn destroy_webview_window(window: &WebviewWindow<tauri::Wry>) {
    destroy_window_handle(window);
}

pub(crate) fn prepare_external_webview_window(window: &WebviewWindow<tauri::Wry>) {
    ensure_close_controls(window);
}

pub(crate) fn prepare_creator_home_window(window: &WebviewWindow<tauri::Wry>) {
    ensure_close_controls(window);
}
