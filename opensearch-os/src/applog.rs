pub fn log(msg: &str) {
    // Basic diagnostic logging to app_log.txt
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("app_log.txt")))
        .unwrap_or_else(|| std::path::PathBuf::from("app_log.txt"));
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
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let _ = writeln!(file, "[{}] {}", now_ms, msg);
    }
}
