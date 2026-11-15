//! Zen — Tauri v2 shell (shared by the desktop app and the Android app).
//!
//! Two modes, chosen at build time by the `ZEN_BACKEND_URL` env var:
//! - Pre-load build (Nov 14, no backend URL set): the window shows the
//!   bundled `preload.html` countdown to the Beta (Nov 20, 2026, midnight
//!   America/Halifax). No backend is needed, so the app works offline.
//! - Beta build (Nov 20, `ZEN_BACKEND_URL` set to the hosted Zen server):
//!   the window loads the hosted Zen app, so the prebuilt frontend's
//!   relative `./actions` calls resolve to the backend origin. The bundled
//!   frontend (`src/`, the Zen UI snapshot) is the static source the server
//!   serves, and the fallback if the backend is unreachable.
//!
//! The auto-updater is how a pre-load install becomes the Beta: the Beta
//! release is published to GitHub Releases, the pre-load app checks on
//! startup, downloads it, installs it, and restarts into the Beta build.
//! The updater and the system tray are desktop-only; on Android the app
//! updates by installing a new APK.
//!
//! API notes (Tauri v2, verified against the docs Nov 13, 2026):
//! - Updater: `app.updater()?.check().await?` returns `Option<Update>`;
//!   `update.download_and_install(on_chunk, on_finish)` takes a chunk
//!   callback `(chunk_length, content_length)` and a no-arg finish
//!   callback. Restart with `app.restart()` — an inherent `AppHandle`
//!   method in Tauri v2 core (no import needed).
//! - Tray: needs the `tray-icon` feature on the `tauri` crate.
//!   Menu items use `MenuItem::with_id` (stable ids) + `Menu::with_items`,
//!   clicks are handled in `on_menu_event` via `event.id`.

use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg(desktop)]
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
#[cfg(desktop)]
use tauri_plugin_notification::NotificationExt;
#[cfg(desktop)]
use tauri_plugin_updater::UpdaterExt;

/// Backend URL baked in at build time (CI sets `ZEN_BACKEND_URL`).
/// Empty / unset => pre-load mode (bundled countdown, no backend needed).
const BACKEND_URL: Option<&'static str> = option_env!("ZEN_BACKEND_URL");
/// Bundled pre-load page (countdown to the Beta). Lives in `src/`.
const PRELOAD_PAGE: &str = "preload.html";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_notification::init());

    // Desktop-only plugins: the auto-updater and the process plugin have
    // no mobile support, and the tray needs the desktop `tray-icon` feature.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .setup(|app| {
            // Main window: hosted Zen app when a backend URL was baked in,
            // otherwise the bundled pre-load countdown page.
            let window = WebviewWindowBuilder::new(app, "main", start_url())
                .title("Zen")
                .inner_size(1280.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .resizable(true)
                .build()?;

            #[cfg(desktop)]
            {
                setup_tray(app)?;

                // Check for updates on startup, in the background. Failures are
                // silent here (no backend / no release yet just means "no update").
                let startup_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = check_for_updates_inner(&startup_handle).await;
                });
            }

            let _ = window;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_backend_url,
            check_for_updates,
            check_for_update_available
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zen");
}

/// System tray: Show / Check for Updates / Quit. Desktop only.
#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Zen", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check-updates", "Check for Updates", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Zen", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &check, &quit])?;
    let _tray = TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .expect("Zen window icon is missing")
                .clone(),
        )
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "check-updates" => {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = check_for_updates_inner(&handle).await;
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// Where the main window starts: the hosted Zen app when a backend URL was
/// baked in at build time, otherwise the bundled pre-load countdown page.
fn start_url() -> WebviewUrl {
    if let Some(raw) = BACKEND_URL {
        if !raw.is_empty() {
            if let Ok(url) = raw.parse::<tauri::Url>() {
                return WebviewUrl::External(url);
            }
        }
    }
    WebviewUrl::App(PRELOAD_PAGE.into())
}

/// Backend URL baked into this build (if any), for the frontend to read.
#[tauri::command]
fn get_backend_url() -> Option<String> {
    BACKEND_URL
        .filter(|u| !u.is_empty())
        .map(|u| u.to_string())
}

/// Check whether an update is available, WITHOUT downloading it.
/// Returns the new version string if one is available, else None.
/// Used by the pre-load page to show/hide the update button.
#[tauri::command]
async fn check_for_update_available(app: tauri::AppHandle) -> Result<Option<String>, String> {
    check_for_update_available_inner(&app).await
}

#[cfg(desktop)]
async fn check_for_update_available_inner(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(update.map(|u| u.version.to_string()))
}

/// On Android there is no in-app updater; a new APK is the update.
#[cfg(not(desktop))]
async fn check_for_update_available_inner(
    _app: &tauri::AppHandle,
) -> Result<Option<String>, String> {
    Ok(None)
}

/// Check for an update (called from the tray menu, or by the frontend).
/// If one is available it is downloaded, installed, and the app restarts
/// into it. Returns a short status string; errors are returned as Err.
#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<String, String> {
    check_for_updates_inner(&app).await
}

#[cfg(desktop)]
async fn check_for_updates_inner(app: &tauri::AppHandle) -> Result<String, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    match update {
        Some(update) => {
            let _ = app
                .notification()
                .builder()
                .title("Zen update available")
                .body(format!("Zen {} is downloading.", update.version))
                .show();
            update
                .download_and_install(|_chunk, _total| {}, || {})
                .await
                .map_err(|e| e.to_string())?;
            // Install finished — restart into the new version.
            // `restart()` never returns, so this arm coerces to the
            // match's `Result<String, String>` type.
            app.restart()
        }
        None => Ok("Zen is up to date.".to_string()),
    }
}

/// On Android there is no in-app updater; a new APK is the update.
#[cfg(not(desktop))]
async fn check_for_updates_inner(_app: &tauri::AppHandle) -> Result<String, String> {
    Ok("Updates on Android come from a new APK.".to_string())
}
