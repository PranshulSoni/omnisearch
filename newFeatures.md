# Universal Memory & Search Platform (Updated)

## Vision

Build a product that is not a launcher clone.

Instead of competing directly with Flow Launcher, Raycast, RustCast, Keypirinha, or PowerToys Run on app launching, build a system that becomes:

- Universal Search
- Computer Memory
- Workflow Engine
- Action Platform
- Knowledge Graph for the Operating System

Core mission:


Search by what you were doing, not by what you saved

Search by visual memory: “the blue chart PDF with the red line”

Timeline rewind: jump back to the exact computer state from 2 hours ago

Clipboard lineage: track where a copied item came from, where it was pasted, and what it became

Project auto-entity: automatically merge files, repos, tabs, notes, screenshots, and commits into one living project page

Intent search: type “pay internet bill” and it finds the app, site, file, or automation needed

Search by relation: “the file I opened before the meeting” or “the note linked to this repo”

Window-state search: reopen the exact app layout, tab state, and monitor arrangement from a past session

One-key session capture: save your entire workspace as a named memory and restore it later

Semantic drag-and-drop: drag a file into the launcher to ask what it is, where it came from, or what it relates to

Auto-generated workflows from repeated behavior: the app suggests automations when it notices you doing the same thing many times

Search by gap: “what am I missing to continue this task?” and it suggests the next file, tab, note, or command

Cross-source fact linking: connect a code symbol, a screenshot, a PDF paragraph, and a Git commit into one entity

Personal search modes: “study mode”, “dev mode”, “meeting mode”, “finance mode” with different ranking and actions

Memory alerts: “you already copied this yesterday” or “this looks like the same file you used last week”

Searchable system history: search every meaningful OS action, not just files

Private local knowledge graph that grows with use and becomes better the longer it stays installed


> I forgot where something is. Find it for me.

and

> I want something done. Do it for me.

---

# Market Positioning

Do NOT market as:

- Another launcher
- Another Raycast clone
- Another Flow Launcher clone

Market as:

> Your computer remembers everything.

The launcher becomes the entry point into a much larger system.

---

# Core Differentiators

## 1. Search Inside Everything

Single search bar for:

### Files

- PDF
- DOCX
- PPTX
- XLSX
- TXT
- Markdown
- Source code

### Images

- OCR text
- Screenshots
- Photos
- Scanned documents

### Development

- Git repositories
- Commits
- Branches
- Pull requests
- Function names
- Classes
- TODO comments

### Browser

- History
- Bookmarks
- Open tabs

### Productivity

- Obsidian
- Notion
- Notes
- Documents

### Operating System

- Windows Settings
- Control Panel
- Installed Apps
- Services

---

# 2. Search Actions Instead Of Objects

Traditional launchers:

    Open Chrome
    Open VS Code

This platform:

    Turn on Bluetooth
    Set volume to 40
    Restart Explorer
    Open Startup Folder
    Convert PDF to Word
    Compress Folder
    Clear Temp Files

Search becomes:

    What do you want done?

instead of:

    What do you want opened?

---

# 3. Computer Memory

Humans rarely remember filenames.

Humans remember:

- Yesterday
- Before lunch
- After class
- During a meeting
- Related to Tradeo

The system should store activity events.

Example:

    10:31 Opened tradeo-plan.pdf
    10:34 Copied paragraph
    10:40 Opened Figma
    10:55 Opened GitHub

This creates a searchable memory timeline.

Queries:

    Show me the PDF I opened before GitHub.
    What was I working on yesterday?
    Show me the code snippet I copied last week.

---

# 4. Entity Graph

Traditional search sees:

    tradeo-plan.pdf
    D:\Projects\Tradeo
    github.com/tradeo

as separate results.

Memory system creates:

Entity: Tradeo

Connected:

- Repository
- Folder
- PDFs
- Images
- Notes
- Websites
- Commits
- Browser History

Tradeo becomes a first-class object.

---

# 5. Workflow Engine

Example:

Command:

    start tradeo

Actions:

1. Open VS Code
2. Open terminal
3. Start backend
4. Start frontend
5. Open browser

---

Advanced:

Command:

    deploy

Actions:

1. Pull latest code
2. Run tests
3. Build
4. Deploy
5. Notify Discord

---

# Universal Command Palette

Potential killer feature.

Everything exposed through commands:

    uninstall discord
    switch microphone
    restart explorer
    enable bluetooth
    disable bluetooth
    open startup folder
    clear clipboard history

Think:

Windows Terminal
+ PowerToys
+ Flow Launcher
+ Settings
+ Control Panel

through one interface.

---

# Search Architecture

Never rely on embeddings alone.

Use hybrid retrieval.

## Layer 1: Metadata Search

Stores:

- Path
- File Type
- Timestamps
- Project Association
- Access Frequency

Example:

{
  "path": "D:/Projects/tradeo",
  "type": "repo",
  "last_opened": "..."
}

---

## Layer 2: Full Text Search

Possible technologies:

- Tantivy
- SQLite FTS5
- Lucene
- Meilisearch

Handles:

- Exact terms
- Names
- Keywords

---

## Layer 3: Vector Search

Used only for:

- Semantic similarity
- Concept matching
- Reranking

Not primary retrieval.

---

# Why Embeddings Alone Are Not Enough

Embeddings struggle with:

- before
- after
- yesterday
- last week
- opened
- copied

These are structured facts.

Correct architecture:

Metadata
+ Full Text Search
+ Semantic Search
+ Ranking

---

# Event Store

Store activity timeline.

Example:

2026-06-20 10:31
Opened tradeo-plan.pdf

2026-06-20 10:34
Copied paragraph

2026-06-20 10:55
Opened github.com/tradeo

Suggested storage:

- SQLite

---

# Data Sources

Watchers:

- Filesystem
- Browser
- Clipboard
- Git
- Screenshots
- OCR
- Applications

Pipeline:

Watchers
↓
Event Store
↓
Full Text Index
↓
Vector Index
↓
Entity Graph
↓
Search UI

---

# Installation Strategy

Critical requirement:

User must get value immediately.

Never block first launch.

---

## Phase 1 (Instant)

Available immediately:

- App Search
- Start Menu Search
- Windows Settings Search
- Recent Files

Target:

< 10 seconds

---

## Phase 2 (Background)

Index:

- PDFs
- Documents
- Code
- Browser History

CPU throttled.

---

## Phase 3 (Advanced)

Generate embeddings only for:

- Frequently accessed files
- Recent files
- Important content

Do NOT embed:

- node_modules
- DLLs
- cache folders
- build outputs
- temp files

---

# Resource Usage Goals

Current prototype:

~50 MB RAM

Target:

## Idle

40 MB – 80 MB

## Searching

80 MB – 150 MB

## Heavy Indexing

150 MB – 300 MB temporary

Must remain competitive with Flow Launcher and Raycast.

---

# Memory Philosophy

Do NOT keep everything in RAM.

Bad:

Millions of files
→ Metadata
→ OCR
→ Embeddings
→ RAM

Good:

SQLite
+ Memory Mapped Files
+ Incremental Indexes
+ Small Caches

Most data should live on disk.

---

# Embedding Strategy

Do NOT generate embeddings for everything.

Options:

## Option 1

Embed:

- Recent files
- Frequently used files

---

## Option 2

Generate on demand.

User searches:

    backtesting document

Generate embedding once.

Cache forever.

---

## Option 3 (Recommended)

Maintain:

Keyword Index

for everything.

Maintain:

Embedding Index

for only important documents.

Approximately:

10k–50k important documents.

---

# What Actually Wins

Not AI.

Winning architecture:

90% Search Infrastructure

10% AI

Not:

90% AI

10% Search

---

# Example Queries

tradeo

Returns:

- Repo
- Folder
- PDFs
- Notes
- Commits
- Websites

---

Find the PDF I opened before the DBMS lecture.

Uses:

- Timeline
- Metadata
- Ranking

---

Show me the code snippet I copied last week.

Uses:

- Clipboard History
- Timeline

---

What was I working on yesterday?

Uses:

- Event Store
- Entity Graph

---

# Long-Term Vision

Create:

Search Engine
+ Launcher
+ Computer Memory
+ Workflow Platform
+ Knowledge Graph

for the entire operating system.

The moat is not launching applications.

The moat is becoming the memory layer of Windows.
