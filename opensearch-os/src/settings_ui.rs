slint::include_modules!();

use crate::settings::AppSettings;
use once_cell::sync::Lazy;
use slint::{CloseRequestResponse, ComponentHandle, SharedString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

// Signal channel: main thread sends () to ask settings window to show
static SHOW_REQUEST: Lazy<Mutex<Option<std::sync::mpsc::SyncSender<()>>>> =
    Lazy::new(|| Mutex::new(None));
// Track whether the settings thread is alive at all
static SETTINGS_READY: AtomicBool = AtomicBool::new(false);

pub fn init_settings_window(hwnd: HWND) {
    std::env::set_var("SLINT_STYLE", "fluent-dark");

    // Channel: main thread sends () → settings thread shows the window
    let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
    if let Ok(mut guard) = SHOW_REQUEST.lock() {
        *guard = Some(tx);
    }
    SETTINGS_READY.store(true, Ordering::SeqCst);

    // Wait for show requests in a loop. Each request creates a fresh window.
    loop {
        // Block until someone calls show_settings_window()
        match rx.recv() {
            Ok(_) => {}
            Err(_) => break, // channel closed, exit thread
        }

        // Create fresh Slint window each time (avoids all hide/show state issues)
        let ui = match SettingsWindow::new() {
            Ok(u) => u,
            Err(_) => continue,
        };

        // Load current settings
        let settings = AppSettings::load();
        let (api_key, endpoint, model, always_approve) = load_ai_settings();

        ui.set_run_on_startup(settings.run_on_startup);
        ui.set_hide_on_lose_focus(settings.hide_on_lose_focus);
        ui.set_theme_mode(SharedString::from(settings.normalized_theme_mode()));
        ui.set_global_hotkey(SharedString::from(settings.global_hotkey.clone()));
        ui.set_voice_hotkey(SharedString::from(crate::hotkey::VOICE_DICTATION_HOTKEY));
        ui.set_hotkey_error(SharedString::from(""));
        ui.set_window_width(settings.window_width as i32);
        ui.set_item_height(settings.item_height as i32);

        // Load Agent properties
        ui.set_agent_api_key(SharedString::from(api_key));
        ui.set_agent_endpoint(SharedString::from(endpoint));
        ui.set_agent_model(SharedString::from(model));
        ui.set_agent_always_approve(always_approve);

        // Close = hide window, then stop the inner run()
        let ui_weak_close = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(ui) = ui_weak_close.upgrade() {
                ui.invoke_set_hotkey_recording(false);
                // Quit the inner event loop to unblock us
                slint::quit_event_loop().ok();
                ui.window().hide().ok();
            }
            CloseRequestResponse::KeepWindowShown
        });

        // Save settings callback
        let ui_weak_save = ui.as_weak();
        ui.on_save_settings(move || {
            if let Some(ui) = ui_weak_save.upgrade() {
                let mut s = AppSettings::load();
                s.run_on_startup = ui.get_run_on_startup();
                s.hide_on_lose_focus = ui.get_hide_on_lose_focus();
                s.theme_mode = ui.get_theme_mode().to_string();
                let next_hotkey = ui.get_global_hotkey().to_string();
                if let Err(message) = crate::hotkey::validate_hotkey(&next_hotkey, &s.global_hotkey)
                {
                    ui.set_hotkey_error(SharedString::from(message));
                    ui.set_global_hotkey(SharedString::from(s.global_hotkey));
                    return;
                }
                s.global_hotkey = next_hotkey;
                s.window_width = ui.get_window_width() as u32;
                s.item_height = ui.get_item_height() as u32;
                s.save();
                ui.set_hotkey_error(SharedString::from(""));
                crate::settings_startup::set_run_on_startup(s.run_on_startup);

                // Save Agent properties
                save_ai_settings(
                    ui.get_agent_api_key().as_str(),
                    ui.get_agent_endpoint().as_str(),
                    ui.get_agent_model().as_str(),
                    ui.get_agent_always_approve(),
                );

                unsafe {
                    let _ = PostMessageW(
                        hwnd,
                        windows::Win32::UI::WindowsAndMessaging::WM_USER + 10,
                        windows::Win32::Foundation::WPARAM(0),
                        windows::Win32::Foundation::LPARAM(0),
                    );
                }
            }
        });

        ui.on_format_hotkey(move |key, ctrl, alt, shift, win| {
            let Some(hotkey) =
                crate::hotkey::format_recorded_hotkey(key.as_str(), ctrl, alt, shift, win)
            else {
                return SharedString::from("");
            };
            SharedString::from(hotkey)
        });

        ui.on_validate_hotkey(move |hotkey| {
            let settings = AppSettings::load();
            match crate::hotkey::validate_hotkey(hotkey.as_str(), &settings.global_hotkey) {
                Ok(()) => SharedString::from("OK"),
                Err(message) => SharedString::from(message),
            }
        });

        ui.on_set_hotkey_recording(move |recording| unsafe {
            let _ = PostMessageW(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::WM_USER + 11,
                windows::Win32::Foundation::WPARAM(recording as usize),
                windows::Win32::Foundation::LPARAM(0),
            );
        });

        // Show the window and run the event loop until it's closed
        ui.window().show().ok();
        ui.window().set_minimized(false);
        slint::run_event_loop().ok();
        // Window was closed — loop back and wait for next show request
    }
}

fn get_db_conn() -> Option<rusqlite::Connection> {
    let appdata = std::env::var("APPDATA").ok()?;
    let path = std::path::PathBuf::from(appdata)
        .join("opensearch-os")
        .join("file_index.db");
    let conn = rusqlite::Connection::open(&path).ok()?;
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    Some(conn)
}

fn load_ai_settings() -> (String, String, String, bool) {
    let mut api_key = String::new();
    let mut endpoint = String::new();
    let mut model = String::new();
    let mut always_approve = false;

    if let Some(conn) = get_db_conn() {
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_settings (key TEXT PRIMARY KEY, value TEXT);",
            [],
        );
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'api_key'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            api_key = val;
        }
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'endpoint'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            endpoint = val;
        }
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'model'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            model = val;
        }
        if let Ok(val) = conn.query_row(
            "SELECT value FROM ai_settings WHERE key = 'always_approve'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            always_approve = val.trim() == "1";
        }
    }
    (api_key, endpoint, model, always_approve)
}

fn save_ai_settings(api_key: &str, endpoint: &str, model: &str, always_approve: bool) {
    if let Some(conn) = get_db_conn() {
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_settings (key TEXT PRIMARY KEY, value TEXT);",
            [],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('api_key', ?);",
            [api_key],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('endpoint', ?);",
            [endpoint],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('model', ?);",
            [model],
        );
        let _ = conn.execute(
            "INSERT OR REPLACE INTO ai_settings (key, value) VALUES ('always_approve', ?);",
            [if always_approve { "1" } else { "0" }],
        );
    }
}


pub fn show_settings_window() {
    if !SETTINGS_READY.load(Ordering::SeqCst) {
        // Settings thread not yet ready — spawn it now lazily
        // (This path shouldn't normally be hit since init is called at startup)
        return;
    }
    if let Ok(guard) = SHOW_REQUEST.lock() {
        if let Some(tx) = guard.as_ref() {
            let _ = tx.try_send(());
        }
    }
}
