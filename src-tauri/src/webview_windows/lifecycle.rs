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

fn schedule_hide_after_close_request(
    window: WebviewWindow<tauri::Wry>,
    closing: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));

        let window_for_main = window.clone();
        let closing_for_main = closing.clone();
        if window
            .run_on_main_thread(move || {
                hide_window(&window_for_main);
                closing_for_main.store(false, std::sync::atomic::Ordering::SeqCst);
            })
            .is_err()
        {
            closing.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });
}

fn hide_creator_home_window_on_close(window: &WebviewWindow<tauri::Wry>) {
    let window_for_close = window.clone();
    let closing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let closing_for_event = closing.clone();

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // Keep creator-home WebView2 instances reusable on Windows. Do not hide or navigate
            // synchronously inside CloseRequested; WebView2 can deadlock while the native close
            // message is still being handled.
            api.prevent_close();
            if !closing_for_event.swap(true, std::sync::atomic::Ordering::SeqCst) {
                schedule_hide_after_close_request(window_for_close.clone(), closing_for_event.clone());
            }
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
