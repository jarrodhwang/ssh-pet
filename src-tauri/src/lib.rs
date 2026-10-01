mod terminal;
#[cfg(target_os = "macos")]
mod touchbar;

use droplet_core::{
    model::{Config, PetPosition, Preferences},
    protocol::{MainRequest, PetReply, PetRequest, Reply},
    security, AppError, Core, ErrorCode, Result,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, PhysicalPosition,
};
use tauri_plugin_autostart::ManagerExt;

struct Drag {
    x: f64,
    y: f64,
    origin: PhysicalPosition<i32>,
    scale: f64,
    started: Instant,
}
struct AppState {
    core: Arc<Core>,
    drag: Mutex<Option<Drag>>,
}
fn os_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Process, error.to_string())
}
fn read_config(app: &tauri::AppHandle) -> Result<Config> {
    app.try_state::<AppState>()
        .ok_or_else(|| os_error("Droplet is still starting."))?
        .core
        .config()
}

#[tauri::command]
async fn main_command(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    request: MainRequest,
) -> Result<Reply> {
    security::authorize(window.label(), "main")?;
    dispatch(&app, request).await
}

async fn dispatch(app: &tauri::AppHandle, request: MainRequest) -> Result<Reply> {
    let core = app
        .try_state::<AppState>()
        .ok_or_else(|| os_error("Droplet is still starting."))?
        .core
        .clone();
    match request {
        MainRequest::View { query } => {
            let mut view = core.view(&query)?;
            view.start_at_login = app.autolaunch().is_enabled().map_err(os_error)?;
            Ok(Reply::View(view))
        }
        MainRequest::Connect { id } => {
            launch(app, Some(&id)).await?;
            Ok(Reply::Done)
        }
        MainRequest::HideLauncher => {
            if let Some(window) = app.get_webview_window("main") {
                window.hide().map_err(os_error)?;
            }
            Ok(Reply::Done)
        }
        MainRequest::StartAtLogin { enabled } => {
            if enabled {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            }
            .map_err(os_error)?;
            let _ = app.emit("state-changed", ());
            Ok(Reply::Done)
        }
        MainRequest::ResetPet => {
            position_pet(app, None)?;
            core.save_position(None)?;
            let config = core.config()?;
            preferences(
                app,
                Preferences {
                    pet_visible: true,
                    reduce_motion: config.reduce_motion,
                    terminal_shell: config.terminal_shell,
                    pet: config.pet,
                },
            )
            .await
        }
        MainRequest::Preferences { preferences: value } => preferences(app, value).await,
        request => core.execute(request, true).await,
    }
}
async fn preferences(app: &tauri::AppHandle, value: Preferences) -> Result<Reply> {
    let core = app
        .try_state::<AppState>()
        .ok_or_else(|| os_error("Droplet is still starting."))?
        .core
        .clone();
    let previous = core.config()?;
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| os_error("The pet window is unavailable."))?;
    if value.pet_visible {
        pet.show()
    } else {
        pet.hide()
    }
    .map_err(os_error)?;
    let result = core
        .execute(MainRequest::Preferences { preferences: value }, true)
        .await;
    if result.is_err() {
        let _ = if previous.pet_visible {
            pet.show()
        } else {
            pet.hide()
        };
    }
    result
}
async fn launch(app: &tauri::AppHandle, id: Option<&str>) -> Result<()> {
    let core = app
        .try_state::<AppState>()
        .ok_or_else(|| os_error("Droplet is still starting."))?
        .core
        .clone();
    let lease = core.begin_launch(id)?;
    let result = terminal::launch(&lease.spec, lease.terminal_shell.clone()).await;
    lease.finish(result)
}
fn connect_favorite(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = launch(&app, None).await {
            report_error(&app, error);
        }
    });
}
fn report_error(app: &tauri::AppHandle, error: AppError) {
    let _ = show_launcher(app.clone());
    let _ = app.emit_to("main", "app-error", error);
}
fn show_launcher(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.unminimize().map_err(os_error)?;
        window.show().map_err(os_error)?;
        window.set_focus().map_err(os_error)?;
    }
    Ok(())
}

#[tauri::command]
async fn pet_command(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    request: PetRequest,
) -> Result<PetReply> {
    security::authorize(window.label(), "pet")?;
    let core = app
        .try_state::<AppState>()
        .ok_or_else(|| os_error("Droplet is still starting."))?
        .core
        .clone();
    match request {
        PetRequest::View => return core.pet().map(PetReply::View),
        PetRequest::OpenLauncher => show_launcher(app.clone())?,
        PetRequest::ConnectFavorite => {
            if let Err(error) = launch(&app, None).await {
                report_error(&app, error.clone());
                return Err(error);
            }
        }
        PetRequest::BeginDrag { x, y } => {
            check_coordinates(x, y)?;
            let state = app
                .try_state::<AppState>()
                .ok_or_else(|| os_error("Droplet is still starting."))?;
            *state.drag.lock().map_err(os_error)? = Some(Drag {
                x,
                y,
                origin: window.outer_position().map_err(os_error)?,
                scale: window.scale_factor().map_err(os_error)?,
                started: Instant::now(),
            });
        }
        PetRequest::Drag { x, y } => {
            check_coordinates(x, y)?;
            let state = app
                .try_state::<AppState>()
                .ok_or_else(|| os_error("Droplet is still starting."))?;
            let drag = state.drag.lock().map_err(os_error)?;
            let drag = drag
                .as_ref()
                .filter(|d| d.started.elapsed() < Duration::from_secs(120))
                .ok_or_else(|| {
                    AppError::new(ErrorCode::Forbidden, "Start dragging the pet first.")
                })?;
            let desired = PetPosition {
                x: (f64::from(drag.origin.x) + (x - drag.x) * drag.scale).round() as i32,
                y: (f64::from(drag.origin.y) + (y - drag.y) * drag.scale).round() as i32,
            };
            let position = clamp_position(&window, desired)?;
            window
                .set_position(PhysicalPosition::new(position.x, position.y))
                .map_err(os_error)?;
        }
        PetRequest::EndDrag => {
            let state = app
                .try_state::<AppState>()
                .ok_or_else(|| os_error("Droplet is still starting."))?;
            let had_drag = state.drag.lock().map_err(os_error)?.take().is_some();
            if had_drag {
                let p = window.outer_position().map_err(os_error)?;
                core.save_position(Some(PetPosition { x: p.x, y: p.y }))?;
            }
        }
    }
    Ok(PetReply::Done)
}
fn check_coordinates(x: f64, y: f64) -> Result<()> {
    if !x.is_finite() || !y.is_finite() || x.abs() > 100_000.0 || y.abs() > 100_000.0 {
        Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid pointer coordinates.",
        ))
    } else {
        Ok(())
    }
}
fn clamp_position(window: &tauri::WebviewWindow, desired: PetPosition) -> Result<PetPosition> {
    let size = window.outer_size().map_err(os_error)?;
    let monitors = window.available_monitors().map_err(os_error)?;
    monitors
        .iter()
        .map(|m| {
            let area = m.work_area();
            let min_x = area.position.x;
            let min_y = area.position.y;
            let max_x = (i64::from(min_x) + i64::from(area.size.width) - i64::from(size.width))
                .max(i64::from(min_x)) as i32;
            let max_y = (i64::from(min_y) + i64::from(area.size.height) - i64::from(size.height))
                .max(i64::from(min_y)) as i32;
            PetPosition {
                x: desired.x.clamp(min_x, max_x),
                y: desired.y.clamp(min_y, max_y),
            }
        })
        .min_by_key(|p| {
            (i64::from(p.x) - i64::from(desired.x)).pow(2)
                + (i64::from(p.y) - i64::from(desired.y)).pow(2)
        })
        .ok_or_else(|| os_error("No display is available for the pet."))
}
fn position_pet(app: &tauri::AppHandle, saved: Option<PetPosition>) -> Result<()> {
    let window = app
        .get_webview_window("pet")
        .ok_or_else(|| AppError::new(ErrorCode::Process, "The pet window is unavailable."))?;
    let monitors = window.available_monitors().map_err(os_error)?;
    let size = window.outer_size().map_err(os_error)?;
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
    } else if let Some(monitor) = window.primary_monitor().map_err(os_error)? {
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
    window.set_position(position).map_err(os_error)
}

fn make_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let config = read_config(app).unwrap_or_default();
    let view = app
        .try_state::<AppState>()
        .and_then(|state| state.core.pet().ok());
    let name = config
        .connections
        .iter()
        .find(|c| Some(&c.id) == config.favorite_id.as_ref())
        .map(|c| c.name.as_str());
    let connect = MenuItem::with_id(
        app,
        "connect-favorite",
        name.map(|n| format!("Connect to {n}"))
            .unwrap_or("Add your first connection…".into()),
        view.is_some_and(|v| v.can_connect),
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "open", "Open Droplet…", true, None::<&str>)?;
    let lock = MenuItem::with_id(
        app,
        "toggle-lock",
        if config.launch_locked {
            "Resume SSH launches"
        } else {
            "Pause SSH launches"
        },
        true,
        None::<&str>,
    )?;
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
    Menu::with_items(
        app,
        &[
            &connect,
            &open,
            &lock,
            &PredefinedMenuItem::separator(app)?,
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
fn tray_action(app: &tauri::AppHandle, action: &str) {
    match action {
        "connect-favorite" => {
            connect_favorite(app);
            return;
        }
        "open" => {
            let _ = show_launcher(app.clone());
            return;
        }
        "quit" => {
            app.exit(0);
            return;
        }
        _ => {}
    }
    let request = match read_config(app) {
        Ok(c) => match action {
            "toggle-lock" => MainRequest::LaunchLock {
                locked: !c.launch_locked,
            },
            "toggle-pet" => MainRequest::Preferences {
                preferences: Preferences {
                    pet_visible: !c.pet_visible,
                    reduce_motion: c.reduce_motion,
                    terminal_shell: c.terminal_shell,
                    pet: c.pet,
                },
            },
            "reset-pet" => MainRequest::ResetPet,
            _ => return,
        },
        Err(error) => {
            report_error(app, error);
            return;
        }
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = dispatch(&app, request).await {
            report_error(&app, error);
        }
    });
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = show_launcher(app.clone());
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("local-navigation")
                .on_navigation(|_, url| {
                    (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                        || (matches!(url.scheme(), "http" | "https")
                            && url.host_str() == Some("tauri.localhost"))
                        || (cfg!(debug_assertions)
                            && url.scheme() == "http"
                            && url.host_str() == Some("127.0.0.1")
                            && url.port() == Some(1420))
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![main_command, pet_command])
        .setup(|app| {
            let core = Core::open(app.path().app_config_dir()?, app.path().home_dir()?);
            let config = core.config()?;
            app.manage(AppState {
                core: core.clone(),
                drag: Mutex::new(None),
            });
            let handle = app.handle().clone();
            let notify_handle = handle.clone();
            core.set_notifier(move || {
                let _ = notify_handle.emit("state-changed", ());
                let h = notify_handle.clone();
                let _ = notify_handle.run_on_main_thread(move || refresh_shortcuts(&h));
            });
            TrayIconBuilder::with_id("droplet")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray.png"
                ))?)
                .icon_as_template(true)
                .tooltip("Droplet · your SSH companion")
                .menu(&make_tray_menu(&handle)?)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| tray_action(app, event.id.as_ref()))
                .build(app)?;
            position_pet(&handle, config.pet_position)?;
            if config.pet_visible {
                if let Some(pet) = app.get_webview_window("pet") {
                    pet.show()?;
                }
            }
            #[cfg(target_os = "macos")]
            touchbar::install(&handle);
            if !std::env::args().any(|arg| arg == "--background") {
                show_launcher(handle.clone())?;
            }
            // Renderers can request their view before setup has registered AppState.
            // Publish readiness after subscribing renderers can safely read the core.
            handle.emit("state-changed", ())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "main" {
                    let _ = window.hide();
                } else {
                    let app = window.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        if let Ok(c) = read_config(&app) {
                            if let Err(error) = preferences(
                                &app,
                                Preferences {
                                    pet_visible: false,
                                    reduce_motion: c.reduce_motion,
                                    terminal_shell: c.terminal_shell,
                                    pet: c.pet,
                                },
                            )
                            .await
                            {
                                report_error(&app, error);
                            }
                        }
                    });
                }
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

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_pointer_coordinates_are_rejected() {
        assert!(super::check_coordinates(f64::NAN, 0.0).is_err());
        assert!(super::check_coordinates(f64::INFINITY, 0.0).is_err());
        assert!(super::check_coordinates(0.0, 100_001.0).is_err());
        assert!(super::check_coordinates(-1200.0, 400.0).is_ok());
    }
}
