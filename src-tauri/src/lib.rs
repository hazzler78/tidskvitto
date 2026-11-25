mod ai;
mod commands;
mod platform;
mod secrets;
mod tracker;

use commands::{
    ask, delete_everything, delete_measurements, get_settings, insights, receipt,
    request_screen_permission, save_settings, status,
};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent, WindowEvent};
use tidskvitto_core::Database;

pub struct AppState {
    pub db: Database,
    pub stop: AtomicBool,
    pub live_app: Mutex<String>,
    pub live_category: Mutex<String>,
    pub live_at: Mutex<i64>,
    pub idle_supported: AtomicBool,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .expect("sökväg till appdata");
            let db = Database::open(&dir.join("tidskvitto.db"))
                .expect("kunde inte öppna den lokala databasen");
            let state = Arc::new(AppState {
                db,
                stop: AtomicBool::new(false),
                live_app: Mutex::new(String::new()),
                live_category: Mutex::new(String::new()),
                live_at: Mutex::new(0),
                idle_supported: AtomicBool::new(false),
            });
            app.manage(Arc::clone(&state));
            install_tray(app.handle())?;
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(win) = handle.get_webview_window("main") {
                            let _ = win.hide();
                        }
                    }
                });
                if std::env::args().any(|arg| arg == "--autostart") {
                    let _ = window.hide();
                }
            }
            tracker::spawn(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            status,
            get_settings,
            receipt,
            insights,
            save_settings,
            delete_measurements,
            delete_everything,
            request_screen_permission,
            ask,
        ])
        .build(tauri::generate_context!())
        .expect("kunde inte starta Tidskvitto")
        .run(|app, event| {
            if matches!(event, RunEvent::Exit) {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    state.stop.store(true, Ordering::Relaxed);
                }
            }
        });
}

fn install_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Öppna Tidskvitto", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Pausa eller återuppta mätning", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Avsluta", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &quit])?;
    let mut builder = TrayIconBuilder::with_id("tray")
        .tooltip("Tidskvitto")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    state.stop.store(true, Ordering::Relaxed);
                }
                app.exit(0);
            }
            "toggle" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    let mut settings = state.db.load_settings();
                    settings.tracking_enabled = !settings.tracking_enabled;
                    settings.onboarded = true;
                    let _ = state.db.save_settings(&settings);
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
