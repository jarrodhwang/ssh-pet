use serde::Serialize;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    sync::OnceLock,
};
use tauri::Manager;

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

extern "C" {
    fn droplet_install_touchbar(
        window: *mut std::ffi::c_void,
        callback: extern "C" fn(*const c_char),
    );
    fn droplet_update_touchbar(connections: *const c_char);
}

#[derive(Serialize)]
struct TouchBarConnection<'a> {
    id: &'a str,
    name: &'a str,
    favorite: bool,
    enabled: bool,
}

extern "C" fn on_action(connection_id: *const c_char) {
    if let Some(app) = APP.get() {
        if connection_id.is_null() {
            let _ = super::show_launcher(app.clone());
            return;
        }
        let id = unsafe { CStr::from_ptr(connection_id) }
            .to_string_lossy()
            .into_owned();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = super::launch(&app, Some(&id)).await {
                super::report_error(&app, error);
            }
        });
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
    let Ok(config) = super::read_config(app) else {
        return;
    };
    let enabled = app
        .state::<super::AppState>()
        .core
        .view("")
        .ok()
        .and_then(|view| {
            view.connections
                .first()
                .map(|connection| connection.can_connect)
        })
        .unwrap_or(false);
    let mut connections: Vec<_> = config
        .connections
        .iter()
        .map(|connection| TouchBarConnection {
            id: &connection.id,
            name: &connection.name,
            favorite: Some(&connection.id) == config.favorite_id.as_ref(),
            enabled,
        })
        .collect();
    connections.sort_by_key(|connection| !connection.favorite);
    let Ok(connections) = serde_json::to_string(&connections) else {
        return;
    };
    let Ok(connections) = CString::new(connections) else {
        return;
    };
    let _ = app.run_on_main_thread(move || unsafe {
        droplet_update_touchbar(connections.as_ptr());
    });
}
