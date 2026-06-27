# MemoryOS: The Memory Layer for Windows

**MemoryOS** is a premium Windows Intelligence Layer that continuously builds a searchable understanding of your digital life. 

While it exposes a high-performance, native Windows launcher UI (`opensearch-os`) as its primary entry point, the core mission of MemoryOS is simple: **Never lose anything on your computer again.**

It helps you find, understand, and continue anything you have seen, opened, copied, written, searched, or worked on.

---

## Core Vision & Product Positioning

MemoryOS is **not** positioned as a simple Raycast clone, a basic launcher, or a Windows Search replacement. The launcher is merely the interface to a persistent, searchable, and explainable computer memory layer.

### Core Promises
* **"My computer finally remembers everything for me."**
* **Search anything. Do anything. Continue everything.**

---

## Technical Architecture & Built Features

MemoryOS is powered by `opensearch-os`, a native Windows shell written in Rust utilizing the raw Win32 APIs and direct GDI graphics rendering for absolute responsiveness.

### 1. The Entry Point: Premium Launcher UI
* **Custom Win32 Window**: Tailored form factor (720px width, 76px row height) with clean `Segoe UI Variable` typography.
* **Opaque Backdrop**: Charcoal-colored window background (100% opacity, zero Acrylic blur overhead) for high-contrast visibility.
* **Modern Search Layout**: Visual representation of active filter pills, search statistics ("Results" & "Best matches first" sub-headers), accent borders, and vertical indicator bars.
* **Lag-Free Async Icons**: Spawns background worker threads to load app and file-type icons, passing them to the main thread via custom Windows message passing (`WM_ICON_LOADED`) to keep the interface completely smooth.
* **Watermark-Free Resolving**: Resolves `.lnk` shortcut targets using `IShellLinkW` and clean PIDLs to strip shortcut arrows.

### 2. Local File & Document Content Memory
* **Background Crawler Indexer**: Runs a throttled background indexer (`indexer.rs`) that indexes files in `Desktop`, `Documents`, and `Downloads`.
* **Deep Document Extraction**: Extracts and indexes text content from PDF (using `pdf-extract`) and Microsoft Word DOCX (using `docx-lite`) files, caching text up to 50KB to keep index databases lightweight.
* **Universal Code Indexing**: Monitors and indexes plain text and source code extensions (`.rs`, `.py`, `.js`, `.ts`, `.c`, `.cpp`, `.h`, `.hpp`, `.cs`, `.go`, `.java`, `.kt`, `.sh`, `.bat`, `.ps1`, `.yaml`, `.yml`, `.toml`, `.ini`, `.sql`, `.xml`).
* **SQLite FTS5**: Stores index text inside SQLite FTS5 (`files_fts` table) for sub-millisecond lexical full-text queries.

### 3. Multi-Browser Profiles Memory
* **Cross-Browser Bookmarks & History**: Scans Chromium profiles (Chrome, Edge, Brave) and Gecko profiles (Firefox). Pre-copies profile database locks before parsing to prevent browse conflict locks.
* **Places Integration**: Direct SQLite querying of Firefox's `places.sqlite` structure.

### 4. Developer & Git Repository Memory
* **Fast Git Scanner**: Walks scan folders metadata-first (ignoring `node_modules`, `target`, etc.) to locate Git repositories in under **186ms** without recursive loops.
* **Commits & Branches**: Directly queries the `HEAD` branch and the last 100 commits via Git CLI tools.
* **TODO / FIXME Tasks**: Scans codebase comments for task tags. Pressing `Enter` deep-links directly into VS Code at the exact file and line using `code -g <file>:<line>`.

### 5. In-Process Tools & Power Controls
* **Math Parser / Calculator**: High-speed, recursive descent math parser (evaluates formulas like `2+2`, `15% of 340`, `sqrt(9)*4`, etc.) and copies results to the clipboard.
* **Quick System Actions**: Lock, sleep, shutdown, restart, and recycle bin empty.
* **Recent Files Tracker**: Parses `%APPDATA%\Microsoft\Windows\Recent` to display recently used files with appropriate file-type icons.

---

## Search Prefixes & Scopes

To avoid database congestion, MemoryOS utilizes specific search prefixes:

> [!NOTE]
> Prefix-based queries bypass the mocked search results layout, querying the active indexing databases directly and displaying results in the original card/category layout.

| Category | Prefix | Empty State Placeholder | Badge | Description |
|---|---|---|---|---|
| **Bookmarks** | `bookmarks: <query>` | `📁 Browser Bookmarks` | `BOOKMARK` | Search browser favorites |
| **History** | `history: <query>` | `📁 Browser History` | `HISTORY` | Search browser history URLs |
| **Commits** | `commits: <query>` | `📁 Git Commits` | `COMMIT` | Search recent repository commits |
| **TODOs** | `todos: <query>` | `📁 Git TODOs` | `TODO` | Search code tasks / comments |
| **Local Files** | `file: <query>` | `📁 Local Files` | `FILE` | Search documents (PDF, DOCX, TXT) |
| **Source Code** | `code: <query>` | `📁 Source Code` | `CODE` | Search code files |

---

## Codebase Architecture

The project is structured under the `opensearch-os/` subdirectory:

* **`src/main.rs`**: Core window management, GDI-based double-buffered rendering, input handling, and event loop.
* **`src/indexer.rs`**: Handles background threads for crawling, Chromium/Firefox profile database extraction, and document content parsing (PDF/Word).
* **`src/search.rs`**: Core ranking, prefix matching logic, and SQLite database connector configuring WAL (Write-Ahead Logging) and thread concurrency settings.
* **`build.rs`**: Embeds icons, manifests, and compilation properties for the executable.

---

## Build & Run Instructions

### Prerequisites
* Rust compiler toolchain (Stable channel target `x86_64-pc-windows-msvc`).
* SQLite runtime dependencies.

### Development Build
To compile the launcher in debug mode:
```powershell
cargo build
```

### Production Build
To compile a fully optimized release target:
```powershell
cargo build --release
```

### Build Constraints & Clean Compilation
> [!IMPORTANT]
> If the launcher is running in the background, file locks will cause compile errors (`Access is denied / os error 5`). Always terminate any running instance of the application before building:

```powershell
taskkill /F /IM opensearch-os.exe
```