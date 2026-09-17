use std::{ffi::CString, sync::OnceLock};
use tauri::Manager;

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

extern "C" {
    fn droplet_install_touchbar(window: *mut std::ffi::c_void, callback: extern "C" fn(i32));
    fn droplet_update_touchbar(title: *const std::ffi::c_char, enabled: bool);
}

extern "C" fn on_action(action: i32) {
    if let Some(app) = APP.get() {
        match action {
            1 => super::connect_favorite(app),
            _ => {
                let _ = super::show_launcher(app.clone());
            }
        }
    }
}

pub fn install(app: &tauri::AppHandle) {
    let _ = APP.set(app.clone());
    for label in ["main", "pet"] {
        if let Some(window) = app.get_webview_window(label) {
            if let Ok(pointer) = window.ns_window() {
                // Setup runs on the main thread; AppKit retains the bar on the window.
                unsafe {
                    droplet_install_touchbar(pointer, on_action);
                }
            }
        }
    }
    refresh(app);
}

pub fn refresh(app: &tauri::AppHandle) {
    if let Ok(config) = super::read_config(app) {
        let favorite = config
            .connections
            .iter()
            .find(|item| Some(&item.id) == config.favorite_id.as_ref());
        let enabled = app
            .state::<super::AppState>()
            .core
            .pet()
            .map(|v| v.can_connect)
            .unwrap_or(false);
        let title = favorite
            .map(|item| format!("Connect to {}", item.name))
            .unwrap_or_else(|| "Add a connection".into());
        if let Ok(title) = CString::new(title) {
            let _ = app.run_on_main_thread(move || unsafe {
                droplet_update_touchbar(title.as_ptr(), enabled);
            });
        }
    }
}
