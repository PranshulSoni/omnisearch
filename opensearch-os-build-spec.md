# OpenSearch OS — App Build Spec

## What this is

A native Windows search tool. One global hotkey opens a search bar. The user
types a natural-language query. Results appear instantly showing matching
Windows settings with their location. The user hits Enter and the correct
settings dialog opens directly — scoped to the right tab/page, not just the
top-level control panel.

This document covers the full MVP build. Read every section before writing
any code.

---

## Companion documents

These already exist and must be read alongside this spec:

- `windows-settings-launcher-spec.md` — how to dispatch each `launch_command`
  format at runtime (four different shapes, different spawn strategies for each)
- `settings-catalog-csv-spec.md` — how the catalog CSV was built and what each
  field means
- `settings_catalog_master.csv` — 833-row catalog, the search index data source

---

## Architecture overview

```
settings_catalog_master.csv
        |
        | (build time, once)
        v
  embed_catalog.py  -->  catalog.bin  (833 x 384 float32 vectors + metadata)
                              |
                              | (shipped with app, loaded at startup)
                              v
                        opensearch-os.exe
                              |
                        [global hotkey]
                              |
                        [search input]
                              |
                        embed query (ONNX)
                              |
                        cosine similarity
                              |
                        top-5 results
                              |
                        [results list UI]
                              |
                        user selects result
                              |
                        launch_command dispatch
                              |
                        Windows opens the right settings page
```

---

## Tech stack

| Layer | Choice | Reason |
|---|---|---|
| Language | Rust | Required to hit <50ms search, <150MB RAM, ~0% idle CPU |
| UI | Win32 via `windows` crate | No Electron, no Tauri overhead — native window, minimal RAM |
| Embedding model | `bge-small-en-v1.5` (ONNX) | 33MB, best quality/size tradeoff for technical terminology |
| ONNX runtime | `ort` crate | Standard Rust ONNX Runtime bindings |
| Tokenizer | `tokenizers` crate (HuggingFace) | Pairs with bge-small |
| Vector math | `ndarray` crate | Cosine similarity over 833 vectors |
| Build-time embedding | Python + `sentence-transformers` | One-time script, not shipped |

Do NOT use Tauri. Do NOT use any web renderer. The UI is a native Win32 window.

---

## Performance targets

These are hard requirements, not aspirational:

| Metric | Target |
|---|---|
| Startup time | < 200ms |
| Search response (after first keystroke) | < 50ms |
| RAM usage (idle) | < 150MB |
| Idle CPU | ~0% |
| Binary + model size | < 60MB total |

---

## Step 1 — Build-time embedding pipeline (Python)

Write `embed_catalog.py`. Run this once to produce `catalog.bin`.

### What it does

1. Reads `settings_catalog_master.csv`
2. For each row, concatenates:
   `{control_name} {synonyms_pipe_to_space} {description} {breadcrumb_path}`
   (replace `|` in synonyms with spaces before concatenating)
3. Embeds all 833 strings using `bge-small-en-v1.5` via `sentence-transformers`
4. Writes `catalog.bin` — a binary file containing:
   - 4 bytes: row count (u32 little-endian)
   - 4 bytes: embedding dimension (u32 little-endian), should be 384
   - For each row:
     - 384 × 4 bytes: float32 embedding vector
     - 2 bytes: metadata length (u16 little-endian)
     - N bytes: UTF-8 JSON metadata string

### Metadata JSON per row (what gets stored alongside each vector)

```json
{
  "id": "mouse.pointer_options.enhance_pointer_precision",
  "control_name": "Enhance pointer precision",
  "breadcrumb_path": "Mouse Properties > Pointer Options > Motion > Enhance pointer precision",
  "launch_command": "control.exe main.cpl,,2",
  "description": "Improves cursor accuracy by adjusting pointer speed based on movement.",
  "source": "Native"
}
```

Only these 6 fields. Don't store the full CSV row — keep the binary small.

### Python dependencies

```
pip install sentence-transformers numpy
```

### Notes

- Normalize each vector to unit length after encoding (required for cosine
  similarity to work correctly with this model).
- Print progress every 100 rows — 833 embeddings takes ~30-60 seconds on CPU.
- Verify output: print the file size and first row's metadata after writing,
  so it's obvious if the format is wrong before handing it to Rust.

---

## Step 2 — Rust project structure

```
opensearch-os/
  Cargo.toml
  build.rs          (copies catalog.bin and model files to output dir)
  src/
    main.rs         (entry point, Win32 window setup, message loop)
    hotkey.rs       (global hotkey registration/handling)
    search.rs       (loads catalog.bin, embeds query, cosine similarity)
    launcher.rs     (dispatches launch_command — see launcher spec doc)
    ui.rs           (Win32 window, input, results rendering)
  assets/
    catalog.bin     (produced by embed_catalog.py)
    model/
      bge-small-en-v1.5/
        model.onnx
        tokenizer.json
        tokenizer_config.json
```

---

## Step 3 — Search module (`search.rs`)

### Startup (called once, at app launch)

1. Read `catalog.bin` from the assets directory embedded in the binary
   (use `include_bytes!` macro — this embeds catalog.bin directly into the
   executable so there's no separate file to manage at runtime).
2. Parse the binary format: read row count, dimension, then for each row
   read the float32 vector and the JSON metadata.
3. Store as two parallel Vecs: `Vec<Vec<f32>>` for vectors,
   `Vec<CatalogEntry>` for metadata.
4. Load the ONNX model and tokenizer from the assets directory.
5. Warm up the model with one dummy query so the first real search isn't slow.

### Per-query search (called on every keystroke after debounce)

```rust
pub struct CatalogEntry {
    pub id: String,
    pub control_name: String,
    pub breadcrumb_path: String,
    pub launch_command: String,
    pub description: String,
    pub source: String,
}

pub struct SearchResult {
    pub entry: CatalogEntry,
    pub score: f32,
}

pub fn search(query: &str, top_k: usize) -> Vec<SearchResult>
```

1. If query is empty or whitespace, return empty vec immediately.
2. Tokenize the query using the loaded tokenizer.
3. Run the ONNX model to get the query embedding (384-dim float32).
4. Normalize the query vector to unit length.
5. Compute cosine similarity against all 833 catalog vectors.
   Since both query and catalog vectors are unit-normalized, cosine similarity
   is just the dot product — a simple loop, no library needed.
6. Collect top-k results by score.
7. Filter out results below a minimum score threshold (start with 0.3,
   tune after testing).
8. Return sorted descending by score.

### Debounce

Don't search on every single keypress — debounce to 80ms after the last
keystroke. Spawn the search on a background thread. Cancel the previous
search if a new keystroke comes in before it completes.

---

## Step 4 — Launcher module (`launcher.rs`)

See `windows-settings-launcher-spec.md` for the full dispatch logic.

Key constraint: **no UI Automation, no window-walking, no accessibility APIs
at runtime**. The launcher does exactly one thing: spawn a process or invoke
a URI handler. That's it.

Quick summary of dispatch logic:
- `ms-settings:*` → `explorer.exe <uri>`
- `control.exe *` → `Command::new("control.exe").arg(rest_as_single_arg)`
- `*.msc` → `Command::new("mmc.exe").arg(path)`
- anything else → `Command::new(cmd)`

---

## Step 5 — UI (`ui.rs`)

### Window spec

- Frameless Win32 window, centered on screen
- Width: 600px, height: auto-expands for results (min 56px input-only,
  max ~400px with results)
- Always on top (`HWND_TOPMOST`)
- Rounded corners (`DwmSetWindowAttribute` with `DWMWCP_ROUND`)
- Semi-transparent background (use `SetLayeredWindowAttributes` or DWM
  composition)
- Disappears when it loses focus (`WM_KILLFOCUS` → hide window)
- Disappears when Escape is pressed

### Input field

- Single-line text input, full width
- Placeholder text: `Search settings...`
- No border, blends with window background
- Font: Segoe UI Variable, 16px

### Results list

- Shows up to 5 results
- Each result row:
  - Line 1: `control_name` — 14px, full weight
  - Line 2: `breadcrumb_path` — 12px, muted color
  - Right side: source badge (small, subtle) — e.g. `LEGACY` for
    `control.exe` launches, `SETTINGS` for `ms-settings:` launches
- Keyboard navigation: Up/Down arrows move selection, Enter launches
- Mouse: hover highlights, click launches
- Selected row has a subtle background highlight

### Global hotkey

- Default: `Win + Space` (configurable later, hardcode for v1)
- Registration: `RegisterHotKey` Win32 API
- On hotkey: show window, clear previous input, focus input field

---

## Step 6 — Performance implementation notes

**Why `include_bytes!` for catalog.bin:**
Embedding the catalog directly in the binary means one file to distribute,
no path resolution at runtime, and the OS memory-maps it efficiently.
At 833 rows × 384 dims × 4 bytes = ~1.3MB — trivial size impact.

**Why brute-force cosine similarity:**
833 dot products of 384-dim vectors = ~320,000 multiply-accumulate ops.
On a modern CPU this takes ~0.1ms. No HNSW, no ANN index, no complexity
needed at this scale. Add an index only if catalog grows past ~50,000 rows.

**Memory layout:**
Store catalog vectors as a flat `Vec<f32>` of length `833 * 384` rather
than `Vec<Vec<f32>>`. This keeps the vectors contiguous in memory and
improves cache performance during the similarity scan.

**ONNX model loading:**
Load the model once at startup, keep it in memory. `ort` sessions are
thread-safe — the same session can be used from the search background thread.

---

## Step 7 — Validation (do this before adding any more features)

Write 50 natural-language queries the way a real user would type them —
without looking at the catalog while writing them. Examples:
- "stop mouse from jumping"
- "disable startup programs"
- "change time zone"
- "turn off notifications"
- "fix blurry text"
- "allow apps through firewall"

Run each against the search function. Count how many return the correct
setting in the top 3 results.

**Target: 70% (35/50 queries) resolve correctly in top 3.**

If you don't hit 70%:
- Look at which queries failed
- If they're synonyms problems → add synonyms to those catalog entries,
  re-embed, rebuild catalog.bin
- If they're embedding quality problems → try a larger model
- Do NOT add complexity elsewhere until the hit rate is validated

---

## What is explicitly out of scope for this MVP

Do not build these. They come after the settings search is validated:

- File search (NTFS USN Journal / MFT indexing)
- App launching (Start Menu shortcuts)
- Git commit search
- Terminal history search
- OCR / screenshot indexing
- Semantic re-ranking
- Settings panel / user preferences UI
- Auto-update mechanism
- Installer / code signing

---

## Definition of done for this MVP

- [ ] Global hotkey opens the window
- [ ] Typing a query returns results within 50ms
- [ ] Selecting a result and pressing Enter opens the correct settings
      dialog/page on the right tab
- [ ] Window closes on Escape and on focus loss
- [ ] RAM usage under 150MB at idle
- [ ] 70% hit rate on the 50-query validation set
