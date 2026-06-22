use std::path::{Path, PathBuf};
use std::thread;
use std::process::Command;
use walkdir::WalkDir;
use rusqlite::{Connection, params};
use crate::indexer::get_scan_folders;

pub fn start_git_indexer(db_path: PathBuf) {
    thread::spawn(move || {
        // Initial delay to let the app start up completely lag-free
        thread::sleep(std::time::Duration::from_secs(15));
        loop {
            if let Err(e) = run_git_indexer(&db_path) {
                eprintln!("Git Indexer error: {:?}", e);
            }
            // Re-scan every 15 minutes
            thread::sleep(std::time::Duration::from_secs(900));
        }
    });
}

fn run_git_indexer(db_path: &Path) -> anyhow::Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_repos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT UNIQUE,
            name TEXT NOT NULL
        );",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_commits (
            hash TEXT PRIMARY KEY,
            repo_id INTEGER,
            author TEXT NOT NULL,
            date INTEGER NOT NULL,
            message TEXT NOT NULL,
            FOREIGN KEY(repo_id) REFERENCES git_repos(id) ON DELETE CASCADE
        );",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_branches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id INTEGER,
            name TEXT NOT NULL,
            is_head INTEGER NOT NULL,
            UNIQUE(repo_id, name),
            FOREIGN KEY(repo_id) REFERENCES git_repos(id) ON DELETE CASCADE
        );",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS git_todos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id INTEGER,
            file_path TEXT NOT NULL,
            line_number INTEGER NOT NULL,
            todo_text TEXT NOT NULL,
            FOREIGN KEY(repo_id) REFERENCES git_repos(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Step 1: Find repositories
    let folders = get_scan_folders();
    let mut found_repos = Vec::new();

    for folder in folders {
        if !folder.exists() { continue; }
        
        let walker = WalkDir::new(folder)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                name != "node_modules" 
                    && name != "target" 
                    && name != "build" 
                    && name != "dist" 
                    && name != "venv" 
                    && name != ".venv"
                    && name != ".git"
                    && name != "appdata"
            });

        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if path.join(".git").exists() {
                    found_repos.push(path.to_path_buf());
                }
            }
            // Yield to avoid pegging CPU
            thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    // Step 2: Index repositories
    let mut active_repo_ids = Vec::new();
    for repo_path in found_repos {
        let repo_path_str = match repo_path.to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        let repo_name = repo_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Repo")
            .to_string();

        let _ = conn.execute(
            "INSERT OR IGNORE INTO git_repos (path, name) VALUES (?, ?)",
            params![repo_path_str, repo_name],
        );

        let repo_id: Option<i64> = conn.query_row(
            "SELECT id FROM git_repos WHERE path = ?",
            [&repo_path_str],
            |row| row.get(0),
        ).ok();

        if let Some(r_id) = repo_id {
            active_repo_ids.push(r_id);
            if let Err(e) = index_single_repo(&conn, r_id, &repo_path) {
                eprintln!("Error indexing repo {:?}: {:?}", repo_path, e);
            }
        }
    }

    // Step 3: Delete repos that no longer exist
    let mut stmt = conn.prepare("SELECT id, path FROM git_repos")?;
    let db_repos = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
    for repo in db_repos.flatten() {
        let (r_id, r_path) = repo;
        if !active_repo_ids.contains(&r_id) || !Path::new(&r_path).exists() {
            let _ = conn.execute("DELETE FROM git_repos WHERE id = ?", [r_id]);
        }
    }

    Ok(())
}

fn index_single_repo(conn: &Connection, repo_id: i64, repo_path: &Path) -> anyhow::Result<()> {
    // 1. Get branches
    let branch_output = Command::new("git")
        .args(["branch", "--no-color"])
        .current_dir(repo_path)
        .output();
    
    if let Ok(out) = branch_output {
        let text = String::from_utf8_lossy(&out.stdout);
        let _ = conn.execute("DELETE FROM git_branches WHERE repo_id = ?", [repo_id]);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            let (is_head, name) = if line.starts_with('*') {
                (1, line.strip_prefix('*').unwrap().trim())
            } else {
                (0, line)
            };
            let _ = conn.execute(
                "INSERT OR REPLACE INTO git_branches (repo_id, name, is_head) VALUES (?, ?, ?)",
                params![repo_id, name, is_head],
            );
        }
    }

    // 2. Get recent commits (up to 100)
    let log_output = Command::new("git")
        .args(["log", "--max-count=100", "--format=%H|%an|%at|%s"])
        .current_dir(repo_path)
        .output();

    if let Ok(out) = log_output {
        let text = String::from_utf8_lossy(&out.stdout);
        let _ = conn.execute("DELETE FROM git_commits WHERE repo_id = ?", [repo_id]);
        for line in text.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 4 { continue; }
            let hash = parts[0].to_string();
            let author = parts[1].to_string();
            let date = parts[2].parse::<i64>().unwrap_or(0);
            let message = parts[3].to_string();

            let _ = conn.execute(
                "INSERT OR REPLACE INTO git_commits (hash, repo_id, author, date, message) VALUES (?, ?, ?, ?, ?)",
                params![hash, repo_id, author, date, message],
            );
        }
    }

    // 3. Scan TODOs
    let _ = conn.execute("DELETE FROM git_todos WHERE repo_id = ?", [repo_id]);
    
    let walker = WalkDir::new(repo_path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            name != "node_modules" 
                && name != "target" 
                && name != "build" 
                && name != "dist" 
                && name != "venv" 
                && name != ".venv"
                && name != ".git"
        });

    let allowed_extensions = [
        "rs", "py", "js", "ts", "go", "cpp", "c", "h", "java", "kt", "cs", "md", "txt"
    ];

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() { continue; }
        
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if !allowed_extensions.contains(&ext.as_str()) { continue; }

        let path_str = match path.to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        if let Ok(content) = std::fs::read_to_string(path) {
            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.contains("TODO") || trimmed.contains("FIXME") || trimmed.contains("BUG") {
                    let line_number = (idx + 1) as i32;
                    let _ = conn.execute(
                        "INSERT INTO git_todos (repo_id, file_path, line_number, todo_text) VALUES (?, ?, ?, ?)",
                        params![repo_id, path_str, line_number, trimmed],
                    );
                }
            }
        }
        thread::sleep(std::time::Duration::from_millis(2));
    }

    Ok(())
}
