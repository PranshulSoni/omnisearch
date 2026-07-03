pub fn log(msg: &str) {
    // Basic diagnostic logging to app_log.txt beside the exe; falls back to
    // %APPDATA%\omnisearch when the exe dir isn't writable (Program Files),
    // so diagnostics aren't silently lost on installed copies.
    let exe_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("app_log.txt")));
    let appdata_path = std::env::var("APPDATA").ok().map(|a| {
        std::path::PathBuf::from(a)
            .join("omnisearch")
            .join("app_log.txt")
    });

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    for path in [exe_path, appdata_path].into_iter().flatten() {
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > 1024 * 1024 {
                let _ = std::fs::remove_file(&path);
            }
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            use std::io::Write;
            if writeln!(file, "[{}] {}", now_ms, msg).is_ok() {
                return;
            }
        }
        // Open/write failed — try the fallback location.
    }
}
