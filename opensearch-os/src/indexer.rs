use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::collections::HashMap;
use walkdir::WalkDir;
use rusqlite::{Connection, params};

pub fn start_indexer(db_path: PathBuf) {
    let db_path_clone = db_path.clone();
    thread::spawn(move || {
        // Initialize COM for WinRT OCR
        let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED
        ) };

        // Set low priority so indexing never slows down foreground apps
        unsafe {
            use windows::Win32::System::Threading::{SetThreadPriority, GetCurrentThread, THREAD_PRIORITY_BELOW_NORMAL};
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
        }

        // ── Phase 1: Priority folders (Desktop, Documents, Downloads) ──────
        // Indexed within ~1 second of launch so common files are instantly searchable.
        thread::sleep(std::time::Duration::from_millis(500));
        if let Err(e) = run_indexer_folders(&db_path_clone, get_priority_folders()) {
            eprintln!("Priority indexer error: {:?}", e);
        }

        // ── Phase 2: Full crawl (entire user profile + other drives) ───────
        // Runs 10s after launch, then every 10 minutes.
        thread::sleep(std::time::Duration::from_secs(10));
        loop {
            if let Err(e) = run_indexer_folders(&db_path_clone, get_scan_folders()) {
                eprintln!("Indexer error: {:?}", e);
            }
            thread::sleep(std::time::Duration::from_secs(600));
        }
    });
}

/// Returns Desktop, Documents, Downloads — fast to scan, highest value to user.
fn get_priority_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    unsafe {
        use windows::Win32::UI::Shell::{
            SHGetKnownFolderPath, KF_FLAG_DEFAULT,
            FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Pictures,
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

        for guid in [&FOLDERID_Desktop, &FOLDERID_Documents, &FOLDERID_Downloads, &FOLDERID_Pictures] {
            if let Some(p) = get_folder(guid) {
                folders.push(p);
            }
        }
    }
    folders
}

fn is_ignored_dir(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    if name_lower.starts_with('$') {
        return true;
    }
    match name_lower.as_str() {
        "node_modules" | "target" | "build" | "dist" | "venv" | ".venv" | ".git" |
        "appdata" | "obj" | "bin" | "out" | ".next" | ".nuxt" | ".cache" | "cache" |
        ".cargo" | ".rustup" | ".npm" | ".m2" | ".nuget" | "vendor" |
        "cmake-build-debug" | "cmake-build-release" | ".yarn" | "__pycache__" |
        ".idea" | ".vscode" | ".gradle" | ".metadata" | "system volume information" |
        "temp" | "tmp" => true,
        _ => false,
    }
}

fn is_ignored_file(name: &str, ext: &str) -> bool {
    if name.starts_with("~$") {
        return true;
    }
    match ext {
        "tmp" | "temp" | "log" | "pdb" | "obj" | "o" | "class" | "db-wal" | "db-shm" => true,
        _ => false,
    }
}

struct PendingUpdate {
    path: String,
    name: String,
    extension: String,
    modified: i64,
    size: i64,
    is_dir: i64,
    content: Option<String>,
}

fn flush_updates(conn: &mut Connection, updates: &mut Vec<PendingUpdate>) -> anyhow::Result<()> {
    if updates.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    {
        let mut insert_file_stmt = tx.prepare(
            "INSERT OR REPLACE INTO files (path, name, extension, modified, size, is_dir) VALUES (?, ?, ?, ?, ?, ?)"
        )?;
        let mut delete_fts_stmt = tx.prepare(
            "DELETE FROM files_fts WHERE path = ?"
        )?;
        let mut insert_fts_stmt = tx.prepare(
            "INSERT INTO files_fts (path, content) VALUES (?, ?)"
        )?;

        for update in updates.drain(..) {
            // Clone path before moving into params! so FTS statements can use it afterwards
            let path_clone = update.path.clone();
            insert_file_stmt.execute(params![
                update.path,
                update.name,
                update.extension,
                update.modified,
                update.size,
                update.is_dir
            ])?;

            if let Some(content) = update.content {
                delete_fts_stmt.execute([&path_clone])?;
                insert_fts_stmt.execute(params![path_clone, content])?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

fn run_indexer_folders(db_path: &Path, folders: Vec<PathBuf>) -> anyhow::Result<()> {
    let mut conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            extension TEXT NOT NULL,
            modified INTEGER NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            is_dir INTEGER NOT NULL DEFAULT 0
        );",
        [],
    )?;

    // Migrate existing databases that may lack the new columns
    let _ = conn.execute("ALTER TABLE files ADD COLUMN size INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE files ADD COLUMN is_dir INTEGER NOT NULL DEFAULT 0", []);
    
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            path UNINDEXED,
            content
        );",
        [],
    )?;

    let mut seen_paths = std::collections::HashSet::new();

    // Cache existing database file paths and modified times in memory to avoid query overhead
    let mut db_files = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT path, modified FROM files")?;
        let db_files_iter = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for item in db_files_iter {
            if let Ok((p, m)) = item {
                db_files.insert(p, m);
            }
        }
    }

    // Cache existing FTS5 indexed paths
    let mut fts_paths = std::collections::HashSet::new();
    {
        let mut stmt = conn.prepare("SELECT path FROM files_fts")?;
        let fts_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for item in fts_iter {
            if let Ok(p) = item {
                fts_paths.insert(p);
            }
        }
    }

    let mut file_count = 0;
    let mut pending_updates = Vec::new();

    for folder in folders {
        if !folder.exists() { continue; }
        let walker = WalkDir::new(folder)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !is_ignored_dir(&name)
            });
            
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            let is_file = path.is_file();
            let is_dir = path.is_dir();
            if !is_file && !is_dir { continue; }
            
            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            let ext = if is_dir {
                "folder".to_string()
            } else {
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase()
            };

            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let name = if name.is_empty() {
                path_str.clone()
            } else {
                name
            };

            if is_file && is_ignored_file(&name, &ext) { continue; }

            seen_paths.insert(path_str.clone());

            let metadata = entry.metadata().ok();
            let modified = metadata.as_ref()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let file_size = metadata.as_ref().map(|m| m.len() as i64).unwrap_or(0);

            let db_modified = db_files.get(&path_str).copied();
            
            let text_extensions = [
                "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "json", "html", "css",
                "c", "cpp", "h", "hpp", "cs", "go", "java", "kt", "sh", "bat",
                "ps1", "yaml", "yml", "toml", "ini", "sql", "xml",
                "rb", "php", "lua", "swift", "dart", "vue", "svelte", "csv",
                "tex", "rst", "adoc", "conf", "env"
            ];
            let image_extensions = ["png", "jpg", "jpeg", "bmp", "gif"];

            let is_text_or_doc = is_file && (text_extensions.contains(&ext.as_str()) || ext == "pdf" || ext == "docx");
            let is_image = is_file && image_extensions.contains(&ext.as_str());
            let should_fts = is_text_or_doc || is_image;
            let needs_fts_check = should_fts && !fts_paths.contains(&path_str);

            if db_modified.is_none() || db_modified.unwrap() != modified || needs_fts_check {
                let mut content = None;
                if is_file && should_fts {
                    if is_text_or_doc {
                        let is_pdf = ext == "pdf";
                        let is_docx = ext == "docx";

                        let extracted = if is_pdf {
                            match pdf_extract::extract_text(path) {
                                Ok(text) => {
                                    let mut truncated = text;
                                    truncated.truncate(50 * 1024);
                                    Some(truncated)
                                }
                                Err(e) => {
                                    eprintln!("PDF extract failed for {:?}: {:?}", path, e);
                                    None
                                }
                            }
                        } else if is_docx {
                            match docx_lite::extract_text(path) {
                                Ok(text) => {
                                    let mut truncated = text;
                                    truncated.truncate(50 * 1024);
                                    Some(truncated)
                                }
                                Err(e) => {
                                    eprintln!("DOCX extract failed for {:?}: {:?}", path, e);
                                    None
                                }
                            }
                        } else {
                            read_text_file(path).ok()
                        };

                        content = Some(extracted.unwrap_or_default());

                        if is_pdf || is_docx {
                            thread::sleep(std::time::Duration::from_millis(50));
                        }
                    } else if is_image {
                        let extracted = extract_ocr_text(path);
                        content = Some(extracted.unwrap_or_default());
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                }

                pending_updates.push(PendingUpdate {
                    path: path_str,
                    name,
                    extension: ext,
                    modified,
                    size: file_size,
                    is_dir: if is_dir { 1 } else { 0 },
                    content,
                });

                if pending_updates.len() >= 1000 {
                    flush_updates(&mut conn, &mut pending_updates)?;
                }
            }

            // Yield CPU cycles after scanning every 1000 files
            file_count += 1;
            if file_count % 1000 == 0 {
                thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }

    flush_updates(&mut conn, &mut pending_updates)?;

    // Clean up deleted files from the database in a single transaction
    let mut to_delete = Vec::new();
    for p_str in db_files.keys() {
        if !seen_paths.contains(p_str) {
            to_delete.push(p_str);
        }
    }

    if !to_delete.is_empty() {
        let tx = conn.transaction()?;
        for p_str in to_delete {
            tx.execute("DELETE FROM files WHERE path = ?", [&p_str])?;
            tx.execute("DELETE FROM files_fts WHERE path = ?", [&p_str])?;
        }
        tx.commit()?;
    }

    Ok(())
}

pub fn get_scan_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    
    let system_drive = std::env::var("SystemDrive")
        .unwrap_or_else(|_| "C:".to_string())
        .to_uppercase();

    // 1. Get the User Profile folder
    unsafe {
        use windows::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_Profile, KF_FLAG_DEFAULT};
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

        if let Some(p) = get_folder(&FOLDERID_Profile) {
            folders.push(p);
        }
    }

    if folders.is_empty() {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            folders.push(PathBuf::from(profile));
        }
    }

    // 2. Discover all other fixed drives and scan them from their roots
    for c in b'A'..=b'Z' {
        let drive_letter = c as char;
        let drive_path_str = format!("{}:\\", drive_letter);
        if drive_path_str.to_uppercase().starts_with(&system_drive) {
            continue;
        }
        let wide_path: Vec<u16> = drive_path_str.encode_utf16().chain(Some(0)).collect();
        unsafe {
            use windows::Win32::Storage::FileSystem::GetDriveTypeW;
            let drive_type = GetDriveTypeW(windows::core::PCWSTR(wide_path.as_ptr()));
            if drive_type == 3 { // 3 corresponds to DRIVE_FIXED in Win32
                folders.push(PathBuf::from(drive_path_str));
            }
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

fn extract_ocr_text(path: &Path) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Storage::StorageFile;
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;

    let path_str = path.to_str()?;
    let path_wide = HSTRING::from(path_str);
    
    let file = match StorageFile::GetFileFromPathAsync(&path_wide).ok()?.get() {
        Ok(f) => f,
        Err(_) => return None,
    };
    
    let stream = match file.OpenAsync(windows::Storage::FileAccessMode::Read).ok()?.get() {
        Ok(s) => s,
        Err(_) => return None,
    };
    
    let decoder = match BitmapDecoder::CreateAsync(&stream).ok()?.get() {
        Ok(d) => d,
        Err(_) => return None,
    };
    
    let software_bitmap = match decoder.GetSoftwareBitmapAsync().ok()?.get() {
        Ok(b) => b,
        Err(_) => return None,
    };
    
    let ocr_engine = match OcrEngine::TryCreateFromUserProfileLanguages() {
        Ok(engine) => engine,
        Err(_) => return None,
    };
    
    let ocr_result = match ocr_engine.RecognizeAsync(&software_bitmap).ok()?.get() {
        Ok(res) => res,
        Err(_) => return None,
    };
    
    let text = match ocr_result.Text() {
        Ok(t) => t.to_string(),
        Err(_) => return None,
    };
    
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
