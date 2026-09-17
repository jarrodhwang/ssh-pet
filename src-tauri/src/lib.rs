mod model;
mod storage;
mod terminal;
#[cfg(target_os = "macos")]
mod touchbar;

use model::{Config, Connection, PetPosition};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, PhysicalPosition,
};
use tauri_plugin_autostart::ManagerExt;

struct AppState {
    config: Mutex<Config>,
    path: PathBuf,
    home: PathBuf,
    notice: Option<String>,
    load_error: Option<String>,
    launching: std::sync::atomic::AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    config: Config,
    start_at_login: bool,
    platform: &'static str,
    notice: Option<String>,
    load_error: Option<String>,
}

fn read_config(app: &tauri::AppHandle) -> Result<Config, String> {
    app.state::<AppState>()
        .config
        .lock()
        .map(|config| config.clone())
        .map_err(|_| "Settings are temporarily unavailable.".into())
}

fn update_config(
    app: &tauri::AppHandle,
    update: impl FnOnce(&mut Config) -> Result<(), String>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    if let Some(error) = &state.load_error {
        return Err(error.clone());
    }
    let mut current = state
        .config
        .lock()
        .map_err(|_| "Settings are temporarily unavailable.")?;
    let mut next = current.clone();
    update(&mut next)?;
    storage::save(&state.path, &next)?;
    *current = next;
    drop(current);
    let _ = app.emit("config-changed", ());
    Ok(())
}

#[tauri::command]
fn get_snapshot(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    Ok(Snapshot {
        config: read_config(&app)?,
        start_at_login: app
            .autolaunch()
            .is_enabled()
            .map_err(|error| error.to_string())?,
        platform: std::env::consts::OS,
        notice: state.notice.clone(),
        load_error: state.load_error.clone(),
    })
}

#[tauri::command]
fn save_connection(app: tauri::AppHandle, mut connection: Connection) -> Result<(), String> {
    connection.name = connection.name.trim().into();
    connection.host = connection.host.trim().into();
    connection.username = connection.username.trim().into();
    connection.identity_file = connection.identity_file.trim().into();
    connection.validate()?;
    update_config(&app, |config| {
        if connection.id.is_empty() {
            connection.id = uuid::Uuid::new_v4().to_string();
            if config.connections.is_empty() {
                config.favorite_id = Some(connection.id.clone());
            }
            config.connections.push(connection);
        } else {
            let current = config
                .connections
                .iter_mut()
                .find(|item| item.id == connection.id)
                .ok_or("This connection no longer exists.")?;
            *current = connection;
        }
        Ok(())
    })?;
    refresh_shortcuts(&app);
    Ok(())
}

#[tauri::command]
fn import_command(app: tauri::AppHandle, command: String, name: String) -> Result<(), String> {
    if command.len() > 8192 {
        return Err("The command is too long.".into());
    }
    let mut connection = model::parse_launcher(&command, name.trim())?;
    connection.id.clear();
    save_connection(app, connection)
}

#[tauri::command]
fn delete_connection(app: tauri::AppHandle, id: String) -> Result<(), String> {
    update_config(&app, |config| {
        if !config.connections.iter().any(|item| item.id == id) {
            return Err("This connection no longer exists.".into());
        }
        config.connections.retain(|item| item.id != id);
        if config.favorite_id.as_deref() == Some(&id) {
            config.favorite_id = config.connections.first().map(|item| item.id.clone());
        }
        Ok(())
    })?;
    refresh_shortcuts(&app);
    Ok(())
}

#[tauri::command]
fn set_favorite(app: tauri::AppHandle, id: String) -> Result<(), String> {
    update_config(&app, |config| {
        if !config.connections.iter().any(|item| item.id == id) {
            return Err("This connection no longer exists.".into());
        }
        config.favorite_id = Some(id);
        Ok(())
    })?;
    refresh_shortcuts(&app);
    Ok(())
}

#[tauri::command]
async fn connect(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = read_config(&app)?;
    let connection = config
        .connections
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("This connection no longer exists.")?;
    if state
        .launching
        .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        return Err("A Terminal window is already opening. Give it a moment.".into());
    }
    let home = state.home.clone();
    let _ = app.emit("launch-state", "opening");
    let result = tauri::async_runtime::spawn_blocking(move || terminal::launch(&connection, &home))
        .await
        .map_err(|error| error.to_string())
        .and_then(|value| value);
    state
        .launching
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = app.emit(
        "launch-state",
        if result.is_ok() { "opened" } else { "error" },
    );
    if let Err(error) = &result {
        let _ = app.emit("app-error", error);
    }
    result
}

fn connect_favorite(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match read_config(&app).and_then(|config| {
            config
                .favorite_id
                .ok_or("Add a connection to get started.".into())
        }) {
            Ok(id) => {
                if let Err(error) = connect(app.clone(), id).await {
                    report_error(&app, error);
                }
            }
            Err(_) => {
                let _ = show_launcher(app);
            }
        }
    });
}

fn report_error(app: &tauri::AppHandle, error: String) {
    let _ = show_launcher(app.clone());
    let _ = app.emit("app-error", error);
}

#[tauri::command]
fn show_launcher(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.unminimize().map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn hide_launcher(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn set_preferences(
    app: tauri::AppHandle,
    pet_visible: bool,
    reduce_motion: bool,
) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or("The pet window is unavailable.")?;
    if pet_visible { pet.show() } else { pet.hide() }.map_err(|error| error.to_string())?;
    if let Err(error) = update_config(&app, |config| {
        config.pet_visible = pet_visible;
        config.reduce_motion = reduce_motion;
        Ok(())
    }) {
        if let Ok(config) = read_config(&app) {
            let _ = if config.pet_visible {
                pet.show()
            } else {
                pet.hide()
            };
        }
        return Err(error);
    }
    refresh_shortcuts(&app);
    Ok(())
}

#[tauri::command]
fn set_start_at_login(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|error| error.to_string())?;
    let _ = app.emit("config-changed", ());
    Ok(())
}

fn position_pet(app: &tauri::AppHandle, saved: Option<PetPosition>) -> Result<(), String> {
    let window = app
        .get_webview_window("pet")
        .ok_or("The pet window is unavailable.")?;
    let monitors = window
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let valid = saved.filter(|position| {
        monitors.iter().any(|monitor| {
            let origin = monitor.position();
            let bounds = monitor.size();
            position.x >= origin.x
                && position.y >= origin.y
                && i64::from(position.x) + i64::from(size.width)
                    <= i64::from(origin.x) + i64::from(bounds.width)
                && i64::from(position.y) + i64::from(size.height)
                    <= i64::from(origin.y) + i64::from(bounds.height)
        })
    });
    let position = if let Some(position) = valid {
        PhysicalPosition::new(position.x, position.y)
    } else if let Some(monitor) = window
        .primary_monitor()
        .map_err(|error| error.to_string())?
    {
        let scale = monitor.scale_factor();
        PhysicalPosition::new(
            monitor.position().x + monitor.size().width as i32
                - size.width as i32
                - (32.0 * scale) as i32,
            monitor.position().y + monitor.size().height as i32
                - size.height as i32
                - (100.0 * scale) as i32,
        )
    } else {
        PhysicalPosition::new(40, 100)
    };
    window
        .set_position(position)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn reset_pet_position(app: tauri::AppHandle) -> Result<(), String> {
    position_pet(&app, None)?;
    update_config(&app, |config| {
        config.pet_position = None;
        Ok(())
    })
}

#[tauri::command]
fn save_pet_position(app: tauri::AppHandle) -> Result<(), String> {
    let position = app
        .get_webview_window("pet")
        .ok_or("The pet window is unavailable.")?
        .outer_position()
        .map_err(|error| error.to_string())?;
    update_config(&app, |config| {
        config.pet_position = Some(PetPosition {
            x: position.x,
            y: position.y,
        });
        Ok(())
    })
}

fn make_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let config = read_config(app).unwrap_or_default();
    let name = config
        .connections
        .iter()
        .find(|item| Some(&item.id) == config.favorite_id.as_ref())
        .map(|item| item.name.as_str());
    let connect = MenuItem::with_id(
        app,
        "connect-favorite",
        name.map(|name| format!("Connect to {name}"))
            .unwrap_or("Add your first connection…".into()),
        true,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "open", "Open Droplet…", true, None::<&str>)?;
    let pet = MenuItem::with_id(
        app,
        "toggle-pet",
        if config.pet_visible {
            "Hide desktop pet"
        } else {
            "Show desktop pet"
        },
        true,
        None::<&str>,
    )?;
    let reset = MenuItem::with_id(
        app,
        "reset-pet",
        "Bring pet back to screen",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit Droplet", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    Menu::with_items(
        app,
        &[
            &connect,
            &open,
            &separator,
            &pet,
            &reset,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

fn refresh_shortcuts(app: &tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id("droplet") {
        if let Ok(menu) = make_tray_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
    #[cfg(target_os = "macos")]
    touchbar::refresh(app);
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| { let _ = show_launcher(app.clone()); }))
        .plugin(tauri_plugin_autostart::Builder::new().args(["--background"]).build())
        .invoke_handler(tauri::generate_handler![get_snapshot, save_connection, import_command, delete_connection, set_favorite, connect, show_launcher, hide_launcher, set_preferences, set_start_at_login, reset_pet_position, save_pet_position])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("connections.json");
            let home = app.path().home_dir()?;
            let mut notice = None;
            let mut load_error = None;
            let config = match storage::load(&path) {
                Ok(Some(config)) => config,
                Ok(None) => {
                    let mut config = Config::default();
                    let launcher = home.join("Desktop/zbook-studio-ssh.command");
                    if launcher.is_file() {
                        match std::fs::read_to_string(launcher).map_err(|error| error.to_string()).and_then(|text| model::parse_launcher(&text, "Zbook Studio")) {
                            Ok(connection) => {
                                config.favorite_id = Some(connection.id.clone());
                                config.connections.push(connection);
                                notice = Some("Your Zbook Studio connection was imported from the Desktop launcher.".into());
                            }
                            Err(error) => notice = Some(format!("The Desktop launcher could not be imported: {error}")),
                        }
                    }
                    if let Err(error) = storage::save(&path, &config) { load_error = Some(error); }
                    config
                }
                Err(error) => {
                    load_error = Some(format!("{error} Your original file is unchanged at {}. Repair or move it, then reopen Droplet.", path.display()));
                    Config::default()
                }
            };
            let pet_visible = config.pet_visible;
            let pet_position = config.pet_position.clone();
            app.manage(AppState { config: Mutex::new(config), path, home, notice, load_error, launching: std::sync::atomic::AtomicBool::new(false) });
            let handle = app.handle();
            TrayIconBuilder::with_id("droplet")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
                .icon_as_template(true)
                .tooltip("Droplet · your SSH companion")
                .menu(&make_tray_menu(handle)?)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "connect-favorite" => connect_favorite(app),
                    "open" => { let _ = show_launcher(app.clone()); },
                    "toggle-pet" => if let Ok(config) = read_config(app) {
                        if let Err(error) = set_preferences(app.clone(), !config.pet_visible, config.reduce_motion) { report_error(app, error); }
                    },
                    "reset-pet" => {
                        if let Err(error) = reset_pet_position(app.clone()).and_then(|_| {
                            let config = read_config(app)?;
                            set_preferences(app.clone(), true, config.reduce_motion)
                        }) { report_error(app, error); }
                    },
                    "quit" => app.exit(0),
                    _ => {},
                })
                .build(app)?;
            position_pet(handle, pet_position)?;
            if pet_visible { if let Some(pet) = app.get_webview_window("pet") { pet.show()?; } }
            #[cfg(target_os = "macos")]
            touchbar::install(handle);
            if !std::env::args().any(|arg| arg == "--background") { show_launcher(handle.clone())?; }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "main" { let _ = window.hide(); }
                else if let Ok(config) = read_config(window.app_handle()) { let _ = set_preferences(window.app_handle().clone(), false, config.reduce_motion); }
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not start Droplet");
    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            let _ = show_launcher(app.clone());
        }
        let _ = (app, event);
    });
}
