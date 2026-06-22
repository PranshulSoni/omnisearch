use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use walkdir::WalkDir;
use rusqlite::{Connection, params};

pub fn start_indexer(db_path: PathBuf) {
    thread::spawn(move || {
        // Initial delay to let the app start up completely lag-free
        thread::sleep(std::time::Duration::from_secs(5));
        loop {
            if let Err(e) = run_indexer(&db_path) {
                eprintln!("Indexer error: {:?}", e);
            }
            // Re-scan every 10 minutes
            thread::sleep(std::time::Duration::from_secs(600));
        }
    });
}

fn run_indexer(db_path: &Path) -> anyhow::Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            extension TEXT NOT NULL,
            modified INTEGER NOT NULL
        );",
        [],
    )?;
    
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            path UNINDEXED,
            content
        );",
        [],
    )?;

    let folders = get_scan_folders();
    let mut seen_paths = std::collections::HashSet::new();

    for folder in folders {
        if !folder.exists() { continue; }
        for entry in WalkDir::new(folder).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() { continue; }
            
            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let allowed = [
                "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt",
                "txt", "md", "rs", "py", "js", "ts", "json", "html", "css"
            ];
            if !allowed.contains(&ext.as_str()) { continue; }

            seen_paths.insert(path_str.clone());

            let modified = entry.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let db_modified: Option<i64> = conn.query_row(
                "SELECT modified FROM files WHERE path = ?",
                [&path_str],
                |row| row.get(0),
            ).ok();

            if db_modified.is_none() || db_modified.unwrap() != modified {
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                conn.execute(
                    "INSERT OR REPLACE INTO files (path, name, extension, modified) VALUES (?, ?, ?, ?)",
                    params![path_str, name, ext, modified],
                )?;

                let text_extensions = ["txt", "md", "rs", "py", "js", "ts", "json", "html", "css"];
                if text_extensions.contains(&ext.as_str()) {
                    if let Ok(content) = read_text_file(path) {
                        conn.execute("DELETE FROM files_fts WHERE path = ?", [&path_str])?;
                        conn.execute(
                            "INSERT INTO files_fts (path, content) VALUES (?, ?)",
                            params![path_str, content],
                        )?;
                    }
                }
            }

            // Throttling: sleep 5ms between files to keep CPU at ~0%
            thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    // Clean up deleted files from database
    let mut stmt = conn.prepare("SELECT path FROM files")?;
    let db_paths = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut to_delete = Vec::new();
    for p in db_paths {
        if let Ok(p_str) = p {
            if !seen_paths.contains(&p_str) {
                to_delete.push(p_str);
            }
        }
    }

    for p_str in to_delete {
        conn.execute("DELETE FROM files WHERE path = ?", [&p_str])?;
        conn.execute("DELETE FROM files_fts WHERE path = ?", [&p_str])?;
    }

    Ok(())
}

fn get_scan_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    unsafe {
        use windows::Win32::UI::Shell::{
            SHGetKnownFolderPath, FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, KF_FLAG_DEFAULT
        };
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Com::CoTaskMemFree;
        
        let get_folder = |guid| -> Option<PathBuf> {
            let result = SHGetKnownFolderPath(guid, KF_FLAG_DEFAULT, HANDLE::default()).ok()?;
            let mut len = 0;
            while *result.0.add(len) != 0 { len += 1; }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(result.0, len));
            CoTaskMemFree(Some(result.0 as *const _));
            Some(PathBuf::from(s))
        };

        if let Some(p) = get_folder(&FOLDERID_Desktop) { folders.push(p); }
        if let Some(p) = get_folder(&FOLDERID_Documents) { folders.push(p); }
        if let Some(p) = get_folder(&FOLDERID_Downloads) { folders.push(p); }
    }
    
    if folders.is_empty() {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(profile);
            folders.push(p.join("Desktop"));
            folders.push(p.join("Documents"));
            folders.push(p.join("Downloads"));
        }
    }
    
    folders
}

fn read_text_file(path: &Path) -> std::io::Result<String> {
    use std::fs::File;
    use std::io::Read;
    
    let mut file = File::open(path)?;
    let mut buf = vec![0u8; 50 * 1024]; // Limit to 50KB
    let n = file.read(&mut buf)?;
    buf.truncate(n);
    
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
