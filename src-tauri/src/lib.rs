use std::{sync::Arc, sync::Mutex};
use tauri::{Manager, RunEvent, State, WebviewWindow};
#[cfg(desktop)]
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_log::{Target, TargetKind, WEBVIEW_TARGET};

#[cfg(desktop)]
mod tray;
mod updater;

struct SplashscreenWindow(Arc<Mutex<WebviewWindow>>);
struct MainWindow(Arc<Mutex<WebviewWindow>>);

#[tauri::command]
fn close_splashscreen(
    _window: WebviewWindow,
    splashscreen: State<SplashscreenWindow>,
    main: State<MainWindow>,
) {
    #[cfg(desktop)]
    {
        let _ = splashscreen.0.lock().unwrap().close();
        let _ = main.0.lock().unwrap().show();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "debug");

    let context = tauri::generate_context!();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .clear_targets()
                .targets([
                    Target::new(TargetKind::Webview)
                        .filter(|m| m.target() == WEBVIEW_TARGET),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("rust".into()),
                    })
                    .filter(|m| m.target() != WEBVIEW_TARGET),
                ])
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            app.manage(SplashscreenWindow(Arc::new(Mutex::new(
                app.get_webview_window("splashscreen").unwrap(),
            ))));
            app.manage(MainWindow(Arc::new(Mutex::new(
                app.get_webview_window("main").unwrap(),
            ))));

            #[cfg(desktop)]
            {
                let handle = app.handle();

                handle.plugin(tauri_plugin_updater::Builder::new().build())?;

                handle.plugin(tauri_plugin_single_instance::init(|app, _, _| {
                    let _ = app
                        .notification()
                        .builder()
                        .title("This app is already running!")
                        .body("You can find it in the tray menu.")
                        .show();
                }))?;

                handle.plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    Some(vec![]),
                ))?;

                tray::create_tray(handle)?;
            }

            #[cfg(debug_assertions)]
            if let Some(w) = app.get_webview_window("main") {
                w.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            close_splashscreen,
            updater::check_for_updates,
            updater::download_update,
            updater::install_update,
            updater::clear_update_cache
        ])
        .build(context)
        .expect("error while running tauri application");

    #[cfg(desktop)]
    app.run(|app, event| match event {
        RunEvent::Ready => {}
        RunEvent::ExitRequested { api, code, .. } => {
            if code.is_none() {
                api.prevent_exit();
            }
        }
        RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            #[cfg(target_os = "macos")]
            {
                let _ = tauri::AppHandle::hide(&app.app_handle());
            }
            #[cfg(not(target_os = "macos"))]
            {
                if let Some(w) = app.get_webview_window(&label) {
                    let _ = w.hide();
                }
            }
            api.prevent_close();
        }
        _ => {}
    });

    #[cfg(mobile)]
    app.run(|_app, _event| {});
}
