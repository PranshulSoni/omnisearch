# Project Raycast — Ideas & Backlog

Ideas for future implementation, not yet scheduled.

---

## 🗂️ Installer — Index During Installation

**Idea**: When the user runs the installer, perform the initial file indexing in the background while showing a "Building search index..." progress step. By the time installation completes, the index is ready and the app launches with instant results — no first-launch wait.

**How it works**:
- Add a `--index-only` CLI flag to the exe that runs the indexer and exits when done
- Installer (NSIS / WiX / Inno Setup) calls `opensearch-os.exe --index-only` as a post-install step
- Show a progress screen: `"Installing files..." → "Building search index..." → "Ready to search!"`
- Only wait for **priority folders** (Desktop, Documents, Downloads) to finish before declaring done
- Full crawl of other drives continues silently in the background after first launch

**Why it's good**:
- Users perceive the wait as "installation" not "lag" — huge psychological difference
- App feels instant from the very first launch
- Mirrors what Spotlight (macOS) and Everything (Windows) do on first install
